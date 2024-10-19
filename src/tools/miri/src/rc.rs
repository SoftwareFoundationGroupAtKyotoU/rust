#![allow(dead_code, unused_variables)]

use std::{
    collections::{HashMap, HashSet},
    hash::Hash,
};

use crate::rustc_middle::ty::layout::{LayoutOf, MaybeResult};
use crate::*;
use rustc_index::IndexVec;
use rustc_middle::ty::{layout::TyAndLayout, TyKind};
use rustc_target::abi::{FieldIdx, FieldsShape, Integer, Primitive, Scalar, Size, Variants};
use serde::Serialize;

#[derive(serde::Serialize, Default)]
struct VisualizerNode {
    alloc_id: Option<u64>,
    ty: String,
    offset: u64,
    info_messages: Vec<String>,
    error_messages: Vec<String>,
    children: Vec<VisualizerNode>,
}

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
    nodes: Vec<VisualizerNodeKey>,
}

#[serde_with::serde_as]
#[derive(Serialize, Debug, Default)]
struct VisualizerData {
    #[serde_as(as = "Vec<(_, _)>")]
    nodes: HashMap<VisualizerNodeKey, VisualizerNodeValue>,
    edges: HashSet<(VisualizerNodeKey, VisualizerNodeKey)>,
    frames: Vec<VisualizerFrame>,
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
    address: u64,
) -> Option<(AllocId, u64)> {
    let global_state = &*(ecx.machine.alloc_addresses.borrow());

    // This is not the full inverse of base_addr; **dead allocations** have been removed.
    // TODO: find reverse dead allocations
    // base_addr seems to be have the full information
    let offset_to_alloc_id_map = &global_state.int_to_ptr_map;

    // TODO: improve algorithm here (use binary search)
    let mut ptr_alloc_id: Option<AllocId> = None;

    // TODO: support non-zero offset
    for &(offset, alloc_id) in offset_to_alloc_id_map {
        if offset == address {
            ptr_alloc_id = Some(alloc_id);
        }
    }

    match ptr_alloc_id {
        Some(alloc_id) => Some((alloc_id, 0)),
        None => None,
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
) -> VisualizerNode {
    let self_key = VisualizerNodeKey {
        alloc_id: alloc_id.0.into(),
        offset,
        ty: format!("{:?}", ty_and_layout.ty),
    };
    let mut node = VisualizerNode { alloc_id: Some(alloc_id.0.into()), ..Default::default() };
    // Multiple parents may point to the same node, so we need to add the edge before returning
    if let Some(parent) = parent {
        data.add_edge(parent, self_key.clone());
    }
    if data.has_node(&self_key) {
        return node;
    }

    let mut node_context = VisualizerNodeValue::default();
    // Mark as visited
    data.add_node(self_key.clone(), node_context.clone());
    let alloc = ecx.memory.alloc_map().get(alloc_id);

    if let Some((_memory_kind, alloc)) = alloc {
        node_context.alloc_bytes =
            alloc.get_bytes_unchecked((0..alloc.len()).into()).iter().copied().collect();
    }

    // info!("reachability for {:?}", ty_and_layout.ty);
    node.ty = format!("{:?}", ty_and_layout.ty);
    node.offset = offset;

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

                let global_state = &*(ecx.machine.alloc_addresses.borrow());
                // This is not the full inverse of base_addr; **dead allocations** have been removed.
                // TODO: find reverse dead allocations
                // base_addr seems to be have the full information
                let offset_to_alloc_id_map = &global_state.int_to_ptr_map;

                info!("offset_to_alloc_id_map: {offset_to_alloc_id_map:?}");
                info!("address: {address:?}");

                // TODO: improve algorithm here (use binary search)
                let mut ptr_alloc_id: Option<AllocId> = None;

                for &(offset, alloc_id) in offset_to_alloc_id_map {
                    if offset == address {
                        ptr_alloc_id = Some(alloc_id);
                    }
                }

                match ptr_alloc_id {
                    Some(alloc_id) => {
                        let child = visualize(
                            data,
                            Some(self_key.clone()),
                            ecx,
                            alloc_id,
                            0,
                            &ptr_ty_and_layout,
                            None,
                        );
                        node.children.push(child);
                    }
                    None => {
                        node_context.log_error(format!(
                            "cannot find offset {offset:?} in offset_to_alloc_id_map"
                        ));
                    }
                }
            }

            TyKind::Adt(adt_def, adt_args) if adt_def.is_struct() => {
                let (layout_memory_index, layout_offsets) = match &ty_and_layout.layout.fields {
                    FieldsShape::Arbitrary { memory_index, offsets } => (memory_index, offsets),
                    _ => {
                        node.error_messages
                            .push(format!("unknown fields: {:?}", &ty_and_layout.layout.fields));
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

                        let subresult = visualize(
                            data,
                            Some(self_key.clone()),
                            ecx,
                            alloc_id,
                            offset + field_offset,
                            &field_ty_and_layout,
                            None,
                        );
                        node.children.push(subresult);
                    }
                }
            }

            TyKind::Adt(adt_def, _adt_args) if adt_def.is_enum() => {
                let layout = &ty_and_layout.layout;
                node_context.log_error(format!("todo: enum ty_kind {ty_kind:?}"));
                node_context.log_error(format!("todo: enum {adt_def:?}"));
                node_context.log_info(format!("layout: {layout:#?}"));

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
                        node.error_messages
                            .push(format!("unexpected fields: {:?}", &ty_and_layout.layout.fields));
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
                        node.error_messages
                            .push(format!("unknown fields: {:?}", &ty_and_layout.layout.fields));
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

                    let global_state = &*(ecx.machine.alloc_addresses.borrow());
                    // This is not the full inverse of base_addr; **dead allocations** have been removed.
                    // TODO: find reverse dead allocations
                    // base_addr seems to be have the full information
                    let offset_to_alloc_id_map = &global_state.int_to_ptr_map;

                    info!("offset_to_alloc_id_map: {offset_to_alloc_id_map:?}");
                    info!("address: {address:?}");

                    // TODO: improve algorithm here (use binary search)
                    let mut ptr_alloc_id: Option<AllocId> = None;

                    for &(offset, alloc_id) in offset_to_alloc_id_map {
                        if offset == address {
                            ptr_alloc_id = Some(alloc_id);
                        }
                    }

                    match ptr_alloc_id {
                        Some(alloc_id) => {
                            let child = visualize(
                                data,
                                Some(self_key.clone()),
                                ecx,
                                alloc_id,
                                0,
                                &ptr_ty_and_layout,
                                None,
                            );
                            node.children.push(child);
                        }
                        None => {
                            node_context.log_error(format!(
                                "cannot find offset {offset:?} in offset_to_alloc_id_map"
                            ));
                        }
                    }
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
                                node.error_messages
                                    .push(format!("failed to find offset for address {address:?}"));
                                break 'ty_kind_match;
                            };

                            let length = unsafe {
                                *(alloc.get_bytes_unchecked_raw().add(offset as usize + 8usize)
                                    as *const u64)
                            };
                            node.info_messages
                                .push(format!("slice_element_ty: {slice_element_ty:?}"));
                            node.info_messages
                                .push(format!("offset: {address:?}, length: {length:?}"));

                            let child = visualize(
                                data,
                                Some(self_key.clone()),
                                ecx,
                                alloc_id,
                                0,
                                &ptr_ty_and_layout,
                                Some(length),
                            );
                            node.children.push(child);
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

    node
}

