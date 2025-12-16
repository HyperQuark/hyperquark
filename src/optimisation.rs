use crate::ir::IrProject;
use crate::prelude::*;

mod ssa;

pub use ssa::SSAToken;

pub fn ir_optimise(ir: &Rc<IrProject>) -> HQResult<SSAToken> {
    //variables::optimise_var_types(ir)?;
    let ssa_token = ssa::optimise_variables(ir)?;
    Ok(ssa_token)
}
