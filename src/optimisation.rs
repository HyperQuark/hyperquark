use crate::ir::IrProject;
use crate::prelude::*;

mod ssa;

pub fn ir_optimise(ir: &Rc<IrProject>) -> HQResult<()> {
    //variables::optimise_var_types(ir)?;
    let ssa_token = ssa::optimise_variables(ir)?;
    Ok(())
}
