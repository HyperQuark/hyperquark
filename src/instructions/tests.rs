/// Generates unit tests for instructions files.
///
/// For a block `peppa_pig` which takes 0 inputs,
/// ```ignore
/// instructions_test!(mod test for peppa_pig {});
/// ```
///
/// Example:
/// For a block `foo_bar`, which takes 2 inputs, with Fields(bool),
/// ```ignore
/// instructions_test!(mod test_true for foo_bar(t1, t2) {
///     fields = super::Fields(true);
/// });
/// instructions_test!(mod test_false for foo_bar(t1, t2) {
///     fields = super::Fields(false);
/// });
/// ```
#[macro_export]
macro_rules! instructions_test {
    {
        mod $module:ident for $opcode:ident$(($($type_arg:ident $(,)?)*))? {
            $(fields = $fields:expr;)?
            $(flags = $flags:expr;)?
            $(setup = $setup:expr;)?
        }
    } => {
        #[cfg(test)]
        mod $module {
            #![expect(clippy::allow_attributes, reason = "might not always trigger")]

            use super::{wasm, output_type, acceptable_inputs};
            use $crate::prelude::*;
            use $crate::ir::{IrType, ReturnType};
            use wasm_encoder::ValType;
            use $crate::wasm::{StepFunc, Registries, WasmProject, WasmFlags, StepTarget, ExternalEnvironment};
            #[allow(unused, reason = "it might not be unused")]
            use $crate::wasm::flags::{Switch, unit_test_wasm_features};

            fn flags() -> $crate::wasm::WasmFlags {
                #[allow(unused_mut, reason = "may not be unused")]
                #[allow(unused_assignments, reason = "may not be unused")]
                let mut flags = WasmFlags::new(unit_test_wasm_features());
                $(flags = $flags;)?
                flags
            }

            #[allow(unused_macros, reason = "it is not unused")]
            macro_rules! ident_as_irtype {
                ( $_:ident ) => { IrType };
            }

            fn types_iter(base_only: bool) -> impl Iterator<Item=($($(ident_as_irtype!($type_arg),)*)?)> {
                // we need to collect this iterator into a Vec because it doesn't implement clone for some reason,
                // which makes itertools angry
                $($(let $type_arg = IrType::flags().map(|(_, ty)| *ty).collect::<Vec<_>>();)*)?
                itertools::iproduct!($($($type_arg,)*)?).filter(move |($($($type_arg,)*)?)| {
                    let types: &[&IrType] = &[$($($type_arg,)*)?];
                    for (i, input) in (*types).into_iter().enumerate() {
                        // non-base types should be handled and unboxed by a wrapper function
                        // contained in src/instructions/input_switcher.rs
                        if base_only && !input.is_base_type() {
                            return false;
                        }
                        // invalid base input types should be handled by insert_casts in
                        // src.ir/blocks.rs, so we won't test those here
                        if !acceptable_inputs($(&$fields)?).expect("acceptable_inputs shouldn't panic")[i].contains(**input) {
                            return false;
                        }
                    }
                    true
                })
            }

            #[test]
            fn output_type_fails_when_wasm_fails() {
                for ($($($type_arg,)*)?) in types_iter(true) {
                    #[allow(unused_mut, reason = "may not be unused")]
                    #[allow(unused, reason = "might not be unused")]
                    let mut proj = WasmProject::new(flags(), ExternalEnvironment::WebBrowser);
                    $($setup(&mut proj, flags());)?
                    let output_type_result = output_type(Rc::from([$($($type_arg,)*)?]), $(&$fields)?);
                    let registries = Rc::new(Registries::default());
                    let step_func = StepFunc::new(Rc::clone(&registries), flags(), StepTarget::Sprite(0), 0, Rc::new(vec![]));
                    let wasm_result = wasm(&step_func, Rc::from([$($($type_arg,)*)?]), $(&$fields)?);
                    match (output_type_result.clone(), wasm_result.clone()) {
                        (Err(..), Ok(..)) | (Ok(..), Err(..)) => panic!("output_type result doesn't match wasm result for type(s) {:?}:\noutput_type: {:?},\nwasm: {:?}", ($($($type_arg,)*)?), output_type_result, wasm_result),
                        (Err(HQError { err_type: e1, .. }), Err(HQError { err_type: e2, .. })) if e1 != e2 => {
                            panic!("output_type result doesn't match wasm result for type(s) {:?}:\noutput_type: {:?},\nwasm: {:?}", ($($($type_arg,)*)?), output_type_result, wasm_result);
                        }
                        _ => (),
                    }
                }
            }

            #[test]
            fn wasm_output_type_matches_expected_output_type() -> HQResult<()> {
                for ($($($type_arg,)*)?) in types_iter(true) {
                    #[allow(unused_mut, reason = "may not be unused")]
                    let mut proj = WasmProject::new(flags(), ExternalEnvironment::WebBrowser);
                    $($setup(&mut proj, flags());)?
                    let Ok(output_type) = output_type(Rc::from([$($($type_arg,)*)?]), $(&$fields)?) else {
                        println!("skipping failed output_type");
                        continue;
                    };
                    let registries = proj.registries();
                    let types: &[IrType] = &[$($($type_arg,)*)?];
                    let params = [ValType::I32, $crate::wasm::registries::TypeRegistry::STRUCT_REF].into_iter().chain([$($($type_arg,)*)?].into_iter().map(WasmProject::ir_type_to_wasm)).collect::<Vec<_>>();
                    let result = match output_type {
                        ReturnType::Singleton(output) => vec![WasmProject::ir_type_to_wasm(output)],
                        ReturnType::MultiValue(outputs) => outputs.iter().copied().map(WasmProject::ir_type_to_wasm).collect(),
                        ReturnType::None => vec![],
                    };
                    let step_func = StepFunc::new_with_types(params.into(), result.into(), Rc::clone(&registries), flags(), StepTarget::Sprite(0), 0, Rc::new(vec![]));
                    let Ok(wasm) = wasm(&step_func, Rc::from([$($($type_arg,)*)?]), $(&$fields)?) else {
                        println!("skipping failed wasm");
                        continue;
                    };
                    for (i, _) in types.iter().enumerate() {
                        step_func.add_instructions([$crate::wasm::InternalInstruction::Immediate(wasm_encoder::Instruction::LocalGet((i + 2).try_into().unwrap()))])?
                    }
                    step_func.add_instructions(wasm)?;

                    proj.steps()
                        .borrow_mut()
                        .push(step_func);

                    let wasm_bytes = proj.finish().unwrap().wasm_bytes;

                    println!("{}", wasmprinter::print_bytes(wasm_bytes.clone()).unwrap());

                    wasmparser::validate(&wasm_bytes).map_err(|err| make_hq_bug!("invalid wasm module with types {:?}. Original error message: {}", ($($($type_arg,)*)?), err.message()))?;
                }
                Ok(())
            }

            #[test]
            fn wasm_output_type_matches_wrapped_expected_output_type() -> HQResult<()> {
                for ($($($type_arg,)*)?) in types_iter(false) {
                    #[allow(unused_mut, reason = "may not be unused")]
                    let mut proj = WasmProject::new(flags(), ExternalEnvironment::WebBrowser);
                    $($setup(&mut proj, flags());)?
                    let Ok(output_type) = $crate::instructions::boxed_output_type(
                        |inputs| output_type(inputs, $(&$fields)?),
                        Rc::from([$($($type_arg,)*)?]),
                    ) else {
                        println!("skipping failed output_type");
                        continue;
                    };
                    println!("{output_type:?}");
                    let registries = proj.registries();
                    let types: &[IrType] = &[$($($type_arg,)*)?];
                    let params = [ValType::I32, $crate::wasm::registries::TypeRegistry::STRUCT_REF]
                        .into_iter()
                        .chain(
                            [$($($type_arg,)*)?]
                                .into_iter()
                                .map(WasmProject::ir_type_to_wasm)
                        )
                        .collect::<Vec<_>>();
                    let result = vec![];
                    let step_func = StepFunc::new_with_types(
                        params.into(),
                        result.into(),
                        Rc::clone(&registries),
                        flags(),
                        StepTarget::Sprite(0),
                        0,
                        Rc::new(vec![])
                    );
                    let drops = match output_type {
                        ReturnType::Singleton(_) => 1,
                        ReturnType::MultiValue(outs) => outs.len(),
                        ReturnType::None => 0,
                    };
                    let test_opcodes = [$crate::instructions::IrOpcode::$opcode$(($fields))?]
                        .into_iter()
                        .chain(core::iter::repeat_n($crate::instructions::IrOpcode::hq_drop, drops))
                        .collect::<Vec<_>>();
                    let wasm = $crate::instructions::wrap_instructions(
                        &step_func,
                        Rc::from([$($($type_arg,)*)?]),
                        &test_opcodes,
                    )?;
                    // println!("{wasm:?}");
                    for (i, _) in types.iter().enumerate() {
                        step_func.add_instructions(
                            [$crate::wasm::InternalInstruction::Immediate(
                                wasm_encoder::Instruction::LocalGet(
                                    (i + 2).try_into().unwrap()
                                )
                            )]
                        )?
                    }
                    step_func.add_instructions(wasm)?;

                    // println!("{:?}", step_func.instructions().borrow());

                    proj.steps()
                        .borrow_mut()
                        .push(step_func);

                    let wasm_bytes = proj.finish().unwrap().wasm_bytes;

                    println!("{}", wasmprinter::print_bytes(wasm_bytes.clone()).unwrap());

                    wasmparser::validate(&wasm_bytes).map_err(|err|
                        make_hq_bug!(
                            "invalid wasm module with types {:?}. Original error message: {}",
                            ($($($type_arg,)*)?),
                            err.message()
                        )
                    )?;
                }
                Ok(())
            }

            // fn wasm_to_js_type(ty: ValType) -> &'static str {
            //     match ty {
            //         ValType::I32 => "Integer",
            //         ValType::F64 => "number",
            //         ValType::EXTERNREF | ValType::Ref(RefType {
            //             nullable: false,
            //             heap_type: HeapType::EXTERN,
            //         }) => "string",
            //         _ => todo!("unknown js type for wasm type {:?}", ty)
            //     }
            // }

            // #[test]
            // fn js_functions_match_declared_types() {
            //     #![allow(clippy::tuple_array_conversions, reason = "false positive")]
            //     use ezno_checker::{check_project as check_js, Diagnostic, INTERNAL_DEFINITION_FILE_PATH as ts_defs};
            //     use std::path::{Path, PathBuf};
            //     use std::fs;

            //     for ($($type_arg,)*) in types_iter(true) {
            //         let registries = Rc::new(Registries::default());
            //         let step_func = StepFunc::new(Rc::clone(&registries), flags());
            //         if wasm(&step_func, Rc::from([$($type_arg,)*]), $(&$fields)?).is_err() {
            //             println!("skipping failed wasm");
            //             continue;
            //         };
            //         for ((module, name), (params, results)) in registries.external_functions().registry().try_borrow().unwrap().iter() {
            //             assert!(results.len() <= 1, "external function {}::{} registered as returning multiple results", module, name);
            //             let out = if results.len() == 0 {
            //                 "void"
            //             } else {
            //                 wasm_to_js_type(results[0])
            //             };
            //             let arg_idents: Vec<String> = params.iter().enumerate().map(|(i, _)| format!("_{i}")).collect();
            //             let ins = arg_idents.iter().enumerate().map(|(i, ident)| {
            //                 format!(
            //                     "{}: {}",
            //                     ident,
            //                     wasm_to_js_type(*params.get(i).unwrap())
            //                     )
            //             }).collect::<Vec<_>>().join(", ");
            //             let module_path = if *module == "wasm:js-string" {
            //                 "wasm-js-string"
            //             } else {
            //                 module
            //             };
            //             let path_buf = PathBuf::from(format!("js/{}/{}.ts", module_path, name));
            //             let diagnostics = check_js::<_, ezno_checker::synthesis::EznoParser>(
            //                 vec![path_buf.clone()],
            //                 vec![ts_defs.into()],
            //                 &|path: &Path| {
            //                     let func_string = fs::read_to_string(path).ok()?;
            //                     let test_string = if path == path_buf.as_path() {
            //                         format!("function _({ins}): {out} {{ return {name}({ts}); }};",
            //                             ins=ins,
            //                             out=out,
            //                             name=name,
            //                             ts=arg_idents.join(", ")
            //                         )
            //                     } else { String::from("") };
            //                     let total_string = format!("{func_string};\n{test_string}");
            //                     println!("{}", total_string.clone());
            //                     Some(test_string
            //                         .as_str()
            //                         .as_bytes()
            //                         .into_iter()
            //                         .map(|&u| u)
            //                         .collect::<Vec<_>>()
            //                     )
            //                 },
            //                 Default::default(),
            //                 (),
            //                 None,
            //             )
            //             .diagnostics;
            //             if diagnostics.contains_error() {
            //                 let reasons = diagnostics.into_iter().map(|d| {
            //                     match d {
            //                         Diagnostic::Global { reason, .. } => reason,
            //                         Diagnostic::Position { reason, .. } => reason,
            //                         Diagnostic::PositionWithAdditionalLabels { reason, .. } => reason,
            //                     }
            //                 }).collect::<Vec<_>>().join("; ");
            //                 panic!("js for external function {}::{} is not type-safe; reason(s): {}", module, name, reasons);
            //             }
            //         }
            //     }
            // }
        }
    }
}
