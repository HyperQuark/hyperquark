mod file_opcode {
    //! instruction module paths look something like
    //! hyperquark::instructions::category::block
    //! so if we split it by ':', we end up with 7 chunks

    use crate::prelude::*;
    use split_exact::SplitExact;

    pub fn file_block_category(path: &'static str) -> &'static str {
        path.split_exact::<7>(|c| c == ':')[4].unwrap()
    }

    pub fn file_block_name(path: &'static str) -> &'static str {
        path.split_exact::<7>(|c| c == ':')[6].unwrap()
    }

    pub fn file_opcode(path: &'static str) -> String {
        format!("{}_{}", file_block_category(path), file_block_name(path))
    }
}
pub use file_opcode::*;

#[cfg(test)]
pub mod tests {
    #[test]
    fn file_block_category() {
        assert_eq!(super::file_block_category(module_path!()), "utilities");
    }

    #[test]
    fn file_block_name() {
        assert_eq!(super::file_block_name(module_path!()), "tests");
    }

    #[test]
    fn file_opcode() {
        assert_eq!(super::file_opcode(module_path!()), "utilities_tests");
    }
}

/// generates unit tests for instructions files. Takes a module name, followed by a semicolon, followed by an optional comma-separated list of arbitrary identifiers
/// corresponding to the number of inputs the block takes, optionally followed by a semicolon and an expression
/// for a sensible default for any fields; if multiple field values need to be tested, the macro can be repeated.
#[macro_export]
macro_rules! instructions_test {
    {$module:ident; $($type_arg:ident $(,)?)* $(; $fields:expr)?} => {
    #[cfg(test)]
    mod $module {
        use super::{wasm, output_type, acceptable_inputs};
        use $crate::prelude::*;
        use $crate::ir::Type as IrType;
        use wasm_encoder::ValType;

        #[allow(unused)]
        macro_rules! ident_as_irtype {
            ( $_:ident ) => { IrType };
        }

        fn types_iter() -> impl Iterator<Item=($(ident_as_irtype!($type_arg),)*)> {
            // we need to collect this iterator into a Vec because it doesn't implement clone for some reason,
            // which makes itertools angry
            $(let $type_arg = IrType::flags().map(|(_, ty)| *ty).collect::<Vec<_>>();)*
            itertools::iproduct!($($type_arg,)*).filter(|($($type_arg,)*)| {
                let types: &[&IrType] = &[$($type_arg,)*];
                for (i, input) in (*types).into_iter().enumerate() {
                    // invalid input types should be handled by a wrapper function somewhere
                    // so we won't test those here.
                    if !acceptable_inputs()[i].contains(**input) {
                        return false;
                    }
                }
                true
            })
        }

            #[test]
            fn output_type_fails_when_wasm_fails() {
                use $crate::wasm::{StepFunc, TypeRegistry, ExternalFunctionMap};
                for ($($type_arg,)*) in types_iter() {
                    let output_type_result = output_type(Rc::new([$($type_arg,)*]), $(&$fields)?);
                    let type_registry = Rc::new(TypeRegistry::new());
                    let external_functions = Rc::new(ExternalFunctionMap::new());
                    let types: &[IrType] = &[$($type_arg,)*];
                    let step_func = StepFunc::new_with_param_count(types.len(), type_registry.clone(), external_functions.clone()).unwrap();
                    let wasm_result = wasm(&step_func, Rc::new([$($type_arg,)*]), $(&$fields)?);
                    match (output_type_result.clone(), wasm_result.clone()) {
                        (Err(..), Ok(..)) | (Ok(..), Err(..)) => panic!("output_type result doesn't match wasm result for type(s) {:?}:\noutput_type: {:?},\nwasm: {:?}", ($($type_arg,)*), output_type_result, wasm_result),
                        (Err(HQError { err_type: e1, .. }), Err(HQError { err_type: e2, .. })) => {
                            if e1 != e2 {
                                panic!("output_type result doesn't match wasm result for type(s) {:?}:\noutput_type: {:?},\nwasm: {:?}", ($($type_arg,)*), output_type_result, wasm_result);
                            }
                        }
                        _ => continue,
                    }
                }
            }

            #[test]
            fn wasm_output_type_matches_expected_output_type() -> HQResult<()> {
                use wasm_encoder::{
                    CodeSection, FunctionSection, ImportSection, Instruction, Module, TypeSection,
                };
                use $crate::wasm::{StepFunc, TypeRegistry, ExternalFunctionMap};
                use $crate::prelude::Rc;

                for ($($type_arg,)*) in types_iter() {
                    let output_type = match output_type(Rc::new([$($type_arg,)*]), $(&$fields)?) {
                        Ok(a) => a,
                        Err(_) => {
                            println!("skipping failed output_type");
                            continue;
                        }
                    };
                    let type_registry = Rc::new(TypeRegistry::new());
                    let external_functions = Rc::new(ExternalFunctionMap::new());
                    let types: &[IrType] = &[$($type_arg,)*];
                    let step_func = StepFunc::new_with_param_count(types.len(), type_registry.clone(), external_functions.clone())?;
                    let wasm = match wasm(&step_func, Rc::new([$($type_arg,)*]), $(&$fields)?) {
                        Ok(a) => a,
                        Err(_) => {
                            println!("skipping failed wasm");
                            continue;
                        }
                    };
                    for (i, _) in types.iter().enumerate() {
                        step_func.add_instructions([Instruction::LocalGet(i.try_into().unwrap())])
                    }
                    step_func.add_instructions(wasm);
                    let func = step_func.finish();

                    let wasm_proj = $crate::wasm::WasmProject::new(Default::default(), $crate::wasm::ExternalEnvironment::WebBrowser);

                    let mut module = Module::new();

                    let mut imports = ImportSection::new();

                    Rc::unwrap_or_clone(external_functions).finish(&mut imports, type_registry.clone())?;

                    let mut types = TypeSection::new();
                    let params = [$($type_arg,)*].into_iter().map(|ty| wasm_proj.ir_type_to_wasm(ty)).collect::<HQResult<Vec<_>>>()?;
                    let results = match output_type {
                      Some(output) => vec![wasm_proj.ir_type_to_wasm(output)?],
                      None => vec![],
                    };
                    let step_type_index = type_registry.type_index(params, results)?;
                    Rc::unwrap_or_clone(type_registry).finish(&mut types);
                    module.section(&types);

                    module.section(&imports);

                    let mut functions = FunctionSection::new();
                    functions.function(step_type_index);
                    module.section(&functions);

                    let mut codes = CodeSection::new();
                    codes.function(&func);
                    module.section(&codes);

                    let wasm_bytes = module.finish();

                    wasmparser::validate(&wasm_bytes).map_err(|err| make_hq_bug!("invalid wasm module with types {:?}. Original error message: {}", ($($type_arg,)*), err.message()))?;
                }
                Ok(())
            }

            fn wasm_to_js_type(ty: ValType) -> &'static str {
                match ty {
                    ValType::I64 => "Integer",
                    ValType::F64 => "number",
                    ValType::EXTERNREF => "string",
                    _ => todo!("unknown js type for wasm type {:?}", ty)
                }
            }

            #[test]
            fn js_functions_match_declared_types() {
                use $crate::wasm::{ExternalFunctionMap, StepFunc, TypeRegistry};
                use ezno_lib::{check as check_js, Diagnostic};
                use std::path::{Path, PathBuf};
                use std::fs;

                for ($($type_arg,)*) in types_iter() {
                    let type_registry = Rc::new(TypeRegistry::new());
                    let external_functions = Rc::new(ExternalFunctionMap::new());
                    let step_func = StepFunc::new(type_registry.clone(), external_functions.clone());
                    if wasm(&step_func, Rc::new([$($type_arg,)*]), $(&$fields)?).is_err() {
                        println!("skipping failed wasm");
                        continue;
                    };
                    for ((module, name), (params, results)) in external_functions.get_map().borrow().iter() {
                        assert!(results.len() < 1, "external function {}::{} registered as returning multiple results", module, name);
                        let out = if results.len() == 0 {
                          "void"
                        } else {
                            wasm_to_js_type(results[1])
                        };
                        let arg_idents: Vec<String> = params.iter().enumerate().map(|(i, _)| format!("_{i}")).collect();
                        let ins = arg_idents.iter().enumerate().map(|(i, ident)| {
                            format!(
                                "{}: {}",
                                ident,
                                wasm_to_js_type(*params.get(i).unwrap())
                                )
                        }).collect::<Vec<_>>().join(", ");
                        let diagnostics = check_js(
                            vec![PathBuf::from(format!("src/instructions/{}/{}.ts", module, name))],
                            &|path: &Path| {
                                let func_string = fs::read_to_string(path).ok()?;
                                let test_string = format!("function _({ins}): {out} {{
                                  {func}
                                  return {name}({ts});
                                }}", ins=ins, out=out, func=func_string, name=name, ts=arg_idents.join(", "));
                                println!("{}", test_string.clone());
                                Some(test_string.as_str().as_bytes().into_iter().map(|&u| u).collect::<Vec<_>>())
                            },
                            None,
                            Default::default(),
                        )
                        .diagnostics;
                        if diagnostics.contains_error() {
                            let reasons = diagnostics.into_iter().map(|d| {
                                match d {
                                    Diagnostic::Global { reason, .. } => reason,
                                    Diagnostic::Position { reason, .. } => reason,
                                    Diagnostic::PositionWithAdditionalLabels { reason, .. } => reason,
                                }
                            }).collect::<Vec<_>>().join("; ");
                            panic!("js for external function {}::{} is not type-safe; reason(s): {}", module, name, reasons);
                        }
                    }
                }
            }
        }
    }
}
