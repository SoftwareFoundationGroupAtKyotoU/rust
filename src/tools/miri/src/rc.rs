#![allow(dead_code, unused_variables, unused_imports)]

use std::collections::{HashMap, HashSet};
use std::hash::Hash;

use rustc_index::IndexVec;
use rustc_middle::ty::TyKind;
use rustc_middle::ty::layout::TyAndLayout;
use rustc_target::abi::{FieldIdx, FieldsShape, Integer, Primitive, Scalar, Size, Variants};
use serde::Serialize;

use crate::rustc_middle::ty::layout::{LayoutOf, MaybeResult};
use crate::{MemoryKind, Provenance, *};

#[derive(Serialize, Debug, Clone)]
struct VisualizerMessage {
    severity: String,
    message: String,
}

#[derive(Serialize, Debug, Clone, Default)]
struct VisualizerProvenanceFrame {
    description: String,
    nodes: HashSet<u64>,
}

#[derive(Serialize, Debug, Clone)]
struct VisualizerAlloc {
    memory_kind: String,
    backtrace: Option<String>,
    bytes: Vec<u8>,
}

#[serde_with::serde_as]
#[derive(Serialize, Debug, Default)]
struct VisualizerData {
    allocs: HashMap<u64, VisualizerAlloc>,
    provenance_static_roots: HashSet<u64>,
    /// AllocID containing wildcard provenance
    provenance_wildcard: HashSet<u64>,
    /// Exposed provenances
    provenance_exposed: HashSet<u64>,
    provenance_frames: Vec<VisualizerProvenanceFrame>,
    /// Map from AllocID to [AllocID]
    #[serde_as(as = "Vec<(_, _)>")]
    provenance_graph: HashMap<u64, HashSet<u64>>,
}

static FILE_COUNTER: std::sync::atomic::AtomicI32 = std::sync::atomic::AtomicI32::new(0);

fn alloc_map_to_entry<'tcx>(
    alloc_id: &AllocId,
    (memory_kind, alloc): &(MemoryKind, Allocation<Provenance, AllocExtra<'tcx>, MiriAllocBytes>),
) -> Option<(u64, VisualizerAlloc)> {
    let alloc_id: u64 = alloc_id.0.into();
    Some((alloc_id, VisualizerAlloc {
        memory_kind: format!("{:?}", memory_kind),
        bytes: alloc
            .get_bytes_unchecked((0..alloc.len()).into())
            .iter()
            .copied()
            .collect::<Vec<u8>>(),
        backtrace: alloc.extra.backtrace.as_ref().map(|b| format!("{:#?}", b)),
    }))
}

pub fn rc_test<'tcx>(ecx: &InterpCx<'tcx, MiriMachine<'tcx>>) {
    let mut data = VisualizerData::default();

    data.allocs =
        ecx.memory.alloc_map().filter_map_collect(alloc_map_to_entry).into_iter().collect();

    // Fill provenance-graph related fields
    ecx.memory.alloc_map().iter(|it| {
        for (alloc_id, (kind, alloc)) in it {
            for prov in alloc.provenance().provenances() {
                match prov {
                    crate::Provenance::Concrete { alloc_id: prov_alloc_id, tag } => {
                        data.provenance_graph
                            .entry(alloc_id.0.get())
                            .or_default()
                            .insert(prov_alloc_id.0.get());
                    }
                    crate::Provenance::Wildcard => {
                        data.provenance_wildcard.insert(alloc_id.0.get());
                    }
                }
            }
        }
    });

    println!("static roots: {:?}", &ecx.machine.static_roots);

    data.provenance_exposed =
        ecx.machine.alloc_addresses.borrow().exposed.iter().map(|id| id.0.get()).collect();
    data.provenance_static_roots = ecx.machine.static_roots.iter().map(|a| a.0.get()).collect();

    for (_, ptr) in &ecx.machine.threads.thread_local_allocs {
        match ptr.provenance {
            crate::Provenance::Concrete { alloc_id, tag: _ } => {
                data.provenance_static_roots.insert(alloc_id.0.get());
            }
            _ => {}
        }
    }

    for current_thread_frame in ecx.active_thread_stack() {
        let mut prov_frame = VisualizerProvenanceFrame::default();
        prov_frame.description = format!("{:?}", current_thread_frame.current_source_info());

        for (_idx, local) in current_thread_frame.locals.iter_enumerated() {
            let Ok(alloc_id) = (match local.as_mplace_or_imm() {
                Some(either::Either::Left((ptr, _mp))) =>
                    Ok(ptr
                        .provenance
                        .as_ref()
                        .and_then(|p| {
                            (p as &dyn std::any::Any).downcast_ref::<crate::machine::Provenance>()
                        })
                        .and_then(|p| p.get_alloc_id())
                        .map(|id| vec![id])),
                Some(either::Either::Right(imm)) =>
                    match imm {
                        Immediate::Scalar(scalar) =>
                            match scalar {
                                interpret::Scalar::Int(_) => Ok(None),
                                interpret::Scalar::Ptr(p, _) =>
                                    Ok((&(p.provenance) as &dyn std::any::Any)
                                        .downcast_ref::<crate::machine::Provenance>()
                                        .and_then(|p| p.get_alloc_id())
                                        .map(|id| vec![id])),
                            },
                        Immediate::ScalarPair(scalar1, scalar2) => {
                            let alloc_id_1 = match scalar1 {
                                interpret::Scalar::Int(_) => None,
                                interpret::Scalar::Ptr(p, _) =>
                                    (&(p.provenance) as &dyn std::any::Any)
                                        .downcast_ref::<crate::machine::Provenance>()
                                        .and_then(|p| p.get_alloc_id()),
                            };
                            let alloc_id_2 = match scalar2 {
                                interpret::Scalar::Int(_) => None,
                                interpret::Scalar::Ptr(p, _) =>
                                    (&(p.provenance) as &dyn std::any::Any)
                                        .downcast_ref::<crate::machine::Provenance>()
                                        .and_then(|p| p.get_alloc_id()),
                            };

                            Ok(Some(
                                [alloc_id_1, alloc_id_2]
                                    .into_iter()
                                    .filter_map(|id| id)
                                    .collect::<Vec<_>>(),
                            )
                            .filter(|v| !v.is_empty()))
                        }
                        Immediate::Uninit => Ok(None),
                    },
                None => Err("cannot convert to mplace_or_imm"),
            }) else {
                info!("failed to get alloc id");
                continue;
            };
            let Some(alloc_id) = alloc_id else {
                // No allocation needed to be handled
                continue;
            };
            for alloc_id in &alloc_id {
                prov_frame.nodes.insert(alloc_id.0.get());
            }
        }
        data.provenance_frames.push(prov_frame);
    }

    let counter = FILE_COUNTER.fetch_add(1, std::sync::atomic::Ordering::SeqCst);

    use std::io::Write;
    use std::time::{SystemTime, UNIX_EPOCH};

    let now = SystemTime::now();
    let timestamp = now.duration_since(UNIX_EPOCH).expect("Time went backwards").as_millis();
    let new_file_path = format!(".local/dumps/data_{timestamp}_{counter:06}.json");
    let mut file_new = std::fs::File::create(&new_file_path).unwrap();
    let json = serde_json::to_string(&data).unwrap();
    file_new.write_all(json.as_bytes()).unwrap();
}
