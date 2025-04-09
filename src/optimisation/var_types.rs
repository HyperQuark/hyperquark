use crate::instructions::{DataSetvariabletoFields, IrOpcode};
use crate::ir::{IrProject, Type as IrType};
use crate::prelude::*;

pub fn optimise_var_types(project: Rc<IrProject>) -> HQResult<()> {
    crate::log("optimise vars");
    for step in project.steps().borrow().iter() {
        let mut type_stack: Vec<IrType> = vec![]; // a vector of types, and where they came from
        for block in &*step.opcodes().try_borrow()?.clone() {
            let expected_inputs = block.acceptable_inputs();
            if type_stack.len() < expected_inputs.len() {
                hq_bug!("didn't have enough inputs on the type stack")
            }
            let inputs = type_stack
                .splice((type_stack.len() - expected_inputs.len()).., [])
                .collect::<Vec<_>>();
            if let IrOpcode::data_setvariableto(DataSetvariabletoFields(var)) = block {
                var.0.add_type(inputs[0]);
            }
            if let Some(output) = block.output_type(expected_inputs)? {
                type_stack.push(output);
            }
        }
    }
    Ok(())
}
