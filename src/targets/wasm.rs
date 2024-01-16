use crate::ir::{
    BlockType as IrBlockType, IrBlock, IrOpcode, IrProject, Step, ThreadContext, ThreadStart,
};
use alloc::collections::BTreeMap;
use alloc::rc::Rc;
use alloc::string::String;
use alloc::vec::Vec;
use core::hash::BuildHasherDefault;
use hashers::fnv::FNV1aHasher64;
use indexmap::IndexMap;
use wasm_bindgen::prelude::*;
use wasm_encoder::{
    BlockType as WasmBlockType, CodeSection, ConstExpr, ElementSection, Elements, EntityType,
    ExportKind, ExportSection, Function, FunctionSection, GlobalSection, GlobalType, ImportSection,
    Instruction, MemArg, MemorySection, MemoryType, Module, RefType, TableSection, TableType,
    TypeSection, ValType,
};

fn instructions(
    op: &IrBlock,
    context: Rc<ThreadContext>,
    string_consts: &mut Vec<String>,
    steps: &IndexMap<(String, String), Step, BuildHasherDefault<FNV1aHasher64>>,
) -> Vec<Instruction<'static>> {
    use Instruction::*;
    use IrBlockType::*;
    use IrOpcode::*;
    let expected_output = *op.expected_output();
    let mut actual_output = *op.actual_output();
    let mut instructions = match &op.opcode() {
        looks_think => {
            if context.dbg {
                vec![Call(func_indices::DBG_ASSERT)]
            } else {
                vec![
                    I32Const(
                        context
                            .target_index
                            .try_into()
                            .expect("target index out of bounds (E002)"),
                    ),
                    Call(func_indices::LOOKS_THINK),
                ]
            }
        }
        looks_say => {
            if context.dbg {
                vec![Call(func_indices::DBG_LOG)]
            } else {
                vec![
                    I32Const(
                        context
                            .target_index
                            .try_into()
                            .expect("target index out of bounds (E003)"),
                    ),
                    Call(func_indices::LOOKS_SAY),
                ]
            }
        }
        operator_add => vec![F64Add],
        operator_subtract => vec![F64Sub],
        operator_divide => vec![F64Div],
        operator_multiply => vec![F64Mul],
        operator_mod => vec![Call(func_indices::FMOD)],
        operator_round => vec![F64Nearest],
        math_number { NUM }
        | math_integer { NUM }
        | math_angle { NUM }
        | math_whole_number { NUM }
        | math_positive_number { NUM } => {
            if expected_output == Text {
                actual_output = Text;
                instructions(
                    &IrBlock::from(text {
                        TEXT: format!("{}", NUM),
                    }),
                    Rc::clone(&context),
                    string_consts,
                    steps,
                )
            } else if expected_output == Any {
                actual_output = Any;
                vec![
                    I32Const(hq_value_types::FLOAT64),
                    I64Const((*NUM).to_bits() as i64),
                ]
            } else {
                vec![F64Const(*NUM)]
            }
        }
        text { TEXT } => {
            if expected_output == Number {
                actual_output = Number;
                vec![F64Const(TEXT.parse().unwrap_or(f64::NAN))]
            } else {
                let str_idx: i32 = {
                    if let Some(idx) = string_consts.iter().position(|string| string == TEXT) {
                        idx
                    } else {
                        string_consts.push(TEXT.clone());
                        string_consts.len() - 1
                    }
                }
                .try_into()
                .expect("string index out of bounds (E022)");
                if expected_output == Any {
                    actual_output = Any;
                    vec![
                        I32Const(hq_value_types::EXTERN_STRING_REF64),
                        I64Const(str_idx.into()),
                    ]
                } else {
                    vec![I32Const(str_idx), TableGet(table_indices::STRINGS)]
                }
            }
        }
        data_variable { VARIABLE } => {
            let var_index: i32 = context
                .vars
                .borrow()
                .iter()
                .position(|var| VARIABLE == var.id())
                .expect("couldn't find variable index (E033)")
                .try_into()
                .expect("variable index out of bounds (E034)");
            let var_offset: u64 = (byte_offset::VARS + 12 * var_index)
                .try_into()
                .expect("variable offset out of bounds (E035)");
            vec![
                I32Const(0),
                I32Load(MemArg {
                    offset: var_offset,
                    align: 2,
                    memory_index: 0,
                }),
                I32Const(0),
                I64Load(MemArg {
                    offset: var_offset + 4,
                    align: 2,
                    memory_index: 0,
                }),
            ]
        }
        data_setvariableto { VARIABLE } => {
            let var_index: i32 = context
                .vars
                .borrow()
                .iter()
                .position(|var| VARIABLE == var.id())
                .expect("couldn't find variable index (E033)")
                .try_into()
                .expect("variable index out of bounds (E034)");
            let var_offset: u64 = (byte_offset::VARS + 12 * var_index)
                .try_into()
                .expect("variable offset out of bounds (E035)");
            vec![
                LocalSet(step_func_locals::I64),
                LocalSet(step_func_locals::I32),
                I32Const(0),
                LocalGet(step_func_locals::I32),
                I32Store(MemArg {
                    offset: var_offset,
                    align: 2,
                    memory_index: 0,
                }),
                I32Const(0),
                LocalGet(step_func_locals::I64),
                I64Store(MemArg {
                    offset: var_offset + 4,
                    align: 2,
                    memory_index: 0,
                }),
            ]
        }
        data_teevariable { VARIABLE } => {
            let var_index: i32 = context
                .vars
                .borrow()
                .iter()
                .position(|var| VARIABLE == var.id())
                .expect("couldn't find variable index (E033)")
                .try_into()
                .expect("variable index out of bounds (E034)");
            let var_offset: u64 = (byte_offset::VARS + 12 * var_index)
                .try_into()
                .expect("variable offset out of bounds (E035)");
            vec![
                LocalSet(step_func_locals::I64),
                LocalSet(step_func_locals::I32),
                I32Const(0),
                LocalGet(step_func_locals::I32),
                I32Store(MemArg {
                    offset: var_offset,
                    align: 2,
                    memory_index: 0,
                }),
                I32Const(0),
                LocalGet(step_func_locals::I64),
                I64Store(MemArg {
                    offset: var_offset + 4,
                    align: 2,
                    memory_index: 0,
                }),
                LocalGet(step_func_locals::I32),
                LocalGet(step_func_locals::I64),
            ]
        }
        operator_lt => vec![F64Lt, I64ExtendI32S],
        operator_gt => vec![F64Gt, I64ExtendI32S],
        operator_and => vec![I64And],
        operator_or => vec![I64Or],
        operator_not => vec![I64Eqz, I64ExtendI32S],
        operator_equals => vec![Call(func_indices::OPERATOR_EQUALS)],
        operator_random => vec![Call(func_indices::OPERATOR_RANDOM)],
        operator_join => vec![Call(func_indices::OPERATOR_JOIN)],
        operator_letter_of => vec![Call(func_indices::OPERATOR_LETTEROF)],
        operator_length => vec![Call(func_indices::OPERATOR_LENGTH)],
        operator_contains => vec![Call(func_indices::OPERATOR_CONTAINS)],
        operator_mathop { OPERATOR } => match OPERATOR.as_str() {
            "abs" => vec![F64Abs],
            "floor" => vec![F64Floor],
            "ceiling" => vec![F64Ceil],
            "sqrt" => vec![F64Sqrt],
            "sin" => vec![Call(func_indices::MATHOP_SIN)],
            "cos" => vec![Call(func_indices::MATHOP_COS)],
            "tan" => vec![Call(func_indices::MATHOP_TAN)],
            "asin" => vec![Call(func_indices::MATHOP_ASIN)],
            "acos" => vec![Call(func_indices::MATHOP_ACOS)],
            "atan" => vec![Call(func_indices::MATHOP_ATAN)],
            "ln" => vec![Call(func_indices::MATHOP_LN)],
            "log" => vec![Call(func_indices::MATHOP_LOG)],
            "e ^" => vec![Call(func_indices::MATHOP_POW_E)],
            "10 ^" => vec![Call(func_indices::MATHOP_POW10)],
            _ => panic!("invalid OPERATOR field (E041)"),
        },
        sensing_timer => vec![Call(func_indices::SENSING_TIMER)],
        sensing_resettimer => vec![Call(func_indices::SENSING_RESETTIMER)],
        pen_clear => vec![Call(func_indices::PEN_CLEAR)],
        pen_stamp => todo!(),
        pen_penDown => vec![
            I32Const(0),
            I32Const(1),
            I32Store8(MemArg {
                offset: (context.target_index - 1) as u64 * u64::try_from(SPRITE_INFO_LEN).unwrap()
                    + u64::try_from(byte_offset::VARS).unwrap()
                    + u64::try_from(context.vars.borrow().len()).unwrap() * 12
                    + u64::try_from(sprite_info_offsets::PEN_DOWN).unwrap(),
                align: 0,
                memory_index: 0,
            }),
            I32Const(0),
            F64Load(MemArg {
                offset: (context.target_index - 1) as u64 * u64::try_from(SPRITE_INFO_LEN).unwrap()
                    + u64::try_from(byte_offset::VARS).unwrap()
                    + u64::try_from(context.vars.borrow().len()).unwrap() * 12
                    + u64::try_from(sprite_info_offsets::PEN_SIZE).unwrap(),
                align: 0,
                memory_index: 0,
            }),
            I32Const(0),
            F64Load(MemArg {
                offset: (context.target_index - 1) as u64 * u64::try_from(SPRITE_INFO_LEN).unwrap()
                    + u64::try_from(byte_offset::VARS).unwrap()
                    + u64::try_from(context.vars.borrow().len()).unwrap() * 12
                    + u64::try_from(sprite_info_offsets::X_POS).unwrap(),
                align: 0,
                memory_index: 0,
            }),
            I32Const(0),
            F64Load(MemArg {
                offset: (context.target_index - 1) as u64 * u64::try_from(SPRITE_INFO_LEN).unwrap()
                    + u64::try_from(byte_offset::VARS).unwrap()
                    + u64::try_from(context.vars.borrow().len()).unwrap() * 12
                    + u64::try_from(sprite_info_offsets::Y_POS).unwrap(),
                align: 0,
                memory_index: 0,
            }),
            I32Const(0),
            F32Load(MemArg {
                offset: (context.target_index - 1) as u64 * u64::try_from(SPRITE_INFO_LEN).unwrap()
                    + u64::try_from(byte_offset::VARS).unwrap()
                    + u64::try_from(context.vars.borrow().len()).unwrap() * 12
                    + u64::try_from(sprite_info_offsets::PEN_R).unwrap(),
                align: 0,
                memory_index: 0,
            }),
            I32Const(0),
            F32Load(MemArg {
                offset: (context.target_index - 1) as u64 * u64::try_from(SPRITE_INFO_LEN).unwrap()
                    + u64::try_from(byte_offset::VARS).unwrap()
                    + u64::try_from(context.vars.borrow().len()).unwrap() * 12
                    + u64::try_from(sprite_info_offsets::PEN_G).unwrap(),
                align: 0,
                memory_index: 0,
            }),
            I32Const(0),
            F32Load(MemArg {
                offset: (context.target_index - 1) as u64 * u64::try_from(SPRITE_INFO_LEN).unwrap()
                    + u64::try_from(byte_offset::VARS).unwrap()
                    + u64::try_from(context.vars.borrow().len()).unwrap() * 12
                    + u64::try_from(sprite_info_offsets::PEN_B).unwrap(),
                align: 0,
                memory_index: 0,
            }),
            I32Const(0),
            F32Load(MemArg {
                offset: (context.target_index - 1) as u64 * u64::try_from(SPRITE_INFO_LEN).unwrap()
                    + u64::try_from(byte_offset::VARS).unwrap()
                    + u64::try_from(context.vars.borrow().len()).unwrap() * 12
                    + u64::try_from(sprite_info_offsets::PEN_A).unwrap(),
                align: 0,
                memory_index: 0,
            }),
            Call(func_indices::PEN_DOWN),
        ],
        motion_gotoxy => vec![
            LocalSet(step_func_locals::F64), // y
            LocalSet(step_func_locals::F64_2), // x
            I32Const(0),
            I32Load8S(MemArg {
                offset: (context.target_index - 1) as u64 * u64::try_from(SPRITE_INFO_LEN).unwrap()
                    + u64::try_from(byte_offset::VARS).unwrap()
                    + u64::try_from(context.vars.borrow().len()).unwrap() * 12
                    + u64::try_from(sprite_info_offsets::PEN_DOWN).unwrap(),
                align: 0,
                memory_index: 0,
            }),
            If(WasmBlockType::Empty),
            I32Const(0),
            F64Load(MemArg {
                offset: (context.target_index - 1) as u64 * u64::try_from(SPRITE_INFO_LEN).unwrap()
                    + u64::try_from(byte_offset::VARS).unwrap()
                    + u64::try_from(context.vars.borrow().len()).unwrap() * 12
                    + u64::try_from(sprite_info_offsets::PEN_SIZE).unwrap(),
                align: 0,
                memory_index: 0,
            }),
            I32Const(0),
            F64Load(MemArg {
                offset: (context.target_index - 1) as u64 * u64::try_from(SPRITE_INFO_LEN).unwrap()
                    + u64::try_from(byte_offset::VARS).unwrap()
                    + u64::try_from(context.vars.borrow().len()).unwrap() * 12
                    + u64::try_from(sprite_info_offsets::X_POS).unwrap(),
                align: 0,
                memory_index: 0,
            }),
            I32Const(0),
            F64Load(MemArg {
                offset: (context.target_index - 1) as u64 * u64::try_from(SPRITE_INFO_LEN).unwrap()
                    + u64::try_from(byte_offset::VARS).unwrap()
                    + u64::try_from(context.vars.borrow().len()).unwrap() * 12
                    + u64::try_from(sprite_info_offsets::Y_POS).unwrap(),
                align: 0,
                memory_index: 0,
            }),
            LocalGet(step_func_locals::F64),
            LocalGet(step_func_locals::F64_2),
            I32Const(0),
            F32Load(MemArg {
                offset: (context.target_index - 1) as u64 * u64::try_from(SPRITE_INFO_LEN).unwrap()
                    + u64::try_from(byte_offset::VARS).unwrap()
                    + u64::try_from(context.vars.borrow().len()).unwrap() * 12
                    + u64::try_from(sprite_info_offsets::PEN_R).unwrap(),
                align: 0,
                memory_index: 0,
            }),
            I32Const(0),
            F32Load(MemArg {
                offset: (context.target_index - 1) as u64 * u64::try_from(SPRITE_INFO_LEN).unwrap()
                    + u64::try_from(byte_offset::VARS).unwrap()
                    + u64::try_from(context.vars.borrow().len()).unwrap() * 12
                    + u64::try_from(sprite_info_offsets::PEN_G).unwrap(),
                align: 0,
                memory_index: 0,
            }),
            I32Const(0),
            F32Load(MemArg {
                offset: (context.target_index - 1) as u64 * u64::try_from(SPRITE_INFO_LEN).unwrap()
                    + u64::try_from(byte_offset::VARS).unwrap()
                    + u64::try_from(context.vars.borrow().len()).unwrap() * 12
                    + u64::try_from(sprite_info_offsets::PEN_B).unwrap(),
                align: 0,
                memory_index: 0,
            }),
            I32Const(0),
            F32Load(MemArg {
                offset: (context.target_index - 1) as u64 * u64::try_from(SPRITE_INFO_LEN).unwrap()
                    + u64::try_from(byte_offset::VARS).unwrap()
                    + u64::try_from(context.vars.borrow().len()).unwrap() * 12
                    + u64::try_from(sprite_info_offsets::PEN_A).unwrap(),
                align: 0,
                memory_index: 0,
            }),
            Call(func_indices::PEN_LINETO),
            End,
        ],
        pen_penUp => vec![
            I32Const(0),
            I32Const(0),
            I32Store8(MemArg {
                offset: (context.target_index - 1) as u64 * u64::try_from(SPRITE_INFO_LEN).unwrap()
                    + u64::try_from(byte_offset::VARS).unwrap()
                    + u64::try_from(context.vars.borrow().len()).unwrap() * 12
                    + u64::try_from(sprite_info_offsets::PEN_DOWN).unwrap(),
                align: 0,
                memory_index: 0,
            }),
        ],
        pen_setPenColorToColor => vec![
            I32Const(
                context
                    .target_index
                    .try_into()
                    .expect("target index out of bounds (E003)"),
            ),
            Call(func_indices::PEN_SETCOLOR),
        ],
        pen_changePenColorParamBy => vec![
            I32Const(
                context
                    .target_index
                    .try_into()
                    .expect("target index out of bounds (E003)"),
            ),
            Call(func_indices::PEN_CHANGECOLORPARAM),
        ],
        pen_setPenColorParamTo => vec![
            I32Const(
                context
                    .target_index
                    .try_into()
                    .expect("target index out of bounds (E003)"),
            ),
            Call(func_indices::PEN_SETCOLORPARAM),
        ],
        pen_changePenSizeBy => vec![
            I32Const(
                context
                    .target_index
                    .try_into()
                    .expect("target index out of bounds (E003)"),
            ),
            Call(func_indices::PEN_CHANGESIZE),
        ],
        pen_setPenSizeTo => vec![
            LocalSet(step_func_locals::F64),
            I32Const(0),
            LocalGet(step_func_locals::F64),
            F64Store(MemArg {
                offset: (context.target_index - 1) as u64 * u64::try_from(SPRITE_INFO_LEN).unwrap()
                    + u64::try_from(byte_offset::VARS).unwrap()
                    + u64::try_from(context.vars.borrow().len()).unwrap() * 12
                    + u64::try_from(sprite_info_offsets::PEN_SIZE).unwrap(),
                align: 2,
                memory_index: 0,
            }),
        ],
        pen_setPenShadeToNumber => todo!(),
        pen_changePenShadeBy => todo!(),
        pen_setPenHueToNumber => vec![
            I32Const(
                context
                    .target_index
                    .try_into()
                    .expect("target index out of bounds (E003)"),
            ),
            Call(func_indices::PEN_SETHUE),
        ],
        pen_changePenHueBy => vec![
            I32Const(
                context
                    .target_index
                    .try_into()
                    .expect("target index out of bounds (E003)"),
            ),
            Call(func_indices::PEN_CHANGEHUE),
        ],
        hq_drop(n) => vec![Drop; 2 * *n],
        hq_goto { step: None, .. } => {
            let threads_offset: i32 = (byte_offset::VARS as usize
                + 12 * context.vars.borrow().len()
                + usize::try_from(SPRITE_INFO_LEN).unwrap() * (context.target_num - 1))
                .try_into()
                .expect("thread_offset out of bounds");
            vec![
                LocalGet(0),
                I32Const(threads_offset),
                I32Add, // destination (= current thread pos in memory)
                LocalGet(0),
                I32Const(threads_offset + 4),
                I32Add, // source (= current thread pos + 4)
                I32Const(0),
                I32Load(MemArg {
                    offset: byte_offset::THREAD_NUM
                        .try_into()
                        .expect("THREAD_NUM out of bounds (E009)"),
                    align: 2,
                    memory_index: 0,
                }),
                I32Const(4),
                I32Mul,
                LocalGet(0),
                I32Sub, // length (threadnum * 4 - current thread pos)
                MemoryCopy {
                    src_mem: 0,
                    dst_mem: 0,
                },
                I32Const(0),
                I32Const(0),
                I32Load(MemArg {
                    offset: byte_offset::THREAD_NUM
                        .try_into()
                        .expect("THREAD_NUM out of bounds (E010)"),
                    align: 2,
                    memory_index: 0,
                }),
                I32Const(1),
                I32Sub,
                I32Store(MemArg {
                    offset: byte_offset::THREAD_NUM
                        .try_into()
                        .expect("THREAD_NUM out of bounds (E011)"),
                    align: 2,
                    memory_index: 0,
                }),
                I32Const(0),
                Return,
            ]
        }
        hq_goto {
            step: Some(next_step_id),
            does_yield: true,
        } => {
            let next_step_index = steps.get_index_of(next_step_id).unwrap();
            let threads_offset: u64 = (byte_offset::VARS as usize
                + 12 * context.vars.borrow().len()
                + usize::try_from(SPRITE_INFO_LEN).unwrap() * (context.target_num - 1))
                .try_into()
                .expect("threads_offset length out of bounds");
            vec![
                LocalGet(0),
                I32Const(
                    next_step_index
                        .try_into()
                        .expect("step index out of bounds (E001)"),
                ),
                I32Store(MemArg {
                    offset: threads_offset,
                    align: 2,
                    memory_index: 0,
                }),
                I32Const(1),
                Return,
            ]
        }
        hq_goto {
            step: Some(next_step_id),
            does_yield: false,
        } => {
            let next_step_index = steps.get_index_of(next_step_id).unwrap();
            vec![
                LocalGet(step_func_locals::MEM_LOCATION),
                Call(
                    BUILTIN_FUNCS
                        + u32::try_from(next_step_index).expect("next_step_index out of bounds"),
                ),
                Return,
            ]
        }
        hq_goto_if { step: None, .. } => {
            let threads_offset: i32 = (byte_offset::VARS as usize
                + 12 * context.vars.borrow().len()
                + usize::try_from(SPRITE_INFO_LEN).unwrap() * (context.target_num - 1))
                .try_into()
                .expect("thread_offset out of bounds");
            vec![
                I32WrapI64,
                If(WasmBlockType::Empty),
                LocalGet(0),
                I32Const(threads_offset),
                I32Add, // destination (= current thread pos in memory)
                LocalGet(0),
                I32Const(threads_offset + 4),
                I32Add, // source (= current thread pos + 4)
                I32Const(0),
                I32Load(MemArg {
                    offset: byte_offset::THREAD_NUM
                        .try_into()
                        .expect("THREAD_NUM out of bounds (E009)"),
                    align: 2,
                    memory_index: 0,
                }),
                I32Const(4),
                I32Mul,
                LocalGet(0),
                I32Sub, // length (threadnum * 4 - current thread pos)
                MemoryCopy {
                    src_mem: 0,
                    dst_mem: 0,
                },
                I32Const(0),
                I32Const(0),
                I32Load(MemArg {
                    offset: byte_offset::THREAD_NUM
                        .try_into()
                        .expect("THREAD_NUM out of bounds (E010)"),
                    align: 2,
                    memory_index: 0,
                }),
                I32Const(1),
                I32Sub,
                I32Store(MemArg {
                    offset: byte_offset::THREAD_NUM
                        .try_into()
                        .expect("THREAD_NUM out of bounds (E011)"),
                    align: 2,
                    memory_index: 0,
                }),
                I32Const(0),
                Return,
                End,
            ]
        }
        hq_goto_if {
            step: Some(next_step_id),
            does_yield: true,
        } => {
            let next_step_index = steps.get_index_of(next_step_id).unwrap();
            let threads_offset: u64 = (byte_offset::VARS as usize
                + 12 * context.vars.borrow().len()
                + usize::try_from(SPRITE_INFO_LEN).unwrap() * (context.target_num - 1))
                .try_into()
                .expect("threads_offset length out of bounds");
            vec![
                I32WrapI64,
                If(WasmBlockType::Empty),
                LocalGet(0),
                I32Const(
                    next_step_index
                        .try_into()
                        .expect("step index out of bounds (E001)"),
                ),
                I32Store(MemArg {
                    offset: threads_offset,
                    align: 2,
                    memory_index: 0,
                }),
                I32Const(1),
                Return,
                End,
            ]
        }
        hq_goto_if {
            step: Some(next_step_id),
            does_yield: false,
        } => {
            let next_step_index = steps.get_index_of(next_step_id).unwrap();
            vec![
                I32WrapI64,
                If(WasmBlockType::Empty),
                LocalGet(step_func_locals::MEM_LOCATION),
                Call(
                    BUILTIN_FUNCS
                        + u32::try_from(next_step_index).expect("next_step_index out of bounds"),
                ),
                Return,
                End,
            ]
        }
        _ => todo!(),
    };
    instructions.append(&mut match (actual_output, expected_output) {
        (Text, Number) => vec![Call(func_indices::CAST_PRIMITIVE_STRING_FLOAT)],
        (Text, Boolean) => vec![Call(func_indices::CAST_PRIMITIVE_STRING_BOOL)],
        (Text, Any) => vec![
            LocalSet(step_func_locals::EXTERNREF),
            I32Const(hq_value_types::EXTERN_STRING_REF64),
            LocalGet(step_func_locals::EXTERNREF),
            Call(func_indices::TABLE_ADD_STRING),
            I64ExtendI32S,
        ],
        (Boolean, Number) => vec![Call(func_indices::CAST_BOOL_FLOAT)],
        (Boolean, Text) => vec![Call(func_indices::CAST_BOOL_STRING)],
        (Boolean, Any) => vec![
            LocalSet(step_func_locals::I64),
            I32Const(hq_value_types::BOOL64),
            LocalGet(step_func_locals::I64),
        ],
        (Number, Text) => vec![Call(func_indices::CAST_PRIMITIVE_FLOAT_STRING)],
        (Number, Boolean) => vec![Call(func_indices::CAST_FLOAT_BOOL)],
        (Number, Any) => vec![
            LocalSet(step_func_locals::F64),
            I32Const(hq_value_types::FLOAT64),
            LocalGet(step_func_locals::F64),
            I64ReinterpretF64,
        ],
        (Any, Text) => vec![Call(func_indices::CAST_ANY_STRING)],
        (Any, Number) => vec![Call(func_indices::CAST_ANY_FLOAT)],
        (Any, Boolean) => vec![Call(func_indices::CAST_ANY_BOOL)],
        _ => vec![],
    });
    if op.does_request_redraw() && !(*op.opcode() == looks_say && context.dbg) {
        instructions.append(&mut vec![
            I32Const(byte_offset::REDRAW_REQUESTED),
            I32Const(1),
            I32Store8(MemArg {
                offset: 0,
                align: 0,
                memory_index: 0,
            }),
        ]);
    }
    instructions
}

