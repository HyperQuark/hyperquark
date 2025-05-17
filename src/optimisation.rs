use crate::ir::IrProject;
use crate::prelude::*;

mod variables;

pub fn ir_optimise(ir: &Rc<IrProject>) -> HQResult<()> {
    variables::optimise_var_types(ir)?;
    Ok(())
}
