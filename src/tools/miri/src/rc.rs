#![allow(dead_code, unused_variables, unused_imports)]

use std::collections::{HashMap, HashSet, VecDeque};
use std::hash::Hash;
use std::ops::Range;
use std::sync::Mutex;

use once_cell::sync::Lazy;
use rustc_index::IndexVec;
use rustc_middle::ty::TyKind;
use rustc_middle::ty::layout::TyAndLayout;
use rustc_span::sym::dealloc;
use rustc_target::abi::{FieldIdx, FieldsShape, Integer, Primitive, Scalar, Size, Variants};
use serde::Serialize;

use crate::rustc_middle::ty::layout::{LayoutOf, MaybeResult};
use crate::{MemoryKind, Provenance, *};

static GLOBAL_IGNORE_SET: Lazy<Mutex<HashSet<u64>>> = Lazy::new(|| Mutex::new(HashSet::new()));

fn is_memory_kind_leakable(kind: &str) -> bool {
    // TODO: confirm how to handle Machine(Runtime)
    return kind == "Machine(Machine)"
        || kind == "Machine(Global)"
        || kind == "Machine(ExternStatic)"
        || kind == "Machine(Tls)"
        || kind == "Machine(Runtime)";
}

fn report_leak<'tcx>(ecx: &InterpCx<'tcx, MiriMachine<'tcx>>, alloc_id: u64) {
    println!("Alloc {alloc_id} has leaked");
    let alloc = ecx.memory.alloc_map().get(AllocId(alloc_id.try_into().unwrap()));
    println!("  allocated");
    if let Some((memory_kind, alloc)) = alloc {
        if let Some(backtrace) = &alloc.extra.backtrace {
            for frame in backtrace {
                println!("    in {:?}", frame.span);
            }
        }
    }
    println!("  lost");
    for frame in ecx.active_thread_stack().iter().rev() {
        if let Some(source_info) = frame.current_source_info() {
            println!("    in {:?}", source_info.span);
        } else {
            println!("    in (unknown)")
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
        Some(either::Either::Left((ptr, _mp))) =>
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

pub fn rc_test<'tcx>(ecx: &InterpCx<'tcx, MiriMachine<'tcx>>) {
    let mut provenance_root_tags: Vec<u64> = vec![];
    let mut leakable_alloc_ids = HashSet::<u64>::new();

    for alloc_id in ecx.machine.static_roots.iter().cloned() {
        leakable_alloc_ids.insert(alloc_id.0.into());
        if let Some((_, alloc)) = ecx.memory.alloc_map().get(alloc_id) {
            for tag in alloc.provenance().provenances() {
                if let crate::Provenance::Concrete { tag, .. } = tag {
                    provenance_root_tags.push(tag.get());
                }
            }
        }
    }

    for (_, ptr) in &ecx.machine.threads.thread_local_allocs {
        match ptr.provenance {
            crate::Provenance::Concrete { tag, alloc_id } => {
                provenance_root_tags.push(tag.get());
            }
            _ => {}
        }
    }

    let mut local_tags = vec![];

    for current_thread_frame in ecx.active_thread_stack() {
        for (_idx, local) in current_thread_frame.locals.iter_enumerated() {
            let Some(provenances) = local_to_provenances(local) else {
                continue;
            };

            for prov in &provenances {
                if let crate::machine::Provenance::Concrete { tag, .. } = prov {
                    local_tags.push(tag.clone());
                }
            }
        }
    }

    let mut tag_to_readable_bytes: HashMap<u64, Vec<Range<u64>>> = HashMap::new();
    // Required range to read the tag
    let mut byte_to_tag: HashMap<u64, (Range<u64>, u64)> = HashMap::new();

    ecx.memory.alloc_map().iter(|it| {
        for (alloc_id, (memory_kind, alloc)) in it {
            let sb = alloc.extra.borrow_tracker_sb().borrow();
            let global_state = ecx.machine.alloc_addresses.borrow();
            let base_addr = global_state.base_addr[alloc_id];
            // note: this range is relative to alloc (e.g. 0 -> start of alloc) so we need to offset it
            for (range, stack) in sb.stacks.iter_all() {
                let range = range.start + base_addr..range.end + base_addr;
                for item in &stack.borrows {
                    if item.perm() != Permission::Disabled {
                        tag_to_readable_bytes
                            .entry(item.tag().get())
                            .or_default()
                            .push(range.clone());
                    }
                }
            }

            let pointer_size = 8; // TODO extract the information from correct source
            let provenance = alloc.provenance();
            for (size, prov) in provenance.ptrs.iter() {
                if let crate::machine::Provenance::Concrete { tag, .. } = prov {
                    let ptr_base_addr = base_addr + size.bytes();
                    byte_to_tag.insert(
                        ptr_base_addr,
                        (ptr_base_addr..ptr_base_addr + pointer_size, tag.get()),
                    );
                }
            }
            if let Some(bytes) = &provenance.bytes {
                for (size, prov) in bytes.iter() {
                    let byte_base_addr = base_addr + size.bytes();
                    if let crate::machine::Provenance::Concrete { tag, .. } = prov {
                        byte_to_tag.insert(
                            byte_base_addr,
                            ((byte_base_addr..byte_base_addr + 1), tag.get()),
                        );
                    }
                }
            }
        }
    });

    // TODO: make sure invariant that tag_to_readable_bytes is maximally compacted (i.e. all adjacent ranges are combined)
    //       should be guaranteed by stacked borrows stack

    let mut visited_tags = HashSet::<u64>::new();
    let mut visited_byte_ranges = HashSet::<Range<u64>>::new();

    let mut queue_tags = VecDeque::<u64>::new();
    let mut queue_byte_ranges = VecDeque::<Range<u64>>::new();

    for tag in provenance_root_tags.iter().copied().chain(local_tags.iter().map(|t| t.get())) {
        queue_tags.push_back(tag);
    }

    // TODO: handle cases where only part of the pointer can be read
    loop {
        while let Some(tag) = queue_tags.pop_front() {
            if visited_tags.contains(&tag) {
                continue;
            }
            visited_tags.insert(tag);
            if let Some(byte_ranges) = tag_to_readable_bytes.get(&tag) {
                for byte_range in byte_ranges {
                    queue_byte_ranges.push_back(byte_range.clone());
                }
            }
        }

        while let Some(byte_range) = queue_byte_ranges.pop_front() {
            if visited_byte_ranges.contains(&byte_range) {
                continue;
            }
            visited_byte_ranges.insert(byte_range.clone());
            for byte in byte_range.clone() {
                if let Some((required, tag)) = byte_to_tag.get(&byte) {
                    if byte_range.start <= required.start
                        && required.end <= byte_range.end
                        && !visited_tags.contains(tag)
                    {
                        queue_tags.push_back(*tag);
                    }
                }
            }
        }

        if queue_tags.is_empty() && queue_byte_ranges.is_empty() {
            break;
        }
    }

    ecx.memory.alloc_map().iter(|it| {
        for (alloc_id, (memory_kind, alloc)) in it {
            let memory_kind_str = format!("{memory_kind:?}");
            if is_memory_kind_leakable(&memory_kind_str)
                || leakable_alloc_ids.contains(&alloc_id.0.into())
            {
                continue;
            }

            if alloc.bytes.layout.size() == 0 {
                continue;
            }
            let sb = alloc.extra.borrow_tracker_sb().borrow();
            // note: this range is relative to alloc (e.g. 0 -> start of alloc) so we need to offset it
            let mut intersected: Option<HashSet<u64>> = None;
            for (range, stack) in sb.stacks.iter_all() {
                let mut deallocatable_set = HashSet::<u64>::new();
                for borrow in &stack.borrows {
                    if matches!(borrow.perm(), Permission::SharedReadWrite | Permission::Unique) {
                        deallocatable_set.insert(borrow.tag().get());
                    }
                }
                if let Some(intersected) = &mut intersected {
                    *intersected = intersected.intersection(&deallocatable_set).copied().collect();
                } else {
                    intersected = Some(deallocatable_set);
                }
            }
            let Some(intersected) = intersected else {
                panic!("Unexpected empty intersection result for non-zero sized alloc");
            };
            if intersected.iter().all(|deallocatable_tag| !visited_tags.contains(deallocatable_tag))
            {
                // println!("Alloc {alloc_id:?} has surely leaked");
                let mut ignore_set = GLOBAL_IGNORE_SET.lock().unwrap();
                if !ignore_set.contains(&(alloc_id.0.into())) {
                    report_leak(ecx, alloc_id.0.into());
                    ignore_set.insert(alloc_id.0.into());
                }
            }
        }
    });
}
