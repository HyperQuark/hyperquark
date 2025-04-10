use proc_macro::TokenStream;
use proc_macro2::TokenStream as TokenStream2;
use quote::{format_ident, quote, quote_spanned};
use std::collections::{HashMap, HashSet};
use syn::parse::{Parse, ParseStream};
use syn::{parenthesized, Error as SynError, Expr, Ident, Token};

enum Item {
    Instruction { expr: Expr },
    SpecialInstruction { expr: Expr },
    NanReduce { input_ident: Ident },
    IsNan { input_ident: Ident },
    Box { input_ident: Ident },
    Error(TokenStream2),
}

impl Parse for Item {
    fn parse(input: ParseStream) -> syn::Result<Self> {
        if input.peek(Token![@]) {
            input.parse::<Token![@]>()?;
            let ident: Ident = input.parse()?;
            match ident.to_string().as_str() {
                "nanreduce" => {
                    let content;
                    parenthesized!(content in input);
                    if let Ok(input_ident) = content.parse::<Ident>() {
                        Ok(Item::NanReduce { input_ident })
                    } else {
                        let span = content.span();
                        Ok(Item::Error(
                            quote_spanned! { span=> compile_error!("Expected an ident in @nanreduce") },
                        ))
                    }
                }
                "isnan" => {
                    let content;
                    parenthesized!(content in input);
                    if let Ok(input_ident) = content.parse::<Ident>() {
                        Ok(Item::IsNan { input_ident })
                    } else {
                        let span = content.span();
                        Ok(Item::Error(
                            quote_spanned! { span=> compile_error!("Expected an ident in @isnan") },
                        ))
                    }
                }
                "boxed" => {
                    let content;
                    parenthesized!(content in input);
                    if let Ok(input_ident) = content.parse::<Ident>() {
                        Ok(Item::Box { input_ident })
                    } else {
                        let span = content.span();
                        Ok(Item::Error(
                            quote_spanned! { span=> compile_error!("Expected an ident in @boxed") },
                        ))
                    }
                }
                _ => Err(SynError::new(ident.span(), "Unknown special instruction")),
            }
        } else if input.peek(Token![#]) {
            input.parse::<Token![#]>()?;
            let expr: Expr = input.parse()?;
            Ok(Item::SpecialInstruction { expr })
        } else {
            let expr: Expr = input.parse()?;
            Ok(Item::Instruction { expr })
        }
    }
}

struct WasmInput {
    items: Vec<Item>,
    nan_checks: HashSet<String>,
    boxed_checks: HashSet<String>,
}

impl Parse for WasmInput {
    fn parse(input: ParseStream) -> syn::Result<Self> {
        let mut items = Vec::new();
        let mut nan_checks = HashSet::new();
        let mut boxed_checks = HashSet::new();
        while !input.is_empty() {
            let item = input.parse()?;
            if let Item::NanReduce { ref input_ident } | Item::IsNan { ref input_ident } = item {
                nan_checks.insert(input_ident.clone().to_string());
            }
            if let Item::Box { ref input_ident } = item {
                boxed_checks.insert(input_ident.clone().to_string());
            }
            items.push(item);
            if input.peek(Token![,]) {
                input.parse::<Token![,]>()?;
            } else {
                break;
            }
        }
        Ok(WasmInput {
            items,
            nan_checks,
            boxed_checks,
        })
    }
}

#[proc_macro]
pub fn wasm(input: TokenStream) -> TokenStream {
    let parsed = syn::parse_macro_input!(input as WasmInput);

    let nan_checks: Vec<_> = parsed.nan_checks.into_iter().collect();
    let boxed_checks: Vec<_> = parsed.boxed_checks.into_iter().collect();

    if nan_checks.is_empty() && boxed_checks.is_empty() {
        let instructions = parsed.items.iter().filter_map(|item| {
            if let Item::Instruction { expr } = item {
                Some(quote! { ImmediateInstruction(wasm_encoder::Instruction::#expr) })
            } else if let Item::SpecialInstruction { expr } = item {
                Some(quote! { #expr })
            } else {
                None
            }
        });
        quote! { vec![#(crate::wasm::InternalInstruction::#instructions),*] }
    } else {
        let conditions = (0..(1 << (nan_checks.len() + 2 * boxed_checks.len()))).map(|mask| {
            let checks = nan_checks.iter().enumerate().map(|(i, ident)| {
                let ident = format_ident!("{ident}");
                let nan_check = quote! { #ident.contains(crate::ir::Type::FloatNan) };
                let not_nan_check = quote! { !#ident.contains(crate::ir::Type::FloatNan) };
                if (mask & (1 << i)) == 0 {
                    nan_check
                } else {
                    not_nan_check
                }
            }).chain(boxed_checks.iter().enumerate().map(|(i, ident)| (i * 2 + nan_checks.len(), ident)).map(|(i,ident)| {
                let ident = format_ident!("{ident}");
                let boxed_check = quote! { !#ident.is_base_type() };
                let string_check = quote! { #ident.base_type() == Some(crate::ir::Type::String) };
                let float_check = quote! { #ident.base_type() == Some(crate::ir::Type::Float) };
                let int_check = quote! { #ident.base_type() == Some(crate::ir::Type::QuasiInt) };
                if (mask & ((1 << i) + (1 << (i + 1)))) == 0 {
                    boxed_check
                } else if (mask & ((1 << i) + (1 << (i + 1)))) == 1 {
                    string_check
                } else if (mask & ((1 << i) + (1 << (i + 1)))) == 2 {
                    float_check
                } else {
                    int_check
                }
            }));
            let these_nan: HashSet<_> = nan_checks
                .iter()
                .enumerate()
                .filter_map(|(i, expr)| {
                    if (mask & (1 << i)) == 0 {
                        Some(expr)
                    } else {
                        None
                    }
                })
                .collect();
            let these_unboxed: HashMap<_, _> = boxed_checks
                .iter()
                .enumerate()
                .filter_map(|(i, expr)| {
                    let state = mask & ((1 << i) + (1 << (i + 1)));
                    if state == 0 {
                        None
                    } else {
                        Some((expr, state))
                    }
                })
                .collect();
            let (instructions, locals): (Vec<_>, Vec<_>) = parsed
                .items
                .iter()
                .enumerate()
                .filter_map(|(i, item)| match item {
                    Item::Instruction { expr } => {
                        Some((vec![quote! { ImmediateInstruction(wasm_encoder::Instruction::#expr) }].into_iter(), None))
                    }
                    Item::NanReduce { input_ident } => {
                        if these_nan.contains(&input_ident.clone().to_string()) {
                            let local_ident = format_ident!("__local_{}", i);
                            Some((
                                vec![
                                    quote! { ImmediateInstruction(wasm_encoder::Instruction::LocalTee(#local_ident)) },
                                    quote! { ImmediateInstruction(wasm_encoder::Instruction::LocalGet(#local_ident)) },
                                    quote! { ImmediateInstruction(wasm_encoder::Instruction::F64Eq) },
                                    quote! { ImmediateInstruction(wasm_encoder::Instruction::If(wasm_encoder::BlockType::Result(wasm_encoder::ValType::F64))) },
                                    quote! { ImmediateInstruction(wasm_encoder::Instruction::LocalGet(#local_ident)) },
                                    quote! { ImmediateInstruction(wasm_encoder::Instruction::Else) },
                                    quote! { ImmediateInstruction(wasm_encoder::Instruction::F64Const(0.0)) },
                                    quote! { ImmediateInstruction(wasm_encoder::Instruction::End) }
                                ]
                                .into_iter(),
                                Some((local_ident, format_ident!("F64")))
                            ))
                        } else {
                            None
                        }
                    }
                    Item::IsNan { input_ident } => {
                        if these_nan.contains(&input_ident.clone().to_string()) {
                            let local_ident = format_ident!("__local_{}", i);
                            Some((
                                vec![
                                    quote! { ImmediateInstruction(wasm_encoder::Instruction::LocalTee(#local_ident)) },
                                    quote! { ImmediateInstruction(wasm_encoder::Instruction::LocalGet(#local_ident)) },
                                    quote! { ImmediateInstruction(wasm_encoder::Instruction::F64Ne) },
                                ]
                                .into_iter(),
                                Some((local_ident, format_ident!("F64")))
                            ))
                        } else {
                            Some((vec![
                                quote! { ImmediateInstruction(wasm_encoder::Instruction::Drop) },
                                quote! { ImmediateInstruction(wasm_encoder::Instruction::I32Const(0)) }
                            ].into_iter(), None))
                        }
                    }
                    Item::Box { input_ident} => {
                        let local_ident = format_ident!("__local_{}", i);
                        if let Some(state) = these_unboxed.get(&input_ident.clone().to_string()) {
                            match state {
                                1 => Some((vec![
                                    quote! { ImmediateInstruction(wasm_encoder::Instruction::I32Const(1)) },
                                    quote! { ImmediateInstruction(wasm_encoder::Instruction::TableGrow(__strings_table_index)) },
                                    quote! { ImmediateInstruction(wasm_encoder::Instruction::I64ExtendI32S) },
                                    quote! { ImmediateInstruction(wasm_encoder::Instruction::I64Const(BOXED_STRING_PATTERN)) },
                                    quote! { ImmediateInstruction(wasm_encoder::Instruction::I64Or) },
                                ].into_iter(), Some((local_ident, format_ident!("EXTERNREF"))))),
                                2 => Some((vec![quote! { ImmediateInstruction(wasm_encoder::Instruction::I64ReinterpretF64) }].into_iter(), None)),
                                3 => Some((vec![
                                    quote! { ImmediateInstruction(wasm_encoder::Instruction::I64ExtendI32S) },
                                    quote! { ImmediateInstruction(wasm_encoder::Instruction::I64Const(BOXED_INT_PATTERN)) },
                                    quote! { ImmediateInstruction(wasm_encoder::Instruction::I64Or) },
                                ].into_iter(), None)),
                                _ => panic!("invalid state")
                            }
                        } else {
                            None
                        }
                    }
                    Item::SpecialInstruction { expr } => Some((vec![quote! { #expr }].into_iter(), None)),
                    Item::Error(ts) => Some((vec![ts.clone()].into_iter(), None))
                })
                .unzip();
            let instructions = instructions.into_iter().flatten();
            let (local_names, local_types): (Vec<_>, Vec<_>) = locals.into_iter().flatten().unzip();
            quote! {
                if #(#checks)&&* {
                    #(let #local_names = func.local(wasm_encoder::ValType::#local_types)?;)*
                    vec![#(crate::wasm::InternalInstruction::#instructions),*]
                }
            }
        });
        quote! {
            {
                let __strings_table_index: u32 = func
                    .registries()
                    .tables()
                    .register("strings".into(), (RefType::EXTERNREF, 0, None))?;
                #(#conditions) else * else {
                    unreachable!()
                }
            }
        }
    }
    .into()
}