pub trait CompileToWasm {
    fn compile_wasm(
        &self,
        step_funcs: &mut IndexMap<
            Option<(String, String)>,
            Function,
            BuildHasherDefault<FNV1aHasher64>,
        >,
        string_consts: &mut Vec<String>,
        steps: &IndexMap<(String, String), Step, BuildHasherDefault<FNV1aHasher64>>,
    ) -> u32;
}

impl CompileToWasm for (&(String, String), &Step) {
    fn compile_wasm(
        &self,
        step_funcs: &mut IndexMap<
            Option<(String, String)>,
            Function,
            BuildHasherDefault<FNV1aHasher64>,
        >,
        string_consts: &mut Vec<String>,
        steps: &IndexMap<(String, String), Step, BuildHasherDefault<FNV1aHasher64>>,
    ) -> u32 {
        if step_funcs.contains_key(&Some(self.0.clone())) {
            return step_funcs
                .get_index_of(&Some(self.0.clone()))
                .unwrap()
                .try_into()
                .expect("IndexMap index out of bounds");
        }
        let locals = vec![
            ValType::Ref(RefType::EXTERNREF),
            ValType::F64,
            ValType::I64,
            ValType::I32,
            ValType::I32,
            ValType::F64,
        ];
        let mut func = Function::new_with_locals_types(locals);
        for op in self.1.opcodes() {
            let instrs = instructions(op, self.1.context(), string_consts, steps);
            for instr in instrs {
                func.instruction(&instr);
            }
        }
        func.instruction(&Instruction::End);
        step_funcs.insert(Some(self.0.clone()), func);
        (step_funcs.len() - 1)
            .try_into()
            .expect("step_funcs length out of bounds")
    }
}

