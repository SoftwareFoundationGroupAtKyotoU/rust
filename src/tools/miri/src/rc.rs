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

#[derive(Serialize, Debug, Clone, PartialEq, Eq, Hash)]
struct VisualizerNodeKey {
    alloc_id: u64,
    offset: u64,
    ty: String,
}

#[derive(Serialize, Debug, Clone)]
struct VisualizerMessage {
    severity: String,
    message: String,
}

#[derive(Serialize, Debug, Clone, Default)]
struct VisualizerNodeValue {
    alloc_bytes: Vec<u8>,
    messages: Vec<VisualizerMessage>,
}

impl VisualizerNodeValue {
    fn log_info(&mut self, message: String) {
        self.messages.push(VisualizerMessage { severity: "INFO".to_string(), message });
    }

    fn log_error(&mut self, message: String) {
        self.messages.push(VisualizerMessage { severity: "ERROR".to_string(), message });
    }
}

#[derive(Serialize, Debug, Clone, Default)]
struct VisualizerFrame {
    description: String,
    nodes: Vec<VisualizerNodeKey>,
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
    #[serde_as(as = "Vec<(_, _)>")]
    nodes: HashMap<VisualizerNodeKey, VisualizerNodeValue>,
    edges: HashSet<(VisualizerNodeKey, VisualizerNodeKey)>,
    frames: Vec<VisualizerFrame>,
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

#[derive(Serialize, Debug, Clone, Default)]
struct VisualizerProvenanceFrame {
    description: String,
    nodes: HashSet<u64>,
}

impl VisualizerData {
    fn add_node(&mut self, key: VisualizerNodeKey, value: VisualizerNodeValue) {
        self.nodes.insert(key, value);
    }

    fn has_node(&mut self, key: &VisualizerNodeKey) -> bool {
        self.nodes.contains_key(key)
    }

    fn add_edge(&mut self, from: VisualizerNodeKey, to: VisualizerNodeKey) {
        self.edges.insert((from, to));
    }

    fn add_frame(&mut self, frame: VisualizerFrame) {
        self.frames.push(frame);
    }
}

// Not sure what to name this
// Todo: check whether this is actually useful
fn usable_offset(
    offsets: &IndexVec<FieldIdx, Size>,
    memory_index: &IndexVec<FieldIdx, u32>,
) -> IndexVec<FieldIdx, Size> {
    let mut result: IndexVec<FieldIdx, Size> = IndexVec::new();
    result.resize(offsets.len(), Size::from_bytes(0));

    for i in 0..offsets.len() {
        result[memory_index[i.into()].into()] = offsets[i.into()];
    }

    result
}

fn find_alloc_id_and_offset_for_address<'tcx>(
    ecx: &InterpCx<'tcx, MiriMachine<'tcx>>,
    addr: u64,
) -> Option<(AllocId, u64)> {
    let global_state = ecx.machine.alloc_addresses.borrow();
    let pos = global_state.int_to_ptr_map.binary_search_by_key(&addr, |(addr, _)| *addr);

    let alloc_id = match pos {
        Ok(pos) => Some(global_state.int_to_ptr_map[pos].1),
        Err(0) => None,
        Err(pos) => {
            // This is the largest of the addresses smaller than `int`,
            // i.e. the greatest lower bound (glb)
            let (glb, alloc_id) = global_state.int_to_ptr_map[pos - 1];
            // This never overflows because `addr >= glb`
            let offset = addr - glb;
            // We require this to be strict in-bounds of the allocation. This arm is only
            // entered for addresses that are not the base address, so even zero-sized
            // allocations will get recognized at their base address -- but all other
            // allocations will *not* be recognized at their "end" address.
            let size = ecx.get_alloc_info(alloc_id).0;
            if offset < size.bytes() { Some(alloc_id) } else { None }
        }
    }?;

    // // We only use this provenance if it has been exposed.
    // if global_state.exposed.contains(&alloc_id) {
    //     // This must still be live, since we remove allocations from `int_to_ptr_map` when they get freed.
    //     debug_assert!(ecx.is_alloc_live(alloc_id));
    //     Some(alloc_id)
    // } else {
    //     None
    // }

    let alloc_addr_base = global_state.base_addr[&alloc_id];

    if addr >= alloc_addr_base {
        Some((alloc_id, addr - alloc_addr_base))
    } else {
        // TODO: check error
        None
    }
}

