use super::super::prelude::*;
use crate::ir::Proc;
use crate::wasm::StepFunc;

#[derive(Clone, Debug)]
pub struct Fields {
    pub proc: Rc<Proc>,
}

impl fmt::Display for Fields {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(
            f,
            r#"{{
        "proc": {:?},
    }}"#,
            self.proc.proccode()
        )
    }
}

pub fn wasm(
    func: &StepFunc,
    _inputs: Rc<[IrType]>,
    Fields { proc }: &Fields,
) -> HQResult<Vec<InternalInstruction>> {
    Ok(wasm![
        LocalGet((func.params().len() - 1).try_into().map_err(|_| make_hq_bug!("local index out of bounds"))?),
        #LazyWarpedProcCall(Rc::clone(proc))
    ])
}

pub fn acceptable_inputs(Fields { proc }: &Fields) -> HQResult<Rc<[IrType]>> {
    Ok(proc.context().arg_vars().try_borrow()?.iter().map(|var| *var.possible_types()).collect())
}

// for now, this block is a special case because it actually has multiple return values! (because of
// variable shennanigans.) This is handled in src/wasm/func.rs > StepFunc::compile_step.
// TODO: make output_type return a vec rather than an option
pub fn output_type(_inputs: Rc<[IrType]>, _fields: &Fields) -> HQResult<Option<IrType>> {
    Ok(None)
}

pub const REQUESTS_SCREEN_REFRESH: bool = false;
