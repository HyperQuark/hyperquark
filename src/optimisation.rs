use crate::ir::IrProject;
use crate::prelude::*;

mod variables_graph;

pub fn ir_optimise(ir: &Rc<IrProject>) -> HQResult<()> {
    //variables::optimise_var_types(ir)?;
    variables_graph::optimise_variables(ir)?;
    Ok(())
}
