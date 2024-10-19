use crate::rustc_middle::ty::layout::{LayoutOf, MaybeResult};
use crate::*;
use rustc_index::IndexVec;
use rustc_middle::ty::{layout::TyAndLayout, TyKind};
use rustc_target::abi::{FieldIdx, FieldsShape, Primitive, Scalar, Size, Variants, Integer};

#[derive(serde::Serialize, Default)]
struct VisualizerNode {
    alloc_id: Option<u64>,
    ty: String,
    offset: u64,
    info_messages: Vec<String>,
    error_messages: Vec<String>,
    children: Vec<VisualizerNode>,
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
    ecx: &InterpCx<'tcx, MiriMachine<'tcx>>,
    alloc_id: AllocId,
    offset: u64,
    ty_and_layout: &TyAndLayout<'tcx>,
    _hint_length: Option<u64>,
) -> VisualizerNode {
    // info!(
    //     "reachability(alloc_id = {alloc_id:?}, offset = {offset:?}, ty_and_layout = {ty_and_layout:?})"
    // );

    let mut node = VisualizerNode { alloc_id: Some(alloc_id.0.into()), ..Default::default() };
    let alloc = ecx.memory.alloc_map().get(alloc_id);

    // info!("reachability for {:?}", ty_and_layout.ty);
    node.ty = format!("{:?}", ty_and_layout.ty);
    node.offset = offset;

    let ty_kind = ty_and_layout.ty.kind();
    match ty_kind {
        TyKind::Char | TyKind::Bool | TyKind::Int(_) | TyKind::Uint(_) | TyKind::Never => {
            // Nothing to do, recursion ends here
        }

        TyKind::RawPtr(ptr_ty, _ptr_mut) => {
            // TODO: check whether pointers are always stored as 8 bits
            let ptr_ty_and_layout: TyAndLayout<'tcx> =
                ecx.layout_of(*ptr_ty).to_result().ok().unwrap();

            let Some((_memory_kind, alloc)) = alloc else {
                node.error_messages.push("alloc is null".to_string());
                return node;
            };

            let address =
                unsafe { *(alloc.get_bytes_unchecked_raw().add(offset as usize) as *const u64) };

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
                    let child = visualize(ecx, alloc_id, 0, &ptr_ty_and_layout, None);
                    node.children.push(child);
                }
                None => {
                    node.error_messages
                        .push(format!("cannot find offset {offset:?} in offset_to_alloc_id_map"));
                }
            }
        }

        TyKind::Adt(adt_def, adt_args) if adt_def.is_struct() => {
            let (layout_memory_index, layout_offsets) = match &ty_and_layout.layout.fields {
                FieldsShape::Arbitrary { memory_index, offsets } => (memory_index, offsets),
                _ => {
                    node.error_messages
                        .push(format!("unknown fields: {:?}", &ty_and_layout.layout.fields));
                    return node;
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

                    let subresult =
                        visualize(ecx, alloc_id, offset + field_offset, &field_ty_and_layout, None);
                    node.children.push(subresult);
                }
            }
        }

        TyKind::Adt(adt_def, _adt_args) if adt_def.is_enum() => {
            let layout = &ty_and_layout.layout;
            node.error_messages.push(format!("todo: enum ty_kind {ty_kind:?}"));
            node.error_messages.push(format!("todo: enum {adt_def:?}"));
            node.info_messages.push(format!("layout: {layout:#?}"));

            let Variants::Multiple { tag, tag_encoding, tag_field, variants } =
                &ty_and_layout.layout.variants
            else {
                node.error_messages
                    .push(format!("unexpected enum variant: {:?}", &ty_and_layout.layout.variants));
                return node;
            };

            node.info_messages.push(format!("tag: {tag:?}"));
            node.info_messages.push(format!("tag_encoding: {tag_encoding:?}"));
            node.info_messages.push(format!("tag_field: {tag_field:?}"));
            node.info_messages.push(format!("variants: {variants:?}"));

            let tag_offset = match &ty_and_layout.layout.fields {
                FieldsShape::Arbitrary { offsets, .. } => offsets[(*tag_field).into()].bytes(),
                _ => {
                    node.error_messages
                        .push(format!("unexpected fields: {:?}", &ty_and_layout.layout.fields));
                    return node;
                }
            };

            // TODO: support anything other than U128
            let (tag_type, tag_signed) = match tag {
                Scalar::Initialized { value: Primitive::Int(int, signedness), .. }
                | Scalar::Union { value: Primitive::Int(int, signedness) } => (int, signedness),
                _ => {
                    node.error_messages.push(format!("unexpected tag type: {tag:?}"));
                    return node;
                }
            };

            let Some((_memory_kind, alloc)) = alloc else {
                node.error_messages.push("alloc is null".to_string());
                return node;
            };

            let tag_raw: u128 = unsafe {
                match (tag_type, tag_signed) {
                    (Integer::I8, false) =>
                        *(alloc.get_bytes_unchecked_raw().add(tag_offset as usize) as *const u8)
                            as u128,
                        (Integer::I8, true) => 
                        *(alloc.get_bytes_unchecked_raw().add(tag_offset as usize) as *const i8)
                            as u128,
                            _ => todo!()
                }
            };

            let discriminant = match tag_encoding {};
        }

        TyKind::Closure(def_id, generic_args) => {
            node.info_messages.push(format!("def_id: {def_id:?}"));
            node.info_messages.push(format!("generic_args: {generic_args:?}"));
            node.error_messages.push(format!("todo: closure {ty_kind:?}"));

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
                    return node;
                }
            };

            let _actual_offsets = usable_offset(layout_offsets, layout_memory_index);

            if layout_offsets.len() == 1 {
                // TODO: check is pointer only

                // TODO: check whether pointers are always stored as 8 bits
                let ptr_ty_and_layout: TyAndLayout<'tcx> =
                    ecx.layout_of(*ty).to_result().ok().unwrap();

                let Some((_memory_kind, alloc)) = alloc else {
                    node.error_messages.push("alloc is null".to_string());
                    return node;
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
                        let child = visualize(ecx, alloc_id, 0, &ptr_ty_and_layout, None);
                        node.children.push(child);
                    }
                    None => {
                        node.error_messages.push(format!(
                            "cannot find offset {offset:?} in offset_to_alloc_id_map"
                        ));
                    }
                }
            } else {
                let Some((_memory_kind, alloc)) = alloc else {
                    node.error_messages.push("alloc is null".to_string());
                    return node;
                };

                let ptr_ty_and_layout: TyAndLayout<'tcx> =
                    ecx.layout_of(*ty).to_result().ok().unwrap();

                let element_kind = ptr_ty_and_layout.ty.kind();

                match element_kind {
                    TyKind::Slice(slice_element_ty) => {
                        let address = unsafe {
                            *(alloc.get_bytes_unchecked_raw().add(offset as usize) as *const u64)
                        };

                        let Some((alloc_id, offset)) =
                            find_alloc_id_and_offset_for_address(ecx, address)
                        else {
                            node.error_messages
                                .push(format!("failed to find offset for address {address:?}"));
                            return node;
                        };

                        let length = unsafe {
                            *(alloc.get_bytes_unchecked_raw().add(offset as usize + 8usize)
                                as *const u64)
                        };
                        node.info_messages.push(format!("slice_element_ty: {slice_element_ty:?}"));
                        node.info_messages.push(format!("offset: {address:?}, length: {length:?}"));

                        let child = visualize(ecx, alloc_id, 0, &ptr_ty_and_layout, Some(length));
                        node.children.push(child);
                    }
                    other_element_kind => {
                        node.error_messages.push(format!("unknown: {other_element_kind:?}"));
                    }
                }
            }
        }

        TyKind::Tuple(tys) => {
            if tys.len() == 0 {
                // No further processing
                return node;
            }
            node.error_messages.push(format!("todo: tuple {tys:?}"));
        }

        other_ty_kind => {
            node.error_messages.push(format!("unsupported: {other_ty_kind:?}"));
        }
    }

    node
}

static FILE_COUNTER: std::sync::atomic::AtomicI32 = std::sync::atomic::AtomicI32::new(0);

pub fn rc_test<'tcx>(ecx: &InterpCx<'tcx, MiriMachine<'tcx>>) {
    let mut node = VisualizerNode::default();

    for current_thread_frame in ecx.active_thread_stack() {
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

            let frame_node = visualize(ecx, alloc_id, 0, &ty_and_layout, None);
            node.children.push(frame_node);
        }
    }

    let counter = FILE_COUNTER.fetch_add(1, std::sync::atomic::Ordering::SeqCst);

    use std::io::Write;
    use std::time::{SystemTime, UNIX_EPOCH};

    let now = SystemTime::now();
    let timestamp = now.duration_since(UNIX_EPOCH).expect("Time went backwards").as_millis();
    let file_path = format!(".local/dumps/{timestamp}_{counter:06}.json");
    let mut file = std::fs::File::create(&file_path).unwrap();
    let json = serde_json::to_string(&node).unwrap();
    file.write_all(json.as_bytes()).unwrap();
}
