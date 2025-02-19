/// generates unit tests for instructions files. Takes a module name, followed by a semicolon, followed by an optional comma-separated list of arbitrary identifiers
/// corresponding to the number of inputs the block takes, optionally followed by a semicolon and an expression
/// for a sensible default for any fields; if multiple field values need to be tested, the macro can be repeated.
#[macro_export]
macro_rules! instructions_test {
    {$module:ident; $opcode:ident; $($type_arg:ident $(,)?)* $(@$fields:expr)? $(;)?} => {
        $crate::instructions_test!{$module; $opcode; $($type_arg,)* $(@$fields)? ; Default::default()}
    };
    {$module:ident; $opcode:ident; $($type_arg:ident $(,)?)* $(@ $fields:expr)? ; $flags:expr} => {
        #[cfg(test)]
        mod $module {
            fn flags() -> $crate::wasm::WasmFlags {
                $flags
            }

            use super::{wasm, output_type, acceptable_inputs};
            use $crate::prelude::*;
            use $crate::ir::Type as IrType;
            use wasm_encoder::{
                CodeSection, ExportSection, FunctionSection, ImportSection, Instruction, Module, TableSection, TypeSection, MemorySection, MemoryType, ValType,
            };
            use $crate::wasm::{StepFunc, Registries, WasmProject};

            #[allow(unused)]
            macro_rules! ident_as_irtype {
                ( $_:ident ) => { IrType };
            }

            fn types_iter(base_only: bool) -> impl Iterator<Item=($(ident_as_irtype!($type_arg),)*)> {
                // we need to collect this iterator into a Vec because it doesn't implement clone for some reason,
                // which makes itertools angry
                $(let $type_arg = IrType::flags().map(|(_, ty)| *ty).collect::<Vec<_>>();)*
                itertools::iproduct!($($type_arg,)*).filter(move |($($type_arg,)*)| {
                    let types: &[&IrType] = &[$($type_arg,)*];
                    for (i, input) in (*types).into_iter().enumerate() {
                        // invalid input types should be handled by a wrapper function somewhere
                        // so we won't test those here.
                        // TODO: are they actually handled elsewhere?
                        if !acceptable_inputs()[i].contains(**input) {
                            return false;
                        }
                        // again, non-base types should be handled and unboxed by a wrapper function
                        // contained in src/instructions/input_switcher.rs
                        if base_only && !input.is_base_type() {
                            return false;
                        }
                    }
                    true
                })
            }

            #[test]
            fn output_type_fails_when_wasm_fails() {
                for ($($type_arg,)*) in types_iter(true) {
                    let output_type_result = output_type(Rc::new([$($type_arg,)*]), $(&$fields)?);
                    let registries = Rc::new(Registries::default());
                    let step_func = StepFunc::new(Rc::clone(&registries), flags());
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
                for ($($type_arg,)*) in types_iter(true) {
                    let output_type = match output_type(Rc::new([$($type_arg,)*]), $(&$fields)?) {
                        Ok(a) => a,
                        Err(_) => {
                            println!("skipping failed output_type");
                            continue;
                        }
                    };
                    let registries = Rc::new(Registries::default());
                    let types: &[IrType] = &[$($type_arg,)*];
                    let params = [Ok(ValType::I32)].into_iter().chain([$($type_arg,)*].into_iter().map(|ty| WasmProject::ir_type_to_wasm(ty))).collect::<HQResult<Vec<_>>>()?;
                    let result = match output_type {
                        Some(output) => Some(WasmProject::ir_type_to_wasm(output)?),
                        None => None,
                        };
                    let step_func = StepFunc::new_with_types(params.into(), result, Rc::clone(&registries), flags())?;
                    let wasm = match wasm(&step_func, Rc::new([$($type_arg,)*]), $(&$fields)?) {
                        Ok(a) => a,
                        Err(_) => {
                            println!("skipping failed wasm");
                            continue;
                        }
                    };
                    println!("{wasm:?}");
                    for (i, _) in types.iter().enumerate() {
                        step_func.add_instructions([Instruction::LocalGet((i + 1).try_into().unwrap())])?
                    }
                    step_func.add_instructions(wasm)?;

                    let mut module = Module::new();

                    let mut imports = ImportSection::new();
                    let mut types = TypeSection::new();
                    let mut tables = TableSection::new();
                    let mut functions = FunctionSection::new();
                    let mut codes = CodeSection::new();
                    let mut memories = MemorySection::new();
                    let mut exports = ExportSection::new();

                    memories.memory(MemoryType {
                        minimum: 1,
                        maximum: None,
                        memory64: false,
                        shared: false,
                        page_size_log2: None,
                    });

                    registries.external_functions().clone().finish(&mut imports, registries.types())?;
                    step_func.finish(&mut functions, &mut codes)?;
                    registries.types().clone().finish(&mut types);
                    registries.tables().clone().finish(& mut tables, &mut exports);

                    module.section(&types);
                    module.section(&imports);
                    module.section(&functions);
                    module.section(&tables);
                    module.section(&memories);
                    module.section(&codes);

                    let wasm_bytes = module.finish();

                    wasmparser::validate(&wasm_bytes).map_err(|err| make_hq_bug!("invalid wasm module with types {:?}. Original error message: {}", ($($type_arg,)*), err.message()))?;
                }
                Ok(())
            }

            #[test]
            fn wasm_output_type_matches_wrapped_expected_output_type() -> HQResult<()> {
                for ($($type_arg,)*) in types_iter(false) {
                    let output_type = match output_type(Rc::new([$($type_arg,)*]), $(&$fields)?) {
                        Ok(a) => a,
                        Err(_) => {
                            println!("skipping failed output_type");
                            continue;
                        }
                    };
                    println!("{output_type:?}");
                    let registries = Rc::new(Registries::default());
                    let types: &[IrType] = &[$($type_arg,)*];
                    let params = [Ok(ValType::I32)].into_iter().chain([$($type_arg,)*].into_iter().map(|ty| WasmProject::ir_type_to_wasm(ty))).collect::<HQResult<Vec<_>>>()?;
                    let result = match output_type {
                        Some(output) => Some(WasmProject::ir_type_to_wasm(output)?),
                        None => None,
                        };
                    println!("{result:?}");
                    let step_func = StepFunc::new_with_types(params.into(), result, Rc::clone(&registries), flags())?;
                    let wasm = match $crate::instructions::wrap_instruction(&step_func, Rc::new([$($type_arg,)*]), $crate::instructions::IrOpcode::$opcode$(($fields))?) {
                        Ok(a) => a,
                        Err(e) => {
                            println!("skipping failed wasm (message: {})", e.msg);
                            continue;
                        }
                    };
                    println!("{wasm:?}");
                    for (i, _) in types.iter().enumerate() {
                        step_func.add_instructions([Instruction::LocalGet((i + 1).try_into().unwrap())])?
                    }
                    step_func.add_instructions(wasm)?;

                    let mut module = Module::new();

                    let mut imports = ImportSection::new();
                    let mut types = TypeSection::new();
                    let mut tables = TableSection::new();
                    let mut functions = FunctionSection::new();
                    let mut codes = CodeSection::new();
                    let mut memories = MemorySection::new();
                    let mut exports = ExportSection::new();

                    memories.memory(MemoryType {
                        minimum: 1,
                        maximum: None,
                        memory64: false,
                        shared: false,
                        page_size_log2: None,
                    });

                    registries.external_functions().clone().finish(&mut imports, registries.types())?;
                    step_func.finish(&mut functions, &mut codes)?;
                    registries.types().clone().finish(&mut types);
                    registries.tables().clone().finish(& mut tables, &mut exports);

                    module.section(&types);
                    module.section(&imports);
                    module.section(&functions);
                    module.section(&tables);
                    module.section(&memories);
                    module.section(&codes);

                    let wasm_bytes = module.finish();

                    wasmparser::validate(&wasm_bytes).map_err(|err| make_hq_bug!("invalid wasm module with types {:?}. Original error message: {}", ($($type_arg,)*), err.message()))?;
                }
                Ok(())
            }

            fn wasm_to_js_type(ty: ValType) -> &'static str {
                match ty {
                    ValType::I32 => "Integer",
                    ValType::F64 => "number",
                    ValType::EXTERNREF => "string",
                    _ => todo!("unknown js type for wasm type {:?}", ty)
                }
            }

            #[test]
            fn js_functions_match_declared_types() {
                use ezno_checker::{check_project as check_js, Diagnostic, INTERNAL_DEFINITION_FILE_PATH as ts_defs};
                use std::path::{Path, PathBuf};
                use std::fs;

                for ($($type_arg,)*) in types_iter(true) {
                    let registries = Rc::new(Registries::default());
                    let step_func = StepFunc::new(Rc::clone(&registries), flags());
                    if wasm(&step_func, Rc::new([$($type_arg,)*]), $(&$fields)?).is_err() {
                        println!("skipping failed wasm");
                        continue;
                    };
                    for ((module, name), (params, results)) in registries.external_functions().registry().try_borrow().unwrap().iter() {
                        assert!(results.len() <= 1, "external function {}::{} registered as returning multiple results", module, name);
                        let out = if results.len() == 0 {
                            "void"
                        } else {
                            wasm_to_js_type(results[0])
                        };
                        let arg_idents: Vec<String> = params.iter().enumerate().map(|(i, _)| format!("_{i}")).collect();
                        let ins = arg_idents.iter().enumerate().map(|(i, ident)| {
                            format!(
                                "{}: {}",
                                ident,
                                wasm_to_js_type(*params.get(i).unwrap())
                                )
                        }).collect::<Vec<_>>().join(", ");
                        let diagnostics = check_js::<_, ezno_checker::synthesis::EznoParser>(
                            vec![PathBuf::from(format!("js/{}/{}.ts", module, name))],
                            vec![ts_defs.into()],
                            &|path: &Path| {
                                let func_string = fs::read_to_string(path).ok()?;
                                let test_string = format!("{func};
function _({ins}): {out} {{
    return {name}({ts});
}}", ins=ins, out=out, func=func_string, name=name, ts=arg_idents.join(", "));
                                println!("{}", test_string.clone());
                                Some(test_string.as_str().as_bytes().into_iter().map(|&u| u).collect::<Vec<_>>())
                            },
                            Default::default(),
                            (),
                            None,
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
