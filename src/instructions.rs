pub mod operator;

#[allow(non_camel_case_types)]
pub enum IrOpcode {
    operator_add,
}

#[macro_export]
macro_rules! instructions_test {
    ($($type_arg:ident $(,)?)+) => {
        #[cfg(test)]
        pub mod tests {
            use super::{wasm, output_type, acceptable_inputs, IrType, HQError, HQResult};

            macro_rules! ident_as_irtype {
                ( $_:ident ) => { IrType };
            }

            fn types_iter() -> impl Iterator<Item=($(ident_as_irtype!($type_arg),)+)> {
                $(let $type_arg = IrType::flags().map(|(_, ty)| *ty).collect::<Vec<_>>();)+
                itertools::iproduct!($($type_arg,)+).filter(|($($type_arg,)+)| {
                    for (i, input) in [$($type_arg,)+].into_iter().enumerate() {
                        // invalid input types should be handled by a wrapper function somewhere
                        // so we won't test those here.
                        if !acceptable_inputs()[i].contains(*input) {
                            return false;
                        }
                    }
                    true
                })
            }

            #[test]
            fn output_type_fails_when_wasm_fails() {
                // we need to collect this iterator into a Vec because it doesn't implement clone for some reason,
                // which makes itertools angry
                for ($($type_arg,)+) in types_iter() {
                    let output_type_result = output_type($($type_arg,)+);
                    let wasm_result = wasm(&$crate::wasm::StepFunc::new(), $($type_arg,)+);
                    match (output_type_result.clone(), wasm_result.clone()) {
                        (Err(..), Ok(..)) | (Ok(..), Err(..)) => panic!("output_type result doesn't match wasm result for type(s) {:?}:\noutput_type: {:?},\nwasm: {:?}", ($($type_arg,)+), output_type_result, wasm_result),
                        (Err(HQError { err_type: e1, .. }), Err(HQError { err_type: e2, .. })) => {
                            if e1 != e2 {
                                panic!("output_type result doesn't match wasm result for type(s) {:?}:\noutput_type: {:?},\nwasm: {:?}", ($($type_arg,)+), output_type_result, wasm_result);
                            }
                        }
                        _ => continue,
                    }
                }
            }

            #[test]
            fn wasm_output_type_matches_expected_output_type() -> HQResult<()> {
                use wasm_encoder::{
                    CodeSection, FunctionSection, Instruction, Module, TypeSection,
                };

                for ($($type_arg,)+) in types_iter() {
                    let output_type = match output_type($($type_arg,)+) {
                        Ok(a) => a,
                        Err(_) => {
                            println!("skipping failed output_type");
                            continue;
                        }
                    };
                    let step_func = $crate::wasm::StepFunc::new_with_param_count([$($type_arg,)+].len())?;
                    let wasm = match wasm(&step_func, $($type_arg,)+) {
                        Ok(a) => a,
                        Err(_) => {
                            println!("skipping failed wasm");
                            continue;
                        }
                    };
                    println!("not skipping for types: {:?}", ($($type_arg,)+));
                    for (i, _) in [$($type_arg,)+].iter().enumerate() {
                        step_func.add_instructions([Instruction::LocalGet(i.try_into().unwrap())])
                    }
                    step_func.add_instructions(wasm);
                    let func = step_func.finish();

                    let mut module = Module::new();

                    let mut types = TypeSection::new();
                    let params = [$($type_arg,)+].into_iter().map(|ty| $crate::wasm::ir_type_to_wasm(ty)).collect::<HQResult<Vec<_>>>()?;
                    let results = [$crate::wasm::ir_type_to_wasm(output_type)?];
                    types.ty().function(params, results);
                    module.section(&types);

                    let mut functions = FunctionSection::new();
                    let type_index = 0;
                    functions.function(type_index);
                    module.section(&functions);

                    let mut codes = CodeSection::new();
                    codes.function(&func);
                    module.section(&codes);

                    let wasm_bytes = module.finish();

                    wasmparser::validate(&wasm_bytes).map_err(|err| make_hq_bug!("invalid wasm module with types {:?}. Original error message: {}", ($($type_arg,)+), err.message()))?;
                }
                Ok(())
            }
        }
    }
}
