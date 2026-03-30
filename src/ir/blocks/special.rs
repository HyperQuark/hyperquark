//! Handles parsing of "special" blocks, i.e. those that are represented as arrays rather
//! than objects.

use lazy_regex::{Lazy, lazy_regex};
use regex::Regex;

use crate::instructions::{
    DataListcontentsFields, DataVariableFields, HqColorRgbFields, HqFloatFields, HqIntegerFields,
    HqTextFields, IrOpcode,
};
use crate::ir::StepContext;
use crate::prelude::*;
use crate::sb3::BlockArray;
use crate::wasm::WasmFlags;
use crate::wasm::flags::Switch;

static SHORTHAND_HEX_COLOUR_REGEX: Lazy<Regex> = lazy_regex!(r#"^#?([a-f\d])([a-f\d])([a-f\d])$"#i);
static HEX_COLOUR_REGEX: Lazy<Regex> = lazy_regex!(r#"^#?([a-f\d]{2})([a-f\d]{2})([a-f\d]{2})$"#i);

pub fn from_special_block(
    block_array: &BlockArray,
    context: &StepContext,
    flags: &WasmFlags,
) -> HQResult<IrOpcode> {
    Ok(match block_array {
        BlockArray::NumberOrAngle(ty, value) => match ty {
            // number, positive number or angle
            4 | 5 | 8 => {
                // proactively convert to an integer if possible;
                // if a float is needed, it will be cast at const-fold time (TODO),
                // and if integers are disabled a float will be emitted anyway
                if flags.integers == Switch::On && value % 1.0 == 0.0 {
                    #[expect(
                        clippy::cast_possible_truncation,
                        reason = "integer-ness already confirmed; `as` is saturating."
                    )]
                    IrOpcode::hq_integer(HqIntegerFields(*value as i32))
                } else {
                    IrOpcode::hq_float(HqFloatFields(*value))
                }
            }
            // positive integer, integer
            6 | 7 => {
                hq_assert!(
                    value % 1.0 == 0.0,
                    "inputs of integer or positive integer types should be integers"
                );
                #[expect(
                    clippy::cast_possible_truncation,
                    reason = "integer-ness already confirmed; `as` is saturating."
                )]
                if flags.integers == Switch::On {
                    IrOpcode::hq_integer(HqIntegerFields(*value as i32))
                } else {
                    IrOpcode::hq_float(HqFloatFields(*value))
                }
            }
            // string
            10 => IrOpcode::hq_text(HqTextFields(value.to_string().into_boxed_str())),
            _ => hq_bad_proj!("bad project json (block array of type ({}, f64))", ty),
        },
        // a string input should really be a colour or a string, but often numbers
        // are serialised as strings in the project.json
        BlockArray::ColorOrString(ty, value) => match ty {
            // number, positive number or integer
            4 | 5 | 8 => {
                if let Ok(float) = value.parse() {
                    // proactively convert to an integer if possible;
                    // if a float is needed, it will be cast at const-fold time (TODO),
                    // and if integers are disabled a float will be emitted anyway
                    if flags.integers == Switch::On && float % 1.0 == 0.0 {
                        #[expect(
                            clippy::cast_possible_truncation,
                            reason = "integer-ness already confirmed; `as` is saturating."
                        )]
                        IrOpcode::hq_integer(HqIntegerFields(float as i32))
                    } else {
                        IrOpcode::hq_float(HqFloatFields(float))
                    }
                } else {
                    IrOpcode::hq_text(HqTextFields(value.clone()))
                }
            }
            // integer, positive integer
            6 | 7 =>
            {
                #[expect(
                    clippy::same_functions_in_if_condition,
                    reason = "false positive; called with different generic args"
                )]
                if flags.integers == Switch::On {
                    if let Ok(int) = value.parse() {
                        IrOpcode::hq_integer(HqIntegerFields(int))
                    } else if let Ok(float) = value.parse() {
                        IrOpcode::hq_float(HqFloatFields(float))
                    } else {
                        IrOpcode::hq_text(HqTextFields(value.clone()))
                    }
                } else if let Ok(float) = value.parse() {
                    IrOpcode::hq_float(HqFloatFields(float))
                } else {
                    IrOpcode::hq_text(HqTextFields(value.clone()))
                }
            }
            // colour
            9 => {
                let hex = (*SHORTHAND_HEX_COLOUR_REGEX).replace(value, "$1$1$2$2$3$3");
                if let Some(captures) = (*HEX_COLOUR_REGEX).captures(&hex) {
                    if let box [r, g, b] = (1..4)
                        .map(|i| &captures[i])
                        .map(|capture| {
                            u8::from_str_radix(capture, 16)
                                .map_err(|_| make_hq_bug!("hex substring out of u8 bounds"))
                        })
                        .collect::<HQResult<Box<[_]>>>()?
                    {
                        IrOpcode::hq_color_rgb(HqColorRgbFields { r, g, b })
                    } else {
                        IrOpcode::hq_color_rgb(HqColorRgbFields { r: 0, g: 0, b: 0 })
                    }
                } else {
                    IrOpcode::hq_color_rgb(HqColorRgbFields { r: 0, g: 0, b: 0 })
                }
            }
            // string
            10 => 'textBlock: {
                if flags.eager_number_parsing == Switch::On
                    && let Ok(float) = value.parse::<f64>()
                    && *float.to_string() == **value
                {
                    break 'textBlock if flags.integers == Switch::On && float % 1.0 == 0.0 {
                        #[expect(
                            clippy::cast_possible_truncation,
                            reason = "integer-ness already confirmed; `as` is saturating."
                        )]
                        IrOpcode::hq_integer(HqIntegerFields(float as i32))
                    } else {
                        IrOpcode::hq_float(HqFloatFields(float))
                    };
                }
                IrOpcode::hq_text(HqTextFields(value.clone()))
            }
            _ => hq_bad_proj!("bad project json (block array of type ({}, string))", ty),
        },
        BlockArray::Broadcast(ty, name, id) | BlockArray::VariableOrList(ty, name, id, _, _) => {
            match ty {
                11 => IrOpcode::hq_text(HqTextFields(name.clone())),
                12 => {
                    let target = context.target();
                    let variable = if let Some(var) = target.variables().get(id) {
                        var.clone()
                    } else if let Some(var) = context
                        .target()
                        .project()
                        .upgrade()
                        .ok_or_else(|| make_hq_bug!("couldn't upgrade Weak<Project>"))?
                        .global_variables()
                        .get(id)
                    {
                        var.clone()
                    } else {
                        hq_bad_proj!("variable not found")
                    };
                    *variable.is_used.try_borrow_mut()? = true;
                    IrOpcode::data_variable(DataVariableFields {
                        var: RefCell::new(variable.var.clone()),
                        local_read: RefCell::new(false),
                    })
                }
                13 => {
                    let target = context.target();
                    let list = if let Some(list) = target.lists().get(id) {
                        list.clone()
                    } else if let Some(list) = context
                        .target()
                        .project()
                        .upgrade()
                        .ok_or_else(|| make_hq_bug!("couldn't upgrade Weak<Project>"))?
                        .global_lists()
                        .get(id)
                    {
                        list.clone()
                    } else {
                        hq_bad_proj!("list not found")
                    };
                    *list.is_used.try_borrow_mut()? = true;
                    IrOpcode::data_listcontents(DataListcontentsFields {
                        list: list.list.clone(),
                    })
                }
                _ => hq_bad_proj!(
                    "bad project json (block array of type ({}, string, string))",
                    ty
                ),
            }
        }
    })
}
