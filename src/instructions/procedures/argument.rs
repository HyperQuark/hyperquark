use wasm_encoder::{AbstractHeapType, HeapType};

use super::super::prelude::*;
use crate::{
    ir::RcVar,
    wasm::{StepFunc, WasmProject, registries::types::WasmType},
};

#[derive(Clone, Debug)]
pub struct Fields {
    pub index: usize,
    pub arg_var: RcVar,
    pub in_warped: bool,
    pub arg_vars: Rc<RefCell<Vec<RcVar>>>,
}

impl fmt::Display for Fields {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(
            f,
            r#"{{
        "arg_index": {},
        "arg_var": {},
        "in_warped": {}
    }}"#,
            self.index, self.arg_var, self.in_warped
        )
    }
}

pub fn wasm(
    func: &StepFunc,
    _inputs: Rc<[IrType]>,
    Fields {
        index,
        arg_var,
        in_warped,
        arg_vars,
    }: &Fields,
) -> HQResult<Vec<InternalInstruction>> {
    #[expect(clippy::redundant_else, reason = "false positive")]
    if *in_warped {
        hq_assert!(
            WasmProject::ir_type_to_wasm(*arg_var.possible_types())?
                == *func.params().get(*index).ok_or_else(|| make_hq_bug!(
                    "proc argument index was out of bounds for func params"
                ))?,
            "proc argument type didn't match that of the corresponding function param"
        );
        Ok(wasm![LocalGet((*index).try_into().map_err(
            |_| make_hq_bug!("argument index out of bounds")
        )?)])
    } else {
        hq_assert!(
            matches!(
                *func.params().get(1).ok_or_else(|| {
                    make_hq_bug!("missing 2nd parameter for nonwarped procedure function")
                })?,
                ValType::Ref(RefType {
                    nullable: true,
                    heap_type: HeapType::Abstract {
                        shared: false,
                        ty: AbstractHeapType::Struct
                    }
                })
            ),
            "struct parameter did not match the expected type shape; expected (structref), got {:?}",
            func.params().get(1)
        );
        let struct_type_index = func
            .registries()
            .types()
            .proc_arg_struct_type(&(**arg_vars).borrow())?;
        let registries = func.registries();
        let type_registry = registries.types().registry().borrow();
        let WasmType::Struct(struct_type_fields) = type_registry
            .get_index(struct_type_index as usize)
            .ok_or_else(|| make_hq_bug!("type index not found in type registry"))?
            .0
        else {
            hq_bug!("struct type was not a struct type")
        };
        hq_assert!(
            WasmProject::ir_type_to_wasm(*arg_var.possible_types())?
                == struct_type_fields
                    .get(*index)
                    .ok_or_else(|| make_hq_bug!("proc arg index out of bounds in struct fields"))?
                    .element_type
                    .unpack(),
            "proc argument type didn't match that of the corresponding function param"
        );
        Ok(wasm![
            LocalGet(2), // this should be initialised as a nonnull struct ref
            StructGet {
                struct_type_index,
                field_index: (*index)
                    .try_into()
                    .map_err(|_| make_hq_bug!("argument index out of bounds"))?
            }
        ])
    }
}

pub fn acceptable_inputs(_: &Fields) -> HQResult<Rc<[IrType]>> {
    Ok(Rc::from([]))
}

pub fn output_type(_inputs: Rc<[IrType]>, Fields { arg_var, .. }: &Fields) -> HQResult<ReturnType> {
    Ok(Singleton(*arg_var.possible_types()))
}

pub const REQUESTS_SCREEN_REFRESH: bool = false;