static FILE_COUNTER: std::sync::atomic::AtomicI32 = std::sync::atomic::AtomicI32::new(0);

pub fn rc_test<'tcx>(ecx: &InterpCx<'tcx, MiriMachine<'tcx>>) {
    let mut node = VisualizerNode::default();
    let mut data = VisualizerData::default();

    for current_thread_frame in ecx.active_thread_stack() {
        let mut frame = VisualizerFrame::default();
        info!("before_stack_pop has frame {:?}", current_thread_frame.current_source_info());
        for (_idx, local) in current_thread_frame.locals.iter_enumerated() {
            let Some(alloc_id) = (match local.as_mplace_or_imm() {
                Some(either::Either::Left((ptr, _mp))) =>
                    ptr.provenance
                        .as_ref()
                        .and_then(|p| {
                            (p as &dyn std::any::Any).downcast_ref::<crate::machine::Provenance>()
                        })
                        .and_then(|p| p.get_alloc_id()),
                Some(either::Either::Right(_imm)) => {
                    None // TODO
                }
                None => None,
            }) else {
                info!("failed to get alloc id");
                continue;
            };
            let Some(ty_and_layout) = local.layout.get() else {
                info!("failed to get TyAndLayout: is None");
                continue;
            };

            let frame_node = visualize(&mut data, None, ecx, alloc_id, 0, &ty_and_layout, None);
            let node_key = VisualizerNodeKey {
                alloc_id: alloc_id.0.into(),
                offset: 0,
                ty: format!("{:?}", &ty_and_layout.ty),
            };
            frame.nodes.push(node_key);
            node.children.push(frame_node);
        }
        data.add_frame(frame);
    }

    let counter = FILE_COUNTER.fetch_add(1, std::sync::atomic::Ordering::SeqCst);

    use std::io::Write;
    use std::time::{SystemTime, UNIX_EPOCH};

    let now = SystemTime::now();
    let timestamp = now.duration_since(UNIX_EPOCH).expect("Time went backwards").as_millis();
    let old_file_path = format!(".local/dumps/old_{timestamp}_{counter:06}.json");
    let new_file_path = format!(".local/dumps/data_{timestamp}_{counter:06}.json");
    let mut file = std::fs::File::create(&old_file_path).unwrap();
    let json = serde_json::to_string(&node).unwrap();
    file.write_all(json.as_bytes()).unwrap();
    let mut file_new = std::fs::File::create(&new_file_path).unwrap();
    let json = serde_json::to_string(&data).unwrap();
    file_new.write_all(json.as_bytes()).unwrap();
}
