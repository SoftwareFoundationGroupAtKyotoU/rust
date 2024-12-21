use std::collections::{HashMap, HashSet, VecDeque};
use std::sync::Mutex;

use once_cell::sync::Lazy;
use serde::Serialize;

use crate::*;

#[derive(Serialize, Debug, Clone)]
struct AllocInfo {
    memory_kind: String,
}

static GLOBAL_IGNORE_SET: Lazy<Mutex<HashSet<u64>>> = Lazy::new(|| Mutex::new(HashSet::new()));

fn is_memory_kind_leakable(kind: &str) -> bool {
    return kind == "Machine(Machine)"
        || kind == "Machine(Global)"
        || kind == "Machine(ExternStatic)"
        || kind == "Machine(Tls)"
        || kind == "Machine(Runtime)";
}

fn report_leak<'tcx>(ecx: &InterpCx<'tcx, MiriMachine<'tcx>>, alloc_id: u64) {
    println!("Alloc {alloc_id} has leaked");
    for frame in ecx.active_thread_stack().iter().rev() {
        if let Some(source_info) = frame.current_source_info() {
            println!("  in {:?}", source_info.span);
        } else {
            println!("  in (unknown)")
        }
    }
    println!("");
}

fn provenance_to_concrete(
    p: impl rustc_middle::mir::interpret::Provenance,
) -> Option<crate::machine::Provenance> {
    (&p as &dyn std::any::Any).downcast_ref::<crate::machine::Provenance>().cloned()
}

fn provenance_scalar_to_concrete(
    s: rustc_middle::mir::interpret::Scalar<impl rustc_middle::mir::interpret::Provenance>,
) -> Option<crate::machine::Provenance> {
    match s {
        interpret::Scalar::Int(_) => None,
        interpret::Scalar::Ptr(p, _) => provenance_to_concrete(p.provenance),
    }
}

fn local_to_provenances<'tcx>(
    local: &LocalState<'tcx, impl rustc_middle::mir::interpret::Provenance>,
) -> Option<Vec<crate::machine::Provenance>> {
    match local.as_mplace_or_imm() {
        Some(either::Either::Left((ptr, _))) =>
            ptr.provenance.and_then(provenance_to_concrete).map(|p| vec![p]),
        Some(either::Either::Right(imm)) =>
            match imm {
                Immediate::Scalar(scalar) =>
                    match scalar {
                        interpret::Scalar::Int(_) => None,
                        interpret::Scalar::Ptr(p, _) =>
                            provenance_to_concrete(p.provenance).map(|p| vec![p]),
                    },
                Immediate::ScalarPair(scalar1, scalar2) => {
                    let prov_1 = provenance_scalar_to_concrete(scalar1);
                    let prov_2 = provenance_scalar_to_concrete(scalar2);

                    Some([prov_1, prov_2].into_iter().filter_map(|p| p).collect::<Vec<_>>())
                        .filter(|v| !v.is_empty())
                }
                Immediate::Uninit => None,
            },
        None => None,
    }
}

pub fn memory_leak_check_simplified<'tcx>(ecx: &InterpCx<'tcx, MiriMachine<'tcx>>) {
    let allocs: HashMap<u64, AllocInfo> = ecx
        .memory
        .alloc_map()
        .filter_map_collect(|alloc_id, (memory_kind, _)| {
            Some((alloc_id.0.get(), AllocInfo { memory_kind: format!("{:?}", memory_kind) }))
        })
        .into_iter()
        .collect();

    let mut provenance_graph: HashMap<u64, HashSet<u64>> = HashMap::new();
    ecx.memory.alloc_map().iter(|it| {
        for (alloc_id, (_, alloc)) in it {
            for prov in alloc.provenance().provenances() {
                if let crate::Provenance::Concrete { alloc_id: prov_alloc_id, .. } = prov {
                    provenance_graph
                        .entry(alloc_id.0.get())
                        .or_default()
                        .insert(prov_alloc_id.0.get());
                }
            }
        }
    });

    let provenance_exposed: HashSet<u64> =
        ecx.machine.alloc_addresses.borrow().exposed.iter().map(|id| id.0.get()).collect();

    let mut provenance_static_roots: HashSet<u64> =
        ecx.machine.static_roots.iter().map(|a| a.0.get()).collect();
    for (_, ptr) in &ecx.machine.threads.thread_local_allocs {
        if let crate::Provenance::Concrete { alloc_id, .. } = ptr.provenance {
            provenance_static_roots.insert(alloc_id.0.get());
        }
    }

    let mut reachable_set: HashSet<u64> = provenance_static_roots.iter().copied().collect();
    for current_thread_frame in ecx.active_thread_stack() {
        for local in current_thread_frame.locals.iter() {
            let Some(provenances) = local_to_provenances(local) else {
                continue;
            };

            for prov in &provenances {
                if let crate::Provenance::Concrete { alloc_id, .. } = prov {
                    reachable_set.insert(alloc_id.0.get());
                }
            }
        }
    }

    {
        let mut queue: VecDeque<u64> = reachable_set.iter().copied().collect();
        while let Some(parent) = queue.pop_front() {
            if let Some(children) = provenance_graph.get(&parent) {
                for child in children {
                    if !reachable_set.contains(child) {
                        reachable_set.insert(*child);
                        queue.push_back(*child);
                    }
                }
            }
        }
    }

    {
        let mut ignore_set = GLOBAL_IGNORE_SET.lock().unwrap();
        for (alloc_id, alloc) in &allocs {
            if !is_memory_kind_leakable(&alloc.memory_kind)
                && !reachable_set.contains(alloc_id)
                && !ignore_set.contains(alloc_id)
                && !provenance_exposed.contains(alloc_id)
            {
                report_leak(ecx, *alloc_id);
                ignore_set.insert(*alloc_id);
            }
        }
    }
}