#[wasm_bindgen(getter_with_clone)]
pub struct WasmProject {
    pub wasm_bytes: Vec<u8>,
    pub string_consts: Vec<String>,
    pub target_names: Vec<String>,
}
/*
#[wasm_bindgen]
impl WasmProject {
    pub fn wasm_bytes(&self) -> &Vec<u8> {
        &self.wasm_bytes
    }
    pub fn string_consts(&self) -> &Vec<String> {
        &self.string_consts
    }
    pub fn target_names(&self) -> &Vec<String> {
        &self.target_names
    }
}*/

pub mod step_func_locals {
    pub const MEM_LOCATION: u32 = 0;
    pub const EXTERNREF: u32 = 1;
    pub const F64: u32 = 2;
    pub const I64: u32 = 3;
    pub const I32: u32 = 4;
    pub const I32_2: u32 = 5;
    pub const F64_2: u32 = 6;
}

pub mod func_indices {
    /* imported funcs */
    pub const DBG_LOG: u32 = 0;
    pub const DBG_ASSERT: u32 = 1;
    pub const LOOKS_SAY: u32 = 2;
    pub const LOOKS_THINK: u32 = 3;
    pub const CAST_PRIMITIVE_FLOAT_STRING: u32 = 4; // js functions can only return 1 value so need wrapper functions for casting
    pub const CAST_PRIMITIVE_STRING_FLOAT: u32 = 5;
    pub const CAST_PRIMITIVE_STRING_BOOL: u32 = 6;
    pub const DBG_LOGI32: u32 = 7;
    pub const OPERATOR_EQUALS: u32 = 8;
    pub const OPERATOR_RANDOM: u32 = 9;
    pub const OPERATOR_JOIN: u32 = 10;
    pub const OPERATOR_LETTEROF: u32 = 11;
    pub const OPERATOR_LENGTH: u32 = 12;
    pub const OPERATOR_CONTAINS: u32 = 13;
    pub const MATHOP_SIN: u32 = 14;
    pub const MATHOP_COS: u32 = 15;
    pub const MATHOP_TAN: u32 = 16;
    pub const MATHOP_ASIN: u32 = 17;
    pub const MATHOP_ACOS: u32 = 18;
    pub const MATHOP_ATAN: u32 = 19;
    pub const MATHOP_LN: u32 = 20;
    pub const MATHOP_LOG: u32 = 21;
    pub const MATHOP_POW_E: u32 = 22;
    pub const MATHOP_POW10: u32 = 23;
    pub const SENSING_TIMER: u32 = 24;
    pub const SENSING_RESETTIMER: u32 = 25;
    pub const PEN_CLEAR: u32 = 26;
    pub const PEN_DOWN: u32 = 27;
    pub const PEN_LINETO: u32 = 28;
    pub const PEN_SETCOLOR: u32 = 29;
    pub const PEN_CHANGECOLORPARAM: u32 = 30;
    pub const PEN_SETCOLORPARAM: u32 = 31;
    pub const PEN_CHANGESIZE: u32 = 32;
    pub const PEN_SETHUE: u32 = 33;
    pub const PEN_CHANGEHUE: u32 = 34;

    /* wasm funcs */
    pub const FMOD: u32 = 35;
    pub const CAST_FLOAT_BOOL: u32 = 36;
    pub const CAST_BOOL_FLOAT: u32 = 37;
    pub const CAST_BOOL_STRING: u32 = 38;
    pub const CAST_ANY_STRING: u32 = 39;
    pub const CAST_ANY_FLOAT: u32 = 40;
    pub const CAST_ANY_BOOL: u32 = 41;
    pub const TABLE_ADD_STRING: u32 = 42;
    pub const SPRITE_UPDATE_PEN_COLOR: u32 = 43;
}
pub const BUILTIN_FUNCS: u32 = 44;
pub const IMPORTED_FUNCS: u32 = 35;