fn visualize<'tcx>(
    data: &mut VisualizerData,
    parent: Option<VisualizerNodeKey>,
    ecx: &InterpCx<'tcx, MiriMachine<'tcx>>,
    alloc_id: AllocId,
    offset: u64,
    ty_and_layout: &TyAndLayout<'tcx>,
    _hint_length: Option<u64>,
) {
    let self_key = VisualizerNodeKey {
        alloc_id: alloc_id.0.into(),
        offset,
        ty: format!("{:?}", ty_and_layout.ty),
    };
    // Multiple parents may point to the same node, so we need to add the edge before returning
    if let Some(parent) = parent {
        data.add_edge(parent, self_key.clone());
    }
    if data.has_node(&self_key) {
        return;
    }

    let mut node_context = VisualizerNodeValue::default();
    // Mark as visited
    data.add_node(self_key.clone(), node_context.clone());
    let alloc = ecx.memory.alloc_map().get(alloc_id);

    if let Some((_memory_kind, alloc)) = alloc {
        node_context.alloc_bytes =
            alloc.get_bytes_unchecked((0..alloc.len()).into()).iter().copied().collect();
    }

    let ty_kind = ty_and_layout.ty.kind();
    'ty_kind_match: {
        match ty_kind {
            TyKind::Char | TyKind::Bool | TyKind::Int(_) | TyKind::Uint(_) | TyKind::Never => {
                // Nothing to do, recursion ends here
            }

            TyKind::RawPtr(ptr_ty, _ptr_mut) => {
                // TODO: check whether pointers are always stored as 8 bits
                let ptr_ty_and_layout: TyAndLayout<'tcx> =
                    ecx.layout_of(*ptr_ty).to_result().ok().unwrap();

                let Some((_memory_kind, alloc)) = alloc else {
                    node_context.log_error("alloc is null".to_string());
                    break 'ty_kind_match;
                };

                let address = unsafe {
                    *(alloc.get_bytes_unchecked_raw().add(offset as usize) as *const u64)
                };

                let Some((alloc_id, offset)) = find_alloc_id_and_offset_for_address(ecx, address)
                else {
                    node_context.log_error(format!(
                        "cannot convert address {address:?} to alloc id and offset"
                    ));
                    break 'ty_kind_match;
                };

                visualize(
                    data,
                    Some(self_key.clone()),
                    ecx,
                    alloc_id,
                    offset,
                    &ptr_ty_and_layout,
                    None,
                );
            }

            TyKind::Adt(adt_def, adt_args) if adt_def.is_struct() => {
                let (layout_memory_index, layout_offsets) = match &ty_and_layout.layout.fields {
                    FieldsShape::Arbitrary { memory_index, offsets } => (memory_index, offsets),
                    _ => {
                        node_context.log_error(format!(
                            "unknown fields: {:?}",
                            &ty_and_layout.layout.fields
                        ));
                        break 'ty_kind_match;
                    }
                };

                let actual_offsets = usable_offset(layout_offsets, layout_memory_index);

                let fields = adt_def.variants().get(0u32.into()).map(|v| &v.fields);
                if let Some(fields) = fields {
                    for i in 0u32..(fields.len() as u32) {
                        let field = &fields[i.into()];

                        let field_ty = field.ty(*ecx.tcx, adt_args);
                        let field_ty_and_layout: TyAndLayout<'tcx> =
                            ecx.layout_of(field_ty).to_result().ok().unwrap();

                        let field_offset = actual_offsets[i.into()].bytes();

                        visualize(
                            data,
                            Some(self_key.clone()),
                            ecx,
                            alloc_id,
                            offset + field_offset,
                            &field_ty_and_layout,
                            None,
                        );
                    }
                }
            }

            TyKind::Adt(adt_def, _adt_args) if adt_def.is_enum() => {
                let layout = &ty_and_layout.layout;
                node_context.log_error(format!("todo: enum ty_kind {ty_kind:?}"));
                node_context.log_error(format!("todo: enum {adt_def:?}"));
                node_context.log_info(format!("layout: {layout:#?}"));

                // e.g. Result<isize, !>
                if let Variants::Single { index } = &ty_and_layout.layout.variants {
                    // TODO: implement single-variant enum
                    break 'ty_kind_match;
                }

                let Variants::Multiple { tag, tag_encoding, tag_field, variants } =
                    &ty_and_layout.layout.variants
                else {
                    node_context.log_error(format!(
                        "unexpected enum variant: {:?}",
                        &ty_and_layout.layout.variants
                    ));
                    break 'ty_kind_match;
                };

                node_context.log_info(format!("tag: {tag:?}"));
                node_context.log_info(format!("tag_encoding: {tag_encoding:?}"));
                node_context.log_info(format!("tag_field: {tag_field:?}"));
                node_context.log_info(format!("variants: {variants:?}"));

                let tag_offset = match &ty_and_layout.layout.fields {
                    FieldsShape::Arbitrary { offsets, .. } => offsets[(*tag_field).into()].bytes(),
                    _ => {
                        node_context.log_error(format!(
                            "unexpected fields: {:?}",
                            &ty_and_layout.layout.fields
                        ));
                        break 'ty_kind_match;
                    }
                };

                // TODO: support anything other than U128
                let (tag_type, tag_signed) = match tag {
                    Scalar::Initialized { value: Primitive::Int(int, signedness), .. }
                    | Scalar::Union { value: Primitive::Int(int, signedness) } => (int, signedness),
                    _ => {
                        node_context.log_error(format!("unexpected tag type: {tag:?}"));
                        break 'ty_kind_match;
                    }
                };

                let Some((_memory_kind, alloc)) = alloc else {
                    node_context.log_error("alloc is null".to_string());
                    break 'ty_kind_match;
                };

                let tag_raw: u128 = unsafe {
                    match (tag_type, tag_signed) {
                        (Integer::I8, false) =>
                            *(alloc.get_bytes_unchecked_raw().add(tag_offset as usize) as *const u8)
                                as u128,
                        (Integer::I8, true) =>
                            *(alloc.get_bytes_unchecked_raw().add(tag_offset as usize) as *const i8)
                                as u128,
                        _ => {
                            node_context.log_error(format!(
                                "not yet implemented for {tag_type:?} (signed = {tag_signed:?})"
                            ));
                            break 'ty_kind_match;
                        }
                    }
                };

                // let discriminant = match tag_encoding {};
            }

            TyKind::Closure(def_id, generic_args) => {
                node_context.log_info(format!("def_id: {def_id:?}"));
                node_context.log_info(format!("generic_args: {generic_args:?}"));
                node_context.log_error(format!("todo: closure {ty_kind:?}"));

                // Closure might have some captured variables. Need to figure out how to read it.
            }

            TyKind::Ref(_region, ty, _mutability) => {
                // it SHOULD be either:
                // - sized type: Scalar(ptr as *T)
                // - slices: ScalarPair(ptr as *T, len as i64)
                // should handle both? even though in either case, we just read the pointer as offset 0

                // in the second case, we need to loop through each element of the slice

                let (layout_memory_index, layout_offsets) = match &ty_and_layout.layout.fields {
                    FieldsShape::Arbitrary { memory_index, offsets } => (memory_index, offsets),
                    _ => {
                        node_context.log_error(format!(
                            "unknown fields: {:?}",
                            &ty_and_layout.layout.fields
                        ));
                        break 'ty_kind_match;
                    }
                };

                let _actual_offsets = usable_offset(layout_offsets, layout_memory_index);

                if layout_offsets.len() == 1 {
                    // TODO: check is pointer only

                    // TODO: check whether pointers are always stored as 8 bits
                    let ptr_ty_and_layout: TyAndLayout<'tcx> =
                        ecx.layout_of(*ty).to_result().ok().unwrap();

                    let Some((_memory_kind, alloc)) = alloc else {
                        node_context.log_error("alloc is null".to_string());
                        break 'ty_kind_match;
                    };

                    let address = unsafe {
                        *(alloc.get_bytes_unchecked_raw().add(offset as usize) as *const u64)
                    };

                    let Some((alloc_id, offset)) =
                        find_alloc_id_and_offset_for_address(ecx, address)
                    else {
                        node_context.log_error(format!(
                            "cannot convert address {address:?} to alloc id and offset"
                        ));
                        break 'ty_kind_match;
                    };

                    visualize(
                        data,
                        Some(self_key.clone()),
                        ecx,
                        alloc_id,
                        offset,
                        &ptr_ty_and_layout,
                        None,
                    );
                } else {
                    let Some((_memory_kind, alloc)) = alloc else {
                        node_context.log_error("alloc is null".to_string());
                        break 'ty_kind_match;
                    };

                    let ptr_ty_and_layout: TyAndLayout<'tcx> =
                        ecx.layout_of(*ty).to_result().ok().unwrap();

                    let element_kind = ptr_ty_and_layout.ty.kind();

                    match element_kind {
                        TyKind::Slice(slice_element_ty) => {
                            let address = unsafe {
                                *(alloc.get_bytes_unchecked_raw().add(offset as usize)
                                    as *const u64)
                            };

                            let Some((alloc_id, offset)) =
                                find_alloc_id_and_offset_for_address(ecx, address)
                            else {
                                node_context.log_error(format!(
                                    "failed to find offset for address {address:?}"
                                ));
                                break 'ty_kind_match;
                            };

                            let length = unsafe {
                                *(alloc.get_bytes_unchecked_raw().add(offset as usize + 8usize)
                                    as *const u64)
                            };
                            node_context
                                .log_info(format!("slice_element_ty: {slice_element_ty:?}"));
                            node_context
                                .log_info(format!("offset: {address:?}, length: {length:?}"));

                            visualize(
                                data,
                                Some(self_key.clone()),
                                ecx,
                                alloc_id,
                                0,
                                &ptr_ty_and_layout,
                                Some(length),
                            );
                        }
                        other_element_kind => {
                            node_context.log_error(format!("unknown: {other_element_kind:?}"));
                        }
                    }
                }
            }

            TyKind::Tuple(tys) => {
                if tys.len() == 0 {
                    // No further processing
                    break 'ty_kind_match;
                }
                node_context.log_error(format!("todo: tuple {tys:?}"));
            }

            other_ty_kind => {
                node_context.log_error(format!("unsupported: {other_ty_kind:?}"));
            }
        }
    }

    data.add_node(self_key, node_context);
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
        let mut frame = VisualizerFrame::default();
        let mut prov_frame = VisualizerProvenanceFrame::default();
        frame.description = format!("{:?}", current_thread_frame.current_source_info());
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
                let Some(ty_and_layout) = local.layout.get() else {
                    info!("failed to get TyAndLayout: is None");
                    continue;
                };
                let node_key = VisualizerNodeKey {
                    alloc_id: alloc_id.0.into(),
                    offset: 0, // problem: this offset may not be 0 for scalar (non-mplace)? may need to compute offset?
                    ty: format!("{:?}", &ty_and_layout.ty),
                };
                frame.nodes.push(node_key);
                prov_frame.nodes.insert(alloc_id.0.get());

                visualize(&mut data, None, ecx, *alloc_id, 0, &ty_and_layout, None);
            }
        }
        data.add_frame(frame);
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
