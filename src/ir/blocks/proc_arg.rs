use crate::instructions::{HqIntegerFields, IrOpcode, ProceduresArgumentFields};
use crate::ir::StepContext;
use crate::prelude::*;
use crate::sb3::{BlockInfo, VarVal};

#[derive(Copy, Clone, Debug, PartialEq, Eq)]
pub enum ProcArgType {
    Boolean,
    StringNumber,
}

pub fn procedure_argument(
    _arg_type: ProcArgType,
    block_info: &BlockInfo,
    context: &StepContext,
) -> HQResult<Vec<IrOpcode>> {
    let Some(proc_context) = context.proc_context.clone() else {
        // this is always the default, regardless of type
        return Ok(vec![IrOpcode::hq_integer(HqIntegerFields(0))]);
    };
    let VarVal::String(arg_name) = block_info
        .fields
        .get("VALUE")
        .ok_or_else(|| make_hq_bad_proj!("missing VALUE field for proc argument"))?
        .get_0()
        .ok_or_else(|| make_hq_bad_proj!("missing value of VALUE field"))?
    else {
        hq_bad_proj!("non-string proc argument name")
    };
    let Some(index) = proc_context
        .arg_names
        .iter()
        .position(|name| name == arg_name)
    else {
        return Ok(vec![IrOpcode::hq_integer(HqIntegerFields(0))]);
    };
    let arg_vars = (*proc_context.arg_vars).borrow();
    let arg_var = arg_vars
        .get(index)
        .ok_or_else(|| make_hq_bad_proj!("argument index not in range of argumenttypes"))?;
    Ok(vec![IrOpcode::procedures_argument(
        ProceduresArgumentFields {
            index,
            arg_var: arg_var.clone(),
            in_warped: context.warp,
            arg_vars: Rc::clone(&proc_context.arg_vars),
        },
    )])
}