pub mod types {
    #![allow(non_upper_case_globals)]
    pub const F64_NORESULT: u32 = 0;
    pub const NOPARAM_NORESULT: u32 = 1;
    pub const F64x2_F64: u32 = 2;
    pub const F64I32_NORESULT: u32 = 3;
    pub const I32_NORESULT: u32 = 4;
    pub const I32_I32: u32 = 5;
    pub const I32_F64: u32 = 6;
    pub const F64_I32: u32 = 7;
    pub const F64_EXTERNREF: u32 = 8;
    pub const EXTERNREF_F64: u32 = 9;
    pub const EXTERNREF_I32: u32 = 10;
    pub const I32x2_I32x2: u32 = 11;
    pub const I32I64_I32F64: u32 = 12;
    pub const I32F64_I32I64: u32 = 13;
    pub const NOPARAM_I32I64: u32 = 14;
    pub const I32I64_I32I64: u32 = 15;
    pub const NOPARAM_I32F64: u32 = 16;
    pub const F64_I64: u32 = 17;
    pub const I64_F64: u32 = 18;
    pub const I64_I64: u32 = 19;
    pub const I32I64_I64: u32 = 20;
    pub const I32I64_F64: u32 = 21;
    pub const F64_I32I64: u32 = 22;
    pub const NOPARAM_I64: u32 = 23;
    pub const NOPARAM_F64: u32 = 24;
    pub const I32I64_EXTERNREF: u32 = 25;
    pub const NOPARAM_EXTERNREF: u32 = 26;
    pub const I64_EXTERNREF: u32 = 27;
    pub const I32I64I32_NORESULT: u32 = 28;
    pub const I32I64_NORESULT: u32 = 29;
    pub const I32I64I32I64_I64: u32 = 30;
    pub const F64I32I64_EXTERNREF: u32 = 31;
    pub const I32I64I32I64_EXTERNREF: u32 = 32;
    pub const F64_F64: u32 = 33;
    pub const NOPARAM_I32: u32 = 34;
    pub const I32x2_NORESULT: u32 = 35;
    pub const EXTERNREFF64I32_NORESULT: u32 = 36;
    pub const F64x3F32x4_NORESULT: u32 = 37;
    pub const F64x5F32x4_NORESULT: u32 = 38;
}

pub mod table_indices {
    pub const STEP_FUNCS: u32 = 0;
    pub const STRINGS: u32 = 1;
}

pub mod hq_value_types {
    pub const FLOAT64: i32 = 0;
    pub const BOOL64: i32 = 1;
    pub const EXTERN_STRING_REF64: i32 = 2;
}

// the number of bytes that one step takes up in linear memory
pub const THREAD_BYTE_LEN: i32 = 4;

pub mod byte_offset {
    pub const REDRAW_REQUESTED: i32 = 0;
    pub const THREAD_NUM: i32 = 4;
    pub const VARS: i32 = 8;
}

pub const SPRITE_INFO_LEN: i32 = 56;

pub mod sprite_info_offsets {
    pub const X_POS: i32 = 0;
    pub const Y_POS: i32 = 8;
    pub const PEN_COLOR: i32 = 16;
    pub const PEN_SATURATION: i32 = 20;
    pub const PEN_VALUE: i32 = 24;
    pub const PEN_TRANSPARENCY: i32 = 28;
    pub const PEN_R: i32 = 32;
    pub const PEN_G: i32 = 36;
    pub const PEN_B: i32 = 40;
    pub const PEN_A: i32 = 44;
    pub const PEN_SIZE: i32 = 48;
    pub const PEN_DOWN: i32 = 56;
}

