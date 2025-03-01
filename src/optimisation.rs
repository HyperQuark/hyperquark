use crate::ir::IrProject;
use crate::prelude::*;

mod var_types;

pub fn ir_optimise(ir: Rc<IrProject>) -> HQResult<()> {
    var_types::optimise_var_types(Rc::clone(&ir))?;
    Ok(())
}
