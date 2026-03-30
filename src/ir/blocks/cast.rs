use crate::instructions::{
    DataSetvariabletoFields, DataTeevariableFields, DataVariableFields, HqCastFields, IrOpcode,
};
use crate::ir::{ReturnType, Type as IrType};
use crate::prelude::*;

pub fn insert_casts(
    blocks: &mut Vec<IrOpcode>,
    ignore_variables: bool,
    recurse: bool,
) -> HQResult<()> {
    let mut type_stack: Vec<(IrType, usize)> = vec![]; // a vector of types, and where they came from
    let mut casts: Vec<(usize, IrType)> = vec![]; // a vector of cast targets, and where they're needed
    for (i, block) in blocks.iter().enumerate() {
        let mut expected_inputs = if ignore_variables
            && (matches!(
                block,
                IrOpcode::data_setvariableto(DataSetvariabletoFields {
                    local_write, ..
                }) if !*local_write.borrow())
                || matches!(block,
                    IrOpcode::data_teevariable(DataTeevariableFields { local_read_write, .. }) if !*local_read_write.borrow())
                || matches!(
                    block,
                    IrOpcode::data_addtolist(_) | IrOpcode::data_replaceitemoflist(_)
                )) {
            vec![IrType::Any]
        } else {
            block
                .acceptable_inputs()?
                .iter()
                .copied()
                .map(|ty| if ty.is_none() { IrType::Any } else { ty })
                .collect::<Vec<_>>()
        };
        if type_stack.len() < expected_inputs.len() {
            hq_bug!(
                "didn't have enough inputs on the type stack\nat block {}",
                block
            );
        }
        let actual_inputs: Vec<_> = type_stack
            .splice((type_stack.len() - expected_inputs.len()).., [])
            .collect();
        let mut dummy_actual_inputs: Vec<_> = actual_inputs.iter().map(|a| a.0).collect();
        for (j, (expected, actual)) in
            core::iter::zip(expected_inputs.clone().into_iter(), actual_inputs).enumerate()
        {
            if !expected.is_none()
                && !expected
                    .base_types()
                    .fold(IrType::none(), IrType::or)
                    .contains(actual.0)
            {
                if matches!(
                    block,
                    IrOpcode::data_setvariableto(_)
                        | IrOpcode::data_teevariable(_)
                        | IrOpcode::data_addtolist(_)
                        | IrOpcode::data_replaceitemoflist(_)
                ) {
                    hq_bug!(
                        "attempted to insert a cast before a variable/list operation - variables \
                         should encompass all possible types, rather than causing values to be \
                         coerced.
                        Tried to cast from {} (at position {}) to {} (at position {}).
                        Occurred on these opcodes: [
                        {}
                        ]",
                        actual.0,
                        actual.1,
                        expected,
                        i,
                        blocks.iter().map(|block| format!("{block}")).join(",\n"),
                    )
                }
                casts.push((actual.1, expected));
                dummy_actual_inputs[j] = expected;
                expected_inputs[j] = IrOpcode::hq_cast(HqCastFields(expected))
                    .output_type(Rc::from([if actual.0.is_none() {
                        IrType::Any
                    } else {
                        actual.0
                    }]))?
                    .singleton_or_else(|| {
                        make_hq_bug!("hq_cast returned no output type, or multiple output types")
                    })?;
            }
        }
        if ignore_variables
            && (matches!(
                block,
                IrOpcode::data_variable(DataVariableFields {
                    local_read, ..
                }) if !*local_read.borrow())
                || matches!(block,
                    IrOpcode::data_teevariable(DataTeevariableFields { local_read_write, .. }) if !*local_read_write.borrow())
                || matches!(
                    block,
                    IrOpcode::data_itemoflist(_) | IrOpcode::procedures_argument(_)
                ))
        {
            type_stack.push((IrType::Any, i));
        } else {
            match block.output_type(Rc::from(dummy_actual_inputs))? {
                ReturnType::Singleton(output) => type_stack.push((output, i)),
                ReturnType::MultiValue(outputs) => {
                    type_stack.extend(outputs.iter().copied().zip(core::iter::repeat(i)));
                }
                ReturnType::None => (),
            }
        }

        if recurse && let Some(inline_steps) = block.inline_steps(false) {
            for inline_step in inline_steps {
                insert_casts(
                    inline_step.try_borrow_mut()?.opcodes_mut(),
                    ignore_variables,
                    true,
                )?;
            }
        }
    }
    for (pos, ty) in casts.into_iter().rev() {
        blocks.insert(pos + 1, IrOpcode::hq_cast(HqCastFields(ty)));
    }
    Ok(())
}