impl From<IrProject> for WasmProject {
    fn from(project: IrProject) -> Self {
        let mut module = Module::new();

        let mut imports = ImportSection::new();
        let mut functions = FunctionSection::new();
        let mut types = TypeSection::new();
        let mut code = CodeSection::new();
        let mut exports = ExportSection::new();
        let mut tables = TableSection::new();
        let mut elements = ElementSection::new();
        let mut memories = MemorySection::new();
        let mut globals = GlobalSection::new();

        memories.memory(MemoryType {
            minimum: 1,
            maximum: None,
            memory64: false,
            shared: false,
        });

        types.function([ValType::F64], []);
        types.function([], []);
        types.function([ValType::F64, ValType::F64], [ValType::F64]);
        types.function([ValType::F64, ValType::I32], []);
        types.function([ValType::I32], []);
        types.function([ValType::I32], [ValType::I32]);
        types.function([ValType::I32], [ValType::F64]);
        types.function([ValType::F64], [ValType::I32]);
        types.function([ValType::F64], [ValType::Ref(RefType::EXTERNREF)]);
        types.function([ValType::Ref(RefType::EXTERNREF)], [ValType::F64]);
        types.function([ValType::Ref(RefType::EXTERNREF)], [ValType::I32]);
        types.function([ValType::I32, ValType::I32], [ValType::I32, ValType::I32]);
        types.function([ValType::I32, ValType::I64], [ValType::I32, ValType::F64]);
        types.function([ValType::I32, ValType::F64], [ValType::I32, ValType::I64]);
        types.function([], [ValType::I32, ValType::I64]);
        types.function([ValType::I32, ValType::I64], [ValType::I32, ValType::I64]);
        types.function([], [ValType::I32, ValType::F64]);
        types.function([ValType::F64], [ValType::I64]);
        types.function([ValType::I64], [ValType::F64]);
        types.function([ValType::I64], [ValType::I64]);
        types.function([ValType::I32, ValType::I64], [ValType::I64]);
        types.function([ValType::I32, ValType::I64], [ValType::F64]);
        types.function([ValType::F64], [ValType::I32, ValType::I64]);
        types.function([], [ValType::I64]);
        types.function([], [ValType::F64]);
        types.function(
            [ValType::I32, ValType::I64],
            [ValType::Ref(RefType::EXTERNREF)],
        );
        types.function([], [ValType::Ref(RefType::EXTERNREF)]);
        types.function([ValType::I64], [ValType::Ref(RefType::EXTERNREF)]);
        types.function([ValType::I32, ValType::I64, ValType::I32], []);
        types.function([ValType::I32, ValType::I64], []);
        types.function(
            [ValType::I32, ValType::I64, ValType::I32, ValType::I64],
            [ValType::I64],
        );
        types.function(
            [ValType::F64, ValType::I32, ValType::I64],
            [ValType::Ref(RefType::EXTERNREF)],
        );
        types.function(
            [ValType::I32, ValType::I64, ValType::I32, ValType::I64],
            [ValType::Ref(RefType::EXTERNREF)],
        );
        types.function([ValType::F64], [ValType::F64]);
        types.function([], [ValType::I32]);
        types.function([ValType::I32, ValType::I32], []);
        types.function(
            [ValType::Ref(RefType::EXTERNREF), ValType::F64, ValType::I32],
            [],
        );
        types.function(
            [ValType::F64, ValType::F64, ValType::F64, ValType::F32, ValType::F32, ValType::F32, ValType::F32],
            [],
        );
        types.function(
            [ValType::F64, ValType::F64, ValType::F64, ValType::F64, ValType::F64, ValType::F32, ValType::F32, ValType::F32, ValType::F32],
            [],
        );

        imports.import("dbg", "log", EntityType::Function(types::I32I64_NORESULT));
        imports.import(
            "dbg",
            "assert",
            EntityType::Function(types::I32I64_NORESULT),
        );
        imports.import(
            "runtime",
            "looks_say",
            EntityType::Function(types::I32I64I32_NORESULT),
        );
        imports.import(
            "runtime",
            "looks_think",
            EntityType::Function(types::I32I64I32_NORESULT),
        );
        imports.import(
            "cast",
            "floattostring",
            EntityType::Function(types::F64_EXTERNREF),
        );
        imports.import(
            "cast",
            "stringtofloat",
            EntityType::Function(types::EXTERNREF_F64),
        );
        imports.import(
            "cast",
            "stringtobool",
            EntityType::Function(types::EXTERNREF_I32),
        );
        imports.import("dbg", "logi32", EntityType::Function(types::I32_I32));
        imports.import(
            "runtime",
            "operator_equals",
            EntityType::Function(types::I32I64I32I64_I64),
        );
        imports.import(
            "runtime",
            "operator_random",
            EntityType::Function(types::F64x2_F64),
        );
        imports.import(
            "runtime",
            "operator_join",
            EntityType::Function(types::I32I64I32I64_EXTERNREF),
        );
        imports.import(
            "runtime",
            "operator_letterof",
            EntityType::Function(types::F64I32I64_EXTERNREF),
        );
        imports.import(
            "runtime",
            "operator_length",
            EntityType::Function(types::I32I64_F64),
        );
        imports.import(
            "runtime",
            "operator_contains",
            EntityType::Function(types::I32I64I32I64_I64),
        );
        imports.import(
            "runtime",
            "mathop_sin",
            EntityType::Function(types::F64_F64),
        );
        imports.import(
            "runtime",
            "mathop_cos",
            EntityType::Function(types::F64_F64),
        );
        imports.import(
            "runtime",
            "mathop_tan",
            EntityType::Function(types::F64_F64),
        );
        imports.import(
            "runtime",
            "mathop_asin",
            EntityType::Function(types::F64_F64),
        );
        imports.import(
            "runtime",
            "mathop_acos",
            EntityType::Function(types::F64_F64),
        );
        imports.import(
            "runtime",
            "mathop_atan",
            EntityType::Function(types::F64_F64),
        );
        imports.import("runtime", "mathop_ln", EntityType::Function(types::F64_F64));
        imports.import(
            "runtime",
            "mathop_log",
            EntityType::Function(types::F64_F64),
        );
        imports.import(
            "runtime",
            "mathop_pow_e",
            EntityType::Function(types::F64_F64),
        );
        imports.import(
            "runtime",
            "mathop_pow10",
            EntityType::Function(types::F64_F64),
        );
        imports.import(
            "runtime",
            "sensing_timer",
            EntityType::Function(types::NOPARAM_F64),
        );
        imports.import(
            "runtime",
            "sensing_resettimer",
            EntityType::Function(types::NOPARAM_NORESULT),
        );
        imports.import(
            "runtime",
            "pen_clear",
            EntityType::Function(types::NOPARAM_NORESULT),
        );
        imports.import(
            "runtime",
            "pen_down",
            EntityType::Function(types::F64x3F32x4_NORESULT),
        );
        imports.import(
            "runtime",
            "pen_lineto",
            EntityType::Function(types::F64x5F32x4_NORESULT),
        );
        imports.import(
            "runtime",
            "pen_setcolor",
            EntityType::Function(types::I32x2_NORESULT), // todo: decide how to pass around colours - numbers (i32 or i64?) or strings? needs a new type or shares an integer type (needs generic monomorphisation)
        );
        imports.import(
            "runtime",
            "pen_changecolorparam",
            EntityType::Function(types::EXTERNREFF64I32_NORESULT),
        );
        imports.import(
            "runtime",
            "pen_setcolorparam",
            EntityType::Function(types::EXTERNREFF64I32_NORESULT),
        );
        imports.import(
            "runtime",
            "pen_changesize",
            EntityType::Function(types::F64I32_NORESULT),
        );
        imports.import(
            "runtime",
            "pen_changehue",
            EntityType::Function(types::F64I32_NORESULT),
        );
        imports.import(
            "runtime",
            "pen_sethue",
            EntityType::Function(types::F64I32_NORESULT),
        );

        functions.function(types::F64x2_F64);
        let mut fmod_func = Function::new(vec![]);
        // (a, b) => a - (truncate(a / b)) * b) + (b if a/b < 0 else 0)
        fmod_func.instruction(&Instruction::LocalGet(0)); // a
        fmod_func.instruction(&Instruction::LocalGet(0)); // a
        fmod_func.instruction(&Instruction::LocalGet(1)); // b
        fmod_func.instruction(&Instruction::F64Div); // a / b
        fmod_func.instruction(&Instruction::F64Trunc); // truncate(a / b)
        fmod_func.instruction(&Instruction::LocalGet(1)); // b
        fmod_func.instruction(&Instruction::F64Mul); // truncate(a / b) * b
        fmod_func.instruction(&Instruction::F64Sub); // a - (truncate(a / b) * b)
        fmod_func.instruction(&Instruction::LocalGet(1)); // b
        fmod_func.instruction(&Instruction::F64Const(0.0)); // 0
        fmod_func.instruction(&Instruction::LocalGet(0)); // a
        fmod_func.instruction(&Instruction::LocalGet(1)); // b
        fmod_func.instruction(&Instruction::F64Div); // a / b
        fmod_func.instruction(&Instruction::F64Const(0.0)); // 0
        fmod_func.instruction(&Instruction::F64Lt); // a / b < 0
        fmod_func.instruction(&Instruction::Select); // b if a / b < 0 else 0
        fmod_func.instruction(&Instruction::F64Add); // a - (truncate(a / b)) * b) + (b if a/b < 0 else 0)
        fmod_func.instruction(&Instruction::End);
        code.function(&fmod_func);

        functions.function(types::F64_I64);
        let mut float2bool_func = Function::new(vec![]);
        float2bool_func.instruction(&Instruction::LocalGet(0));
        float2bool_func.instruction(&Instruction::F64Abs);
        float2bool_func.instruction(&Instruction::F64Const(0.0));
        float2bool_func.instruction(&Instruction::F64Eq);
        float2bool_func.instruction(&Instruction::I64ExtendI32S);
        float2bool_func.instruction(&Instruction::End);
        code.function(&float2bool_func);

        functions.function(types::I64_F64);
        let mut bool2float_func = Function::new(vec![]);
        bool2float_func.instruction(&Instruction::LocalGet(0));
        bool2float_func.instruction(&Instruction::F64ConvertI64S);
        bool2float_func.instruction(&Instruction::End);
        code.function(&bool2float_func);

        functions.function(types::I64_EXTERNREF);
        let mut bool2string_func = Function::new(vec![]);
        bool2string_func.instruction(&Instruction::LocalGet(0));
        bool2string_func.instruction(&Instruction::I32WrapI64);
        bool2string_func.instruction(&Instruction::TableGet(table_indices::STRINGS));
        bool2string_func.instruction(&Instruction::End);
        code.function(&bool2string_func);

        functions.function(types::I32I64_EXTERNREF);
        let mut any2string_func = Function::new(vec![]);
        any2string_func.instruction(&Instruction::LocalGet(0));
        any2string_func.instruction(&Instruction::I32Const(hq_value_types::BOOL64));
        any2string_func.instruction(&Instruction::I32Eq);
        any2string_func.instruction(&Instruction::If(WasmBlockType::FunctionType(
            types::NOPARAM_EXTERNREF,
        )));
        any2string_func.instruction(&Instruction::LocalGet(1));
        any2string_func.instruction(&Instruction::Call(func_indices::CAST_BOOL_STRING));
        any2string_func.instruction(&Instruction::Else);
        any2string_func.instruction(&Instruction::LocalGet(0));
        any2string_func.instruction(&Instruction::I32Const(hq_value_types::FLOAT64));
        any2string_func.instruction(&Instruction::I32Eq);
        any2string_func.instruction(&Instruction::If(WasmBlockType::FunctionType(
            types::NOPARAM_EXTERNREF,
        )));
        any2string_func.instruction(&Instruction::LocalGet(1));
        any2string_func.instruction(&Instruction::F64ReinterpretI64);
        any2string_func.instruction(&Instruction::Call(
            func_indices::CAST_PRIMITIVE_FLOAT_STRING,
        ));
        any2string_func.instruction(&Instruction::Else);
        any2string_func.instruction(&Instruction::LocalGet(0));
        any2string_func.instruction(&Instruction::I32Const(hq_value_types::EXTERN_STRING_REF64));
        any2string_func.instruction(&Instruction::I32Eq);
        any2string_func.instruction(&Instruction::If(WasmBlockType::FunctionType(
            types::NOPARAM_EXTERNREF,
        )));
        any2string_func.instruction(&Instruction::LocalGet(1));
        any2string_func.instruction(&Instruction::I32WrapI64);
        any2string_func.instruction(&Instruction::TableGet(table_indices::STRINGS));
        any2string_func.instruction(&Instruction::Else);
        any2string_func.instruction(&Instruction::Unreachable);
        any2string_func.instruction(&Instruction::End);
        any2string_func.instruction(&Instruction::End);
        any2string_func.instruction(&Instruction::End);
        any2string_func.instruction(&Instruction::End);
        code.function(&any2string_func);

        functions.function(types::I32I64_F64);
        let mut any2float_func = Function::new(vec![]);
        any2float_func.instruction(&Instruction::LocalGet(0));
        any2float_func.instruction(&Instruction::I32Const(hq_value_types::BOOL64));
        any2float_func.instruction(&Instruction::I32Eq);
        any2float_func.instruction(&Instruction::If(WasmBlockType::FunctionType(
            types::NOPARAM_F64,
        )));
        any2float_func.instruction(&Instruction::LocalGet(1));
        any2float_func.instruction(&Instruction::Call(func_indices::CAST_BOOL_FLOAT));
        any2float_func.instruction(&Instruction::Else);
        any2float_func.instruction(&Instruction::LocalGet(0));
        any2float_func.instruction(&Instruction::I32Const(hq_value_types::EXTERN_STRING_REF64));
        any2float_func.instruction(&Instruction::I32Eq);
        any2float_func.instruction(&Instruction::If(WasmBlockType::FunctionType(
            types::NOPARAM_F64,
        )));
        any2float_func.instruction(&Instruction::LocalGet(1));
        any2float_func.instruction(&Instruction::I32WrapI64);
        any2float_func.instruction(&Instruction::TableGet(table_indices::STRINGS));
        any2float_func.instruction(&Instruction::Call(
            func_indices::CAST_PRIMITIVE_STRING_FLOAT,
        ));
        any2float_func.instruction(&Instruction::Else);
        any2float_func.instruction(&Instruction::LocalGet(0));
        any2float_func.instruction(&Instruction::I32Const(hq_value_types::FLOAT64));
        any2float_func.instruction(&Instruction::I32Eq);
        any2float_func.instruction(&Instruction::If(WasmBlockType::FunctionType(
            types::NOPARAM_F64,
        )));
        any2float_func.instruction(&Instruction::LocalGet(1));
        any2float_func.instruction(&Instruction::F64ReinterpretI64);
        any2float_func.instruction(&Instruction::Else);
        any2float_func.instruction(&Instruction::Unreachable);
        any2float_func.instruction(&Instruction::End);
        any2float_func.instruction(&Instruction::End);
        any2float_func.instruction(&Instruction::End);
        any2float_func.instruction(&Instruction::End);
        code.function(&any2float_func);

        functions.function(types::I32I64_I64);
        let mut any2bool_func = Function::new(vec![]);
        any2bool_func.instruction(&Instruction::LocalGet(0));
        any2bool_func.instruction(&Instruction::I32Const(hq_value_types::EXTERN_STRING_REF64));
        any2bool_func.instruction(&Instruction::I32Eq);
        any2bool_func.instruction(&Instruction::If(WasmBlockType::FunctionType(
            types::NOPARAM_I64,
        )));
        any2bool_func.instruction(&Instruction::LocalGet(1));
        any2bool_func.instruction(&Instruction::I32WrapI64);
        any2bool_func.instruction(&Instruction::TableGet(table_indices::STRINGS));
        any2bool_func.instruction(&Instruction::Call(func_indices::CAST_PRIMITIVE_STRING_BOOL));
        any2bool_func.instruction(&Instruction::I64ExtendI32S);
        any2bool_func.instruction(&Instruction::Else);
        any2bool_func.instruction(&Instruction::LocalGet(0));
        any2bool_func.instruction(&Instruction::I32Const(hq_value_types::FLOAT64));
        any2bool_func.instruction(&Instruction::I32Eq);
        any2bool_func.instruction(&Instruction::If(WasmBlockType::FunctionType(
            types::NOPARAM_I64,
        )));
        any2bool_func.instruction(&Instruction::LocalGet(1));
        any2bool_func.instruction(&Instruction::F64ReinterpretI64);
        any2bool_func.instruction(&Instruction::Call(func_indices::CAST_FLOAT_BOOL));
        any2bool_func.instruction(&Instruction::Else);
        any2bool_func.instruction(&Instruction::LocalGet(0));
        any2bool_func.instruction(&Instruction::I32Const(hq_value_types::BOOL64));
        any2bool_func.instruction(&Instruction::I32Eq);
        any2bool_func.instruction(&Instruction::If(WasmBlockType::FunctionType(
            types::NOPARAM_I64,
        )));
        any2bool_func.instruction(&Instruction::LocalGet(1));
        any2bool_func.instruction(&Instruction::Else);
        any2bool_func.instruction(&Instruction::Unreachable);
        any2bool_func.instruction(&Instruction::End);
        any2bool_func.instruction(&Instruction::End);
        any2bool_func.instruction(&Instruction::End);
        any2bool_func.instruction(&Instruction::End);
        code.function(&any2bool_func);

        functions.function(types::EXTERNREF_I32);
        let mut tbl_add_string_func = Function::new(vec![(1, ValType::I32)]);
        tbl_add_string_func.instruction(&Instruction::LocalGet(0));
        tbl_add_string_func.instruction(&Instruction::I32Const(1));
        tbl_add_string_func.instruction(&Instruction::TableGrow(table_indices::STRINGS));
        tbl_add_string_func.instruction(&Instruction::LocalTee(1));
        tbl_add_string_func.instruction(&Instruction::I32Const(-1));
        tbl_add_string_func.instruction(&Instruction::I32Eq);
        tbl_add_string_func.instruction(&Instruction::If(WasmBlockType::Empty));
        tbl_add_string_func.instruction(&Instruction::Unreachable);
        tbl_add_string_func.instruction(&Instruction::End);
        tbl_add_string_func.instruction(&Instruction::LocalGet(1));
        tbl_add_string_func.instruction(&Instruction::End);
        code.function(&tbl_add_string_func);

        mod supc_locals {
            pub const SPRITE_INDEX: u32 = 0;
            pub const MEM_POS: u32 = 1;
            pub const HUE: u32 = 2;
            pub const SAT: u32 = 3;
            pub const VAL: u32 = 4;
            pub const REGION: u32 = 5;
            pub const REMAINDER: u32 = 6;
            pub const P: u32 = 7;
            pub const Q: u32 = 8;
            pub const T: u32 = 9;
            pub const R: u32 = 10;
            pub const G: u32 = 11;
            pub const B: u32 = 12;
            pub const VAL_F: u32 = 13;
        }

        // hsv->rgb based off of https://stackoverflow.com/a/14733008
        functions.function(types::I32_NORESULT);
        let mut sprite_update_pen_color_func =
            Function::new(vec![(12, ValType::I32), (1, ValType::F32)]);
        sprite_update_pen_color_func.instruction(&Instruction::LocalGet(supc_locals::SPRITE_INDEX)); // sprite index - this is (target index - 1), assuming that the stage is target 0, which could be an issue if we don't confirm this
        sprite_update_pen_color_func.instruction(&Instruction::I32Const(SPRITE_INFO_LEN));
        sprite_update_pen_color_func.instruction(&Instruction::I32Mul);
        sprite_update_pen_color_func.instruction(&Instruction::I32Const(
            (byte_offset::VARS as usize + project.vars.borrow().len() * 12)
                .try_into()
                .unwrap(),
        ));
        sprite_update_pen_color_func.instruction(&Instruction::I32Add);
        sprite_update_pen_color_func.instruction(&Instruction::LocalTee(supc_locals::MEM_POS)); // position in memory of sprite info
        sprite_update_pen_color_func.instruction(&Instruction::Call(func_indices::DBG_LOGI32));
        sprite_update_pen_color_func.instruction(&Instruction::F32Load(MemArg {
            offset: u64::try_from(sprite_info_offsets::PEN_COLOR).unwrap(),
            align: 2,
            memory_index: 0,
        }));
        sprite_update_pen_color_func.instruction(&Instruction::F32Const(2.55));
        sprite_update_pen_color_func.instruction(&Instruction::F32Mul);
        sprite_update_pen_color_func.instruction(&Instruction::I32TruncF32S);
        sprite_update_pen_color_func.instruction(&Instruction::Call(func_indices::DBG_LOGI32));
        sprite_update_pen_color_func.instruction(&Instruction::LocalSet(supc_locals::HUE)); // hue
        sprite_update_pen_color_func.instruction(&Instruction::LocalGet(supc_locals::MEM_POS));
        sprite_update_pen_color_func.instruction(&Instruction::F32Load(MemArg {
            offset: u64::try_from(sprite_info_offsets::PEN_SATURATION).unwrap(),
            align: 2,
            memory_index: 0,
        }));
        sprite_update_pen_color_func.instruction(&Instruction::F32Const(2.55));
        sprite_update_pen_color_func.instruction(&Instruction::F32Mul);
        sprite_update_pen_color_func.instruction(&Instruction::I32TruncF32S);
        sprite_update_pen_color_func.instruction(&Instruction::Call(func_indices::DBG_LOGI32));
        sprite_update_pen_color_func.instruction(&Instruction::LocalSet(supc_locals::SAT)); // saturation ∈ [0, 256)
        sprite_update_pen_color_func.instruction(&Instruction::LocalGet(supc_locals::MEM_POS));
        sprite_update_pen_color_func.instruction(&Instruction::F32Load(MemArg {
            offset: u64::try_from(sprite_info_offsets::PEN_VALUE).unwrap(),
            align: 2,
            memory_index: 0,
        }));
        sprite_update_pen_color_func.instruction(&Instruction::F32Const(2.55));
        sprite_update_pen_color_func.instruction(&Instruction::F32Mul);
        sprite_update_pen_color_func.instruction(&Instruction::I32TruncF32S);
        sprite_update_pen_color_func.instruction(&Instruction::Call(func_indices::DBG_LOGI32));
        sprite_update_pen_color_func.instruction(&Instruction::LocalSet(supc_locals::VAL)); // value ∈ [0, 256)
        sprite_update_pen_color_func.instruction(&Instruction::LocalGet(supc_locals::MEM_POS));
        sprite_update_pen_color_func.instruction(&Instruction::F32Const(100.0));
        sprite_update_pen_color_func.instruction(&Instruction::LocalGet(supc_locals::MEM_POS));
        sprite_update_pen_color_func.instruction(&Instruction::F32Load(MemArg {
            offset: u64::try_from(sprite_info_offsets::PEN_TRANSPARENCY).unwrap(),
            align: 2,
            memory_index: 0,
        })); // transparency ∈ [0, 100]
        sprite_update_pen_color_func.instruction(&Instruction::F32Sub);
        sprite_update_pen_color_func.instruction(&Instruction::F32Const(100.0));
        sprite_update_pen_color_func.instruction(&Instruction::F32Div); // alpha ∈ [0, 1]
        sprite_update_pen_color_func.instruction(&Instruction::F32Store(MemArg {
            offset: u64::try_from(sprite_info_offsets::PEN_A).unwrap(),
            align: 0,
            memory_index: 0,
        }));
        sprite_update_pen_color_func.instruction(&Instruction::LocalGet(supc_locals::MEM_POS));
        sprite_update_pen_color_func.instruction(&Instruction::F32Load(MemArg {
            offset: u64::try_from(sprite_info_offsets::PEN_A).unwrap(),
            align: 0,
            memory_index: 0,
        }));
        sprite_update_pen_color_func.instruction(&Instruction::F32Const(0.01));
        sprite_update_pen_color_func.instruction(&Instruction::F32Lt);
        sprite_update_pen_color_func.instruction(&Instruction::If(WasmBlockType::Empty));
        sprite_update_pen_color_func.instruction(&Instruction::I32Const(
            supc_locals::MEM_POS.try_into().unwrap(),
        ));
        sprite_update_pen_color_func.instruction(&Instruction::F32Const(0.0));
        sprite_update_pen_color_func.instruction(&Instruction::F32Store(MemArg {
            offset: u64::try_from(sprite_info_offsets::PEN_A).unwrap(),
            align: 0,
            memory_index: 0,
        }));
        sprite_update_pen_color_func.instruction(&Instruction::Return); // if alpha is 0, return (it is already set to 0 so it doesn't matter what r, g & b are)
        sprite_update_pen_color_func.instruction(&Instruction::End);
        sprite_update_pen_color_func.instruction(&Instruction::LocalGet(supc_locals::SAT));
        sprite_update_pen_color_func.instruction(&Instruction::I32Eqz);
        sprite_update_pen_color_func.instruction(&Instruction::If(WasmBlockType::Empty));
        sprite_update_pen_color_func.instruction(&Instruction::LocalGet(supc_locals::VAL));
        sprite_update_pen_color_func.instruction(&Instruction::F32ConvertI32S);
        sprite_update_pen_color_func.instruction(&Instruction::F32Const(255.0));
        sprite_update_pen_color_func.instruction(&Instruction::F32Div);
        sprite_update_pen_color_func.instruction(&Instruction::LocalSet(supc_locals::VAL_F));
        sprite_update_pen_color_func.instruction(&Instruction::LocalGet(supc_locals::MEM_POS));
        sprite_update_pen_color_func.instruction(&Instruction::LocalGet(supc_locals::VAL_F));
        sprite_update_pen_color_func.instruction(&Instruction::F32Store(MemArg {
            offset: u64::try_from(sprite_info_offsets::PEN_R).unwrap(),
            align: 0,
            memory_index: 0,
        }));
        sprite_update_pen_color_func.instruction(&Instruction::LocalGet(supc_locals::MEM_POS));
        sprite_update_pen_color_func.instruction(&Instruction::LocalGet(supc_locals::VAL_F));
        sprite_update_pen_color_func.instruction(&Instruction::F32Store(MemArg {
            offset: u64::try_from(sprite_info_offsets::PEN_G).unwrap(),
            align: 0,
            memory_index: 0,
        }));
        sprite_update_pen_color_func.instruction(&Instruction::LocalGet(supc_locals::MEM_POS));
        sprite_update_pen_color_func.instruction(&Instruction::LocalGet(supc_locals::VAL_F));
        sprite_update_pen_color_func.instruction(&Instruction::F32Store(MemArg {
            offset: u64::try_from(sprite_info_offsets::PEN_B).unwrap(),
            align: 0,
            memory_index: 0,
        }));
        sprite_update_pen_color_func.instruction(&Instruction::Return);
        sprite_update_pen_color_func.instruction(&Instruction::End);
        sprite_update_pen_color_func.instruction(&Instruction::LocalGet(supc_locals::HUE));
        sprite_update_pen_color_func.instruction(&Instruction::I32Const(43));
        sprite_update_pen_color_func.instruction(&Instruction::I32DivU);
        sprite_update_pen_color_func.instruction(&Instruction::LocalSet(supc_locals::REGION)); // 'region'
        sprite_update_pen_color_func.instruction(&Instruction::LocalGet(supc_locals::HUE));
        sprite_update_pen_color_func.instruction(&Instruction::I32Const(43));
        sprite_update_pen_color_func.instruction(&Instruction::I32RemU);
        sprite_update_pen_color_func.instruction(&Instruction::I32Const(6));
        sprite_update_pen_color_func.instruction(&Instruction::I32Mul);
        sprite_update_pen_color_func.instruction(&Instruction::LocalSet(supc_locals::REMAINDER)); // 'remainder'
        sprite_update_pen_color_func.instruction(&Instruction::I32Const(255));
        sprite_update_pen_color_func.instruction(&Instruction::LocalGet(supc_locals::SAT));
        sprite_update_pen_color_func.instruction(&Instruction::I32Sub);
        sprite_update_pen_color_func.instruction(&Instruction::LocalGet(supc_locals::VAL));
        sprite_update_pen_color_func.instruction(&Instruction::I32Mul);
        sprite_update_pen_color_func.instruction(&Instruction::I32Const(8));
        sprite_update_pen_color_func.instruction(&Instruction::I32ShrU);
        sprite_update_pen_color_func.instruction(&Instruction::LocalSet(supc_locals::P)); // 'p'
        sprite_update_pen_color_func.instruction(&Instruction::I32Const(255));
        sprite_update_pen_color_func.instruction(&Instruction::LocalGet(supc_locals::REMAINDER));
        sprite_update_pen_color_func.instruction(&Instruction::LocalGet(supc_locals::SAT));
        sprite_update_pen_color_func.instruction(&Instruction::I32Mul);
        sprite_update_pen_color_func.instruction(&Instruction::I32Const(8));
        sprite_update_pen_color_func.instruction(&Instruction::I32ShrU);
        sprite_update_pen_color_func.instruction(&Instruction::I32Sub);
        sprite_update_pen_color_func.instruction(&Instruction::LocalGet(supc_locals::VAL));
        sprite_update_pen_color_func.instruction(&Instruction::I32Mul);
        sprite_update_pen_color_func.instruction(&Instruction::I32Const(8));
        sprite_update_pen_color_func.instruction(&Instruction::I32ShrU);
        sprite_update_pen_color_func.instruction(&Instruction::LocalSet(supc_locals::Q)); // 'q'
        sprite_update_pen_color_func.instruction(&Instruction::I32Const(255));
        sprite_update_pen_color_func.instruction(&Instruction::I32Const(255));
        sprite_update_pen_color_func.instruction(&Instruction::LocalGet(supc_locals::REMAINDER));
        sprite_update_pen_color_func.instruction(&Instruction::I32Sub);
        sprite_update_pen_color_func.instruction(&Instruction::LocalGet(supc_locals::SAT));
        sprite_update_pen_color_func.instruction(&Instruction::I32Mul);
        sprite_update_pen_color_func.instruction(&Instruction::I32Const(8));
        sprite_update_pen_color_func.instruction(&Instruction::I32ShrU);
        sprite_update_pen_color_func.instruction(&Instruction::I32Sub);
        sprite_update_pen_color_func.instruction(&Instruction::LocalGet(supc_locals::VAL));
        sprite_update_pen_color_func.instruction(&Instruction::I32Mul);
        sprite_update_pen_color_func.instruction(&Instruction::I32Const(8));
        sprite_update_pen_color_func.instruction(&Instruction::I32ShrU);
        sprite_update_pen_color_func.instruction(&Instruction::LocalSet(supc_locals::T)); // 't'
        sprite_update_pen_color_func.instruction(&Instruction::LocalGet(supc_locals::REGION));
        sprite_update_pen_color_func.instruction(&Instruction::I32Eqz);
        sprite_update_pen_color_func.instruction(&Instruction::If(WasmBlockType::Empty));
        sprite_update_pen_color_func.instruction(&Instruction::LocalGet(supc_locals::VAL));
        sprite_update_pen_color_func.instruction(&Instruction::LocalSet(supc_locals::R));
        sprite_update_pen_color_func.instruction(&Instruction::LocalGet(supc_locals::T));
        sprite_update_pen_color_func.instruction(&Instruction::LocalSet(supc_locals::G));
        sprite_update_pen_color_func.instruction(&Instruction::LocalGet(supc_locals::P));
        sprite_update_pen_color_func.instruction(&Instruction::LocalSet(supc_locals::B));
        sprite_update_pen_color_func.instruction(&Instruction::End);
        sprite_update_pen_color_func.instruction(&Instruction::LocalGet(supc_locals::REGION));
        sprite_update_pen_color_func.instruction(&Instruction::I32Const(1));
        sprite_update_pen_color_func.instruction(&Instruction::I32Eq);
        sprite_update_pen_color_func.instruction(&Instruction::If(WasmBlockType::Empty));
        sprite_update_pen_color_func.instruction(&Instruction::LocalGet(supc_locals::Q));
        sprite_update_pen_color_func.instruction(&Instruction::LocalSet(supc_locals::R));
        sprite_update_pen_color_func.instruction(&Instruction::LocalGet(supc_locals::VAL));
        sprite_update_pen_color_func.instruction(&Instruction::LocalSet(supc_locals::G));
        sprite_update_pen_color_func.instruction(&Instruction::LocalGet(supc_locals::P));
        sprite_update_pen_color_func.instruction(&Instruction::LocalSet(supc_locals::B));
        sprite_update_pen_color_func.instruction(&Instruction::End);
        sprite_update_pen_color_func.instruction(&Instruction::LocalGet(supc_locals::REGION));
        sprite_update_pen_color_func.instruction(&Instruction::I32Const(2));
        sprite_update_pen_color_func.instruction(&Instruction::I32Eq);
        sprite_update_pen_color_func.instruction(&Instruction::If(WasmBlockType::Empty));
        sprite_update_pen_color_func.instruction(&Instruction::LocalGet(supc_locals::P));
        sprite_update_pen_color_func.instruction(&Instruction::LocalSet(supc_locals::R));
        sprite_update_pen_color_func.instruction(&Instruction::LocalGet(supc_locals::VAL));
        sprite_update_pen_color_func.instruction(&Instruction::LocalSet(supc_locals::G));
        sprite_update_pen_color_func.instruction(&Instruction::LocalGet(supc_locals::T));
        sprite_update_pen_color_func.instruction(&Instruction::LocalSet(supc_locals::B));
        sprite_update_pen_color_func.instruction(&Instruction::End);
        sprite_update_pen_color_func.instruction(&Instruction::LocalGet(supc_locals::REGION));
        sprite_update_pen_color_func.instruction(&Instruction::I32Const(3));
        sprite_update_pen_color_func.instruction(&Instruction::I32Eq);
        sprite_update_pen_color_func.instruction(&Instruction::If(WasmBlockType::Empty));
        sprite_update_pen_color_func.instruction(&Instruction::LocalGet(supc_locals::P));
        sprite_update_pen_color_func.instruction(&Instruction::LocalSet(supc_locals::R));
        sprite_update_pen_color_func.instruction(&Instruction::LocalGet(supc_locals::Q));
        sprite_update_pen_color_func.instruction(&Instruction::LocalSet(supc_locals::G));
        sprite_update_pen_color_func.instruction(&Instruction::LocalGet(supc_locals::VAL));
        sprite_update_pen_color_func.instruction(&Instruction::LocalSet(supc_locals::B));
        sprite_update_pen_color_func.instruction(&Instruction::End);
        sprite_update_pen_color_func.instruction(&Instruction::LocalGet(supc_locals::REGION));
        sprite_update_pen_color_func.instruction(&Instruction::I32Const(4));
        sprite_update_pen_color_func.instruction(&Instruction::I32Eq);
        sprite_update_pen_color_func.instruction(&Instruction::If(WasmBlockType::Empty));
        sprite_update_pen_color_func.instruction(&Instruction::LocalGet(supc_locals::T));
        sprite_update_pen_color_func.instruction(&Instruction::LocalSet(supc_locals::R));
        sprite_update_pen_color_func.instruction(&Instruction::LocalGet(supc_locals::P));
        sprite_update_pen_color_func.instruction(&Instruction::LocalSet(supc_locals::G));
        sprite_update_pen_color_func.instruction(&Instruction::LocalGet(supc_locals::VAL));
        sprite_update_pen_color_func.instruction(&Instruction::LocalSet(supc_locals::B));
        sprite_update_pen_color_func.instruction(&Instruction::End);
        sprite_update_pen_color_func.instruction(&Instruction::LocalGet(supc_locals::REGION));
        sprite_update_pen_color_func.instruction(&Instruction::I32Const(5));
        sprite_update_pen_color_func.instruction(&Instruction::I32Eq);
        sprite_update_pen_color_func.instruction(&Instruction::If(WasmBlockType::Empty));
        sprite_update_pen_color_func.instruction(&Instruction::LocalGet(supc_locals::VAL));
        sprite_update_pen_color_func.instruction(&Instruction::LocalSet(supc_locals::R));
        sprite_update_pen_color_func.instruction(&Instruction::LocalGet(supc_locals::P));
        sprite_update_pen_color_func.instruction(&Instruction::LocalSet(supc_locals::G));
        sprite_update_pen_color_func.instruction(&Instruction::LocalGet(supc_locals::Q));
        sprite_update_pen_color_func.instruction(&Instruction::LocalSet(supc_locals::B));
        sprite_update_pen_color_func.instruction(&Instruction::End);
        sprite_update_pen_color_func.instruction(&Instruction::LocalGet(supc_locals::MEM_POS));
        sprite_update_pen_color_func.instruction(&Instruction::LocalGet(supc_locals::R));
        sprite_update_pen_color_func.instruction(&Instruction::Call(func_indices::DBG_LOGI32));
        sprite_update_pen_color_func.instruction(&Instruction::F32ConvertI32S);
        sprite_update_pen_color_func.instruction(&Instruction::F32Const(255.0));
        sprite_update_pen_color_func.instruction(&Instruction::F32Div);
        sprite_update_pen_color_func.instruction(&Instruction::F32Store(MemArg {
            offset: u64::try_from(sprite_info_offsets::PEN_R).unwrap(),
            align: 0,
            memory_index: 0,
        }));
        sprite_update_pen_color_func.instruction(&Instruction::LocalGet(supc_locals::MEM_POS));
        sprite_update_pen_color_func.instruction(&Instruction::LocalGet(supc_locals::G));
        sprite_update_pen_color_func.instruction(&Instruction::Call(func_indices::DBG_LOGI32));
        sprite_update_pen_color_func.instruction(&Instruction::F32ConvertI32S);
        sprite_update_pen_color_func.instruction(&Instruction::F32Const(255.0));
        sprite_update_pen_color_func.instruction(&Instruction::F32Div);
        sprite_update_pen_color_func.instruction(&Instruction::F32Store(MemArg {
            offset: u64::try_from(sprite_info_offsets::PEN_G).unwrap(),
            align: 0,
            memory_index: 0,
        }));
        sprite_update_pen_color_func.instruction(&Instruction::LocalGet(supc_locals::MEM_POS));
        sprite_update_pen_color_func.instruction(&Instruction::LocalGet(supc_locals::B));
        sprite_update_pen_color_func.instruction(&Instruction::Call(func_indices::DBG_LOGI32));
        sprite_update_pen_color_func.instruction(&Instruction::F32ConvertI32S);
        sprite_update_pen_color_func.instruction(&Instruction::F32Const(255.0));
        sprite_update_pen_color_func.instruction(&Instruction::F32Div);
        sprite_update_pen_color_func.instruction(&Instruction::F32Store(MemArg {
            offset: u64::try_from(sprite_info_offsets::PEN_B).unwrap(),
            align: 0,
            memory_index: 0,
        }));
        sprite_update_pen_color_func.instruction(&Instruction::End);
        code.function(&sprite_update_pen_color_func);

        let mut gf_func = Function::new(vec![]);
        let mut tick_func = Function::new(vec![(2, ValType::I32)]);

        let mut noop_func = Function::new(vec![]);
        noop_func.instruction(&Instruction::I32Const(1));
        noop_func.instruction(&Instruction::End);

        let mut thread_indices: Vec<(ThreadStart, u32)> = vec![]; // (start type, first step index)

        let mut string_consts = vec![String::from("false"), String::from("true")];

        let mut step_funcs: IndexMap<Option<(String, String)>, Function, _> = Default::default();
        step_funcs.insert(None, noop_func);

        for step in &project.steps {
            // make sure to skip the 0th (noop) step because we've added the noop step function 3 lines above
            if step.0 == &("".into(), "".into()) {
                continue;
            };
            step.compile_wasm(&mut step_funcs, &mut string_consts, &project.steps);
        }

        for thread in project.threads {
            let first_idx = project
                .steps
                .get_index_of(&(thread.target_id().clone(), thread.first_step().clone()))
                .unwrap()
                .try_into()
                .expect("step index out of bounds");
            thread_indices.push((thread.start().clone(), first_idx));
        }

        let mut thread_start_counts: BTreeMap<ThreadStart, u32> = Default::default();

        macro_rules! func_for_thread_start {
            ($start_type:ident) => {
                match $start_type {
                    ThreadStart::GreenFlag => &mut gf_func,
                }
            };
        }

        for (_step, func) in &step_funcs {
            functions.function(types::I32_I32);
            code.function(func);
        }

        for (start_type, index) in thread_indices {
            let func = func_for_thread_start!(start_type);
            func.instruction(&Instruction::I32Const(0));
            func.instruction(&Instruction::I32Load(MemArg {
                offset: byte_offset::THREAD_NUM
                    .try_into()
                    .expect("THREAD_NUM out of bounds (E015)"),
                align: 2,
                memory_index: 0,
            }));
            func.instruction(&Instruction::I32Const(THREAD_BYTE_LEN));
            func.instruction(&Instruction::I32Mul);
            let thread_start_count: i32 = (*thread_start_counts.get(&start_type).unwrap_or(&0))
                .try_into()
                .expect("start_type count out of bounds (E017)");
            func.instruction(&Instruction::I32Const(thread_start_count * THREAD_BYTE_LEN));
            func.instruction(&Instruction::I32Add);
            func.instruction(&Instruction::I32Const(
                index
                    .try_into()
                    .expect("step func index out of bounds (E006)"),
            ));
            func.instruction(&Instruction::I32Store(MemArg {
                offset: (byte_offset::VARS as usize
                    + 12 * project.vars.borrow().len()
                    + usize::try_from(SPRITE_INFO_LEN).unwrap() * (project.targets.len() - 1))
                    .try_into()
                    .expect("i32.store offset out of bounds"),
                align: 2, // 2 ** 2 = 4 (bytes)
                memory_index: 0,
            }));

            thread_start_counts
                .entry(start_type)
                .and_modify(|u| *u += 1)
                .or_insert(1);
        }

        for (start_type, count) in thread_start_counts {
            let func = func_for_thread_start!(start_type);
            func.instruction(&Instruction::I32Const(0));
            func.instruction(&Instruction::I32Const(0));
            func.instruction(&Instruction::I32Load(MemArg {
                offset: byte_offset::THREAD_NUM
                    .try_into()
                    .expect("THREAD_NUM out of bounds (E012)"),
                align: 2,
                memory_index: 0,
            }));
            func.instruction(&Instruction::I32Const(
                count
                    .try_into()
                    .expect("thread_start count out of bounds (E014)"),
            ));
            func.instruction(&Instruction::I32Add);
            func.instruction(&Instruction::I32Store(MemArg {
                offset: byte_offset::THREAD_NUM
                    .try_into()
                    .expect("THREAD_NUM out of bounds (E016)"),
                align: 2,
                memory_index: 0,
            }));
        }

        {
            tick_func.instruction(&Instruction::I32Const(0));
            tick_func.instruction(&Instruction::I32Load(MemArg {
                offset: byte_offset::THREAD_NUM
                    .try_into()
                    .expect("THREAD_NUM out of bounds (E013)"),
                align: 2,
                memory_index: 0,
            }));
            tick_func.instruction(&Instruction::LocalTee(1));
            tick_func.instruction(&Instruction::I32Eqz);
            tick_func.instruction(&Instruction::BrIf(0));
            tick_func.instruction(&Instruction::LocalGet(1));
            tick_func.instruction(&Instruction::I32Const(THREAD_BYTE_LEN));
            tick_func.instruction(&Instruction::I32Mul);
            tick_func.instruction(&Instruction::I32Const(THREAD_BYTE_LEN));
            tick_func.instruction(&Instruction::I32Sub);
            tick_func.instruction(&Instruction::LocalSet(1));
            tick_func.instruction(&Instruction::Loop(WasmBlockType::Empty));

            tick_func.instruction(&Instruction::LocalGet(0));
            tick_func.instruction(&Instruction::LocalGet(0));
            tick_func.instruction(&Instruction::I32Load(MemArg {
                offset: (byte_offset::VARS as usize
                    + 12 * project.vars.borrow().len()
                    + usize::try_from(SPRITE_INFO_LEN).unwrap() * (project.targets.len() - 1))
                    .try_into()
                    .expect("i32.store offset out of bounds"),
                align: 2, // 2 ** 2 = 4 (bytes)
                memory_index: 0,
            }));
            tick_func.instruction(&Instruction::CallIndirect {
                ty: types::I32_I32,
                table: table_indices::STEP_FUNCS,
            });

            tick_func.instruction(&Instruction::If(WasmBlockType::Empty));
            tick_func.instruction(&Instruction::LocalGet(0));
            tick_func.instruction(&Instruction::I32Const(THREAD_BYTE_LEN));
            tick_func.instruction(&Instruction::I32Add);
            tick_func.instruction(&Instruction::LocalSet(0));
            tick_func.instruction(&Instruction::Else);
            tick_func.instruction(&Instruction::LocalGet(1));
            tick_func.instruction(&Instruction::I32Const(THREAD_BYTE_LEN));
            tick_func.instruction(&Instruction::I32Sub);
            tick_func.instruction(&Instruction::LocalSet(1));
            tick_func.instruction(&Instruction::End);
            tick_func.instruction(&Instruction::LocalGet(0));
            tick_func.instruction(&Instruction::LocalGet(1));
            tick_func.instruction(&Instruction::I32LeS);
            tick_func.instruction(&Instruction::BrIf(0));
            tick_func.instruction(&Instruction::End);
        }

        gf_func.instruction(&Instruction::End);
        functions.function(types::NOPARAM_NORESULT);
        code.function(&gf_func);
        exports.export(
            "green_flag",
            ExportKind::Func,
            code.len() + IMPORTED_FUNCS - 1,
        );

        tick_func.instruction(&Instruction::End);
        functions.function(types::NOPARAM_NORESULT);
        code.function(&tick_func);
        exports.export("tick", ExportKind::Func, code.len() + IMPORTED_FUNCS - 1);

        tables.table(TableType {
            element_type: RefType::FUNCREF,
            minimum: step_funcs
                .len()
                .try_into()
                .expect("step_funcs length out of bounds (E007)"),
            maximum: Some(
                step_funcs
                    .len()
                    .try_into()
                    .expect("step_funcs length out of bounds (E008)"),
            ),
        });

        tables.table(TableType {
            element_type: RefType::EXTERNREF,
            minimum: string_consts
                .len()
                .try_into()
                .expect("string_consts len out of bounds (E037)"),
            maximum: None,
        });

        let step_indices = (BUILTIN_FUNCS
            ..(u32::try_from(step_funcs.len()).expect("step_funcs length out of bounds")
                + BUILTIN_FUNCS))
            .collect::<Vec<_>>();
        let step_func_indices = Elements::Functions(&step_indices[..]);
        elements.active(
            Some(table_indices::STEP_FUNCS),
            &ConstExpr::i32_const(0),
            RefType::FUNCREF,
            step_func_indices,
        );

        globals.global(
            GlobalType {
                val_type: ValType::I32,
                mutable: false,
            },
            &ConstExpr::i32_const(byte_offset::REDRAW_REQUESTED),
        );
        globals.global(
            GlobalType {
                val_type: ValType::I32,
                mutable: false,
            },
            &ConstExpr::i32_const(byte_offset::THREAD_NUM),
        );
        globals.global(
            GlobalType {
                val_type: ValType::I32,
                mutable: false,
            },
            &ConstExpr::i32_const(
                project
                    .vars
                    .borrow()
                    .len()
                    .try_into()
                    .expect("vars length out of bounds"),
            ),
        );

        exports.export("step_funcs", ExportKind::Table, table_indices::STEP_FUNCS);
        exports.export("strings", ExportKind::Table, table_indices::STRINGS);
        exports.export("memory", ExportKind::Memory, 0);
        exports.export("rr_offset", ExportKind::Global, 0);
        exports.export("thn_offset", ExportKind::Global, 1);
        exports.export("vars_num", ExportKind::Global, 2);
        exports.export(
            "upc",
            ExportKind::Func,
            func_indices::SPRITE_UPDATE_PEN_COLOR,
        );

        module
            .section(&types)
            .section(&imports)
            .section(&functions)
            .section(&tables)
            .section(&memories)
            .section(&globals)
            .section(&exports)
            // start
            .section(&elements)
            // datacount
            .section(&code);
        // data

        let wasm_bytes = module.finish();
        Self {
            target_names: project.targets.clone(),
            wasm_bytes,
            string_consts,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::process::{Command, Stdio};

    /*#[test]
    fn run_wasm() {
        use crate::sb3::Sb3Project;
        use std::fs;
        let proj: Sb3Project = fs::read_to_string("./hq-test.project.json")
            .expect("couldn't read hq-test.project.json")
            .try_into()
            .unwrap();
        let ir: IrProject = proj.into();
        let wasm: WasmProject = ir.into();
        fs::write("./bad.wasm", wasm.wasm_bytes()).expect("failed to write to bad.wasm");
        let output = Command::new("node")
            .arg("-e")
            .arg(format!(
                "({})().catch(e => {{ console.error(e); process.exit(1) }})",
                wasm.js_string()
            ))
            .stdout(Stdio::inherit())
            .output()
            .expect("failed to execute process");
        println!(
            "{:}",
            String::from_utf8(output.stdout).expect("failed to convert stdout from utf8")
        );
        println!(
            "{:}",
            String::from_utf8(output.stderr).expect("failed to convert stderr from utf8")
        );
        if !output.status.success() {
            panic!("couldn't run wasm");
        }
    }*/
}
