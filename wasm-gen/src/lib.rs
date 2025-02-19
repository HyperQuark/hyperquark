use proc_macro::TokenStream;
use quote::{format_ident, quote, ToTokens};
use std::collections::HashSet;
use syn::parse::{Parse, ParseStream};
use syn::{parenthesized, Error as SynError, Expr, Ident, Token};

enum Item {
    Instruction { expr: Expr },
    NanReduce { input_ident: Ident },
}

impl Parse for Item {
    fn parse(input: ParseStream) -> syn::Result<Self> {
        if input.peek(Token![@]) {
            input.parse::<Token![@]>()?;
            let ident: Ident = input.parse()?;
            if ident == "nanreduce" {
                let content;
                parenthesized!(content in input);
                let input_ident = content.parse::<Ident>()?;
                Ok(Item::NanReduce { input_ident })
            } else {
                Err(SynError::new(ident.span(), "Unknown special instruction"))
            }
        } else {
            let expr: Expr = input.parse()?;
            Ok(Item::Instruction { expr })
        }
    }
}

struct WasmInput {
    items: Vec<Item>,
    nan_checks: HashSet<String>,
}

impl Parse for WasmInput {
    fn parse(input: ParseStream) -> syn::Result<Self> {
        let mut items = Vec::new();
        let mut nan_checks = HashSet::new();
        while !input.is_empty() {
            let item = input.parse()?;
            if let Item::NanReduce {
                ref input_ident, ..
            } = item
            {
                nan_checks.insert(input_ident.clone().to_token_stream().to_string());
            }
            items.push(item);
            if input.peek(Token![,]) {
                input.parse::<Token![,]>()?;
            } else {
                break;
            }
        }
        Ok(WasmInput { items, nan_checks })
    }
}

#[proc_macro]
pub fn wasm(input: TokenStream) -> TokenStream {
    let parsed = syn::parse_macro_input!(input as WasmInput);

    let nan_checks: Vec<_> = parsed.nan_checks.into_iter().collect();

    if nan_checks.is_empty() {
        let instructions = parsed.items.iter().filter_map(|item| {
            if let Item::Instruction { expr } = item {
                Some(quote! { #expr })
            } else {
                None
            }
        });
        quote! { vec![#(wasm_encoder::Instruction::#instructions),*] }
    } else {
        let conditions = (0..(1 << nan_checks.len())).map(|mask| {
            let checks = nan_checks.iter().enumerate().map(|(i, ident)| {
                let ident = format_ident!("{ident}");
                let nan_check = quote! { #ident.contains(crate::ir::Type::FloatNan) };
                let not_nan_check = quote! { !#ident.contains(crate::ir::Type::FloatNan) };
                if (mask & (1 << i)) == 0 {
                    nan_check
                } else {
                    not_nan_check
                }
            });
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
            let (instructions, locals): (Vec<_>, Vec<_>) = parsed
                .items
                .iter()
                .enumerate()
                .filter_map(|(i, item)| match item {
                    Item::Instruction { expr } => {
                        Some((vec![quote! { #expr }].into_iter(), None))
                    }
                    Item::NanReduce { input_ident } => {
                        if these_nan.contains(&input_ident.clone().to_token_stream().to_string()) {
                            let local_ident = format_ident!("__local_{}", i);
                            Some((
                                vec![
                                    quote! { LocalTee(#local_ident) },
                                    quote! { LocalGet(#local_ident) },
                                    quote! { F64Eq },
                                    quote! { If(wasm_encoder::BlockType::Result(wasm_encoder::ValType::F64)) },
                                    quote! { LocalGet(#local_ident) },
                                    quote! { Else },
                                    quote! { F64Const(0.0) },
                                    quote! { End }
                                ]
                                .into_iter(),
                                Some(local_ident))
                            )
                        } else {
                            None
                        }
                    }
                })
                .unzip();
            let instructions = instructions.into_iter().flatten();
            let locals = locals.into_iter().flatten();
            quote! {
                if #(#checks)&&* {
                    #(let #locals = func.local(wasm_encoder::ValType::F64)?;)*
                    vec![#(wasm_encoder::Instruction::#instructions),*]
                }
            }
        });
        quote! {
            #(#conditions) else * else {
                unreachable!()
            }
        }
    }
    .into()
}
