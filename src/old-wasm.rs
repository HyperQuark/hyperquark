use crate::ir::{
    InputType, IrBlock, IrOpcode, IrProject, IrVal, Procedure, Step, ThreadContext, ThreadStart,
    TypeStackImpl,
};
use crate::sb3::VarVal;
use crate::HQError;
use alloc::collections::BTreeMap;
use alloc::rc::Rc;
use alloc::string::String;
use alloc::vec::Vec;
use core::hash::BuildHasherDefault;
use hashers::fnv::FNV1aHasher64;
use indexmap::IndexMap;
use wasm_bindgen::prelude::*;
use wasm_encoder::{
    AbstractHeapType, BlockType as WasmBlockType, CodeSection, ConstExpr, DataSection,
    ElementSection, Elements, EntityType, ExportKind, ExportSection, Function, FunctionSection,
    GlobalSection, GlobalType, HeapType, ImportSection, Instruction, MemArg, MemorySection,
    MemoryType, Module, RefType, TableSection, TableType, TypeSection, ValType,
};

fn instructions(
    op: &IrBlock,
    context: Rc<ThreadContext>,
    string_consts: &mut Vec<String>,
    steps: &IndexMap<(String, String), Step, BuildHasherDefault<FNV1aHasher64>>,
    input_types: Vec<InputType>,
) -> Result<Vec<Instruction<'static>>, HQError> {
    use InputType::*;
    use Instruction::*;
    use IrOpcode::*;
    let locals_shift = match context.proc {
        None => 0,
        Some(Procedure {
            warp,
            ref arg_types,
            ..
        }) => {
            if !warp {
                hq_todo!("non-warp procedure")
            } else {
                i32::try_from(arg_types.len())
                    .map_err(|_| make_hq_bug!("arg types len out of bounds"))?
            }
        }
    };
    macro_rules! local {
        ($id:ident) => {{
            u32::try_from(
                i32::try_from(step_func_locals::$id).map_err(|_| make_hq_bug!(""))? + locals_shift,
            )
            .map_err(|_| {
                make_hq_bug!(
                    "shifted local from {:} out of bounds",
                    step_func_locals::$id
                )
            })?
        }};
    }
    //let expected_output = *op.expected_output();
    //let mut actual_output = *op.actual_output();
    //dbg!(&op.opcode(), op.type_stack.len());
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
                            .map_err(|_| make_hq_bug!("target index out of bounds"))?,
                    ),
                    Call(func_indices::LOOKS_THINK),
                    I32Const(0),
                    I32Const(0),
                    I32Store8(MemArg {
                        offset: 0,
                        align: 0,
                        memory_index: 0,
                    }),
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
                            .map_err(|_| make_hq_bug!("target index out of bounds"))?,
                    ),
                    Call(func_indices::LOOKS_SAY),
                    I32Const(0),
                    I32Const(0),
                    I32Store8(MemArg {
                        offset: 0,
                        align: 0,
                        memory_index: 0,
                    }),
                ]
            }
        }
        unknown_const { val } => match val {
            IrVal::Unknown(_) => hq_bug!("unknown const inside of unknown const"),
            IrVal::Int(i) => vec![I32Const(hq_value_types::INT64), I64Const(*i as i64)],
            IrVal::Float(f) => vec![
                I32Const(hq_value_types::FLOAT64),
                I64Const(i64::from_le_bytes(f.to_le_bytes())),
            ],
            IrVal::Boolean(b) => vec![I32Const(hq_value_types::BOOL64), I64Const(*b as i64)],
            IrVal::String(s) => vec![
                I32Const(hq_value_types::EXTERN_STRING_REF64),
                I64Const(
                    {
                        if let Some(idx) = string_consts.iter().position(|string| string == s) {
                            idx
                        } else {
                            string_consts.push(s.clone());
                            string_consts.len() - 1
                        }
                    }
                    .try_into()
                    .map_err(|_| make_hq_bug!("string index out of bounds"))?,
                ),
            ],
        },
        operator_add => {
            if InputType::Integer.includes(input_types.first().unwrap())
                && InputType::Integer.includes(input_types.get(1).unwrap())
            {
                vec![I64Add]
            } else if InputType::Integer.includes(input_types.get(1).unwrap()) {
                vec![F64ConvertI64S, F64Add]
            } else if InputType::Integer.includes(input_types.first().unwrap()) {
                vec![
                    LocalSet(local!(F64)),
                    F64ConvertI64S,
                    LocalGet(local!(F64)),
                    F64Add,
                ]
            } else {
                vec![F64Add]
            }
        }
        operator_subtract => {
            if InputType::Integer.includes(input_types.first().unwrap())
                && InputType::Integer.includes(input_types.get(1).unwrap())
            {
                vec![I64Sub]
            } else if InputType::Integer.includes(input_types.get(1).unwrap()) {
                vec![F64ConvertI64S, F64Sub]
            } else if InputType::Integer.includes(input_types.first().unwrap()) {
                vec![
                    LocalSet(local!(F64)),
                    F64ConvertI64S,
                    LocalGet(local!(F64)),
                    F64Sub,
                ]
            } else {
                vec![F64Sub]
            }
        }
        operator_divide => vec![F64Div],
        operator_multiply => {
            if InputType::Integer.includes(input_types.first().unwrap())
                && InputType::Integer.includes(input_types.get(1).unwrap())
            {
                vec![I64Mul]
            } else if InputType::Integer.includes(input_types.get(1).unwrap()) {
                vec![F64ConvertI64S, F64Mul]
            } else if InputType::Integer.includes(input_types.first().unwrap()) {
                vec![
                    LocalSet(local!(F64)),
                    F64ConvertI64S,
                    LocalGet(local!(F64)),
                    F64Mul,
                ]
            } else {
                vec![F64Mul]
            }
        }
        operator_mod => {
            if InputType::Integer.includes(input_types.first().unwrap())
                && InputType::Integer.includes(input_types.get(1).unwrap())
            {
                vec![I64RemS]
            } else if InputType::Integer.includes(input_types.get(1).unwrap()) {
                vec![F64ConvertI64S, Call(func_indices::FMOD)]
            } else if InputType::Integer.includes(input_types.first().unwrap()) {
                vec![
                    LocalSet(local!(F64)),
                    F64ConvertI64S,
                    LocalGet(local!(F64)),
                    Call(func_indices::FMOD),
                ]
            } else {
                vec![Call(func_indices::FMOD)]
            }
        }
        operator_round => {
            if InputType::Integer.includes(input_types.first().unwrap()) {
                vec![]
            } else {
                vec![F64Nearest, I64TruncF64S]
            }
        }
        math_number { NUM } => {
            vec![F64Const(*NUM)]
        }
        math_integer { NUM }
        | math_angle { NUM }
        | math_whole_number { NUM }
        | math_positive_number { NUM } => {
            vec![I64Const(*NUM as i64)]
        }
        boolean { BOOL } => vec![I32Const(*BOOL as i32)],
        text { TEXT } => {
            let str_idx: i32 = {
                if let Some(idx) = string_consts.iter().position(|string| string == TEXT) {
                    idx
                } else {
                    string_consts.push(TEXT.clone());
                    string_consts.len() - 1
                }
            }
            .try_into()
            .map_err(|_| make_hq_bug!("string index out of bounds"))?;
            vec![I32Const(str_idx), TableGet(table_indices::STRINGS)]
        }
        data_variable {
            VARIABLE,
            assume_type,
        } => {
            let var_index = context
                .vars
                .borrow()
                .iter()
                .position(|var| VARIABLE == var.id())
                .ok_or(make_hq_bug!("couldn't find variable index"))?;
            let var_offset: u64 = (usize::try_from(byte_offset::VARS)
                .map_err(|_| make_hq_bug!("variable offset out of bounds"))?
                + VAR_INFO_LEN as usize * var_index)
                .try_into()
                .map_err(|_| make_hq_bug!("variable offset out of bounds"))?;
            if assume_type.is_none() {
                vec![
                    I32Const(0),
                    I32Load(MemArg {
                        offset: var_offset,
                        align: 2,
                        memory_index: 0,
                    }),
                    I32Const(0),
                    I64Load(MemArg {
                        offset: var_offset + 8,
                        align: 3,
                        memory_index: 0,
                    }),
                ]
            } else {
                match assume_type
                    .as_ref()
                    .unwrap()
                    .least_restrictive_concrete_type()
                {
                    InputType::Float => vec![
                        I32Const(0),
                        F64Load(MemArg {
                            offset: var_offset + 8,
                            align: 3,
                            memory_index: 0,
                        }),
                    ],
                    InputType::ConcreteInteger => vec![
                        I32Const(0),
                        I64Load(MemArg {
                            offset: var_offset + 8,
                            align: 3,
                            memory_index: 0,
                        }),
                    ],
                    InputType::Boolean => vec![
                        I32Const(0),
                        I32Load(MemArg {
                            offset: var_offset + 8,
                            align: 2,
                            memory_index: 0,
                        }), // here we can load an i32 directly because a boolean will never be signed
                    ],
                    InputType::String => vec![GlobalGet(
                        BUILTIN_GLOBALS
                            + TryInto::<u32>::try_into(var_index)
                                .map_err(|_| make_hq_bug!("var index out of bounds"))?,
                    )],
                    other => hq_bug!("unexpected concrete type {:?}", other),
                }
            }
        }
        data_setvariableto {
            VARIABLE,
            assume_type,
        } => {
            let var_index = context
                .vars
                .borrow()
                .iter()
                .position(|var| VARIABLE == var.id())
                .ok_or(make_hq_bug!("couldn't find variable index"))?;
            let var_offset: u64 = (usize::try_from(byte_offset::VARS)
                .map_err(|_| make_hq_bug!("variable offset out of bounds"))?
                + VAR_INFO_LEN as usize * var_index)
                .try_into()
                .map_err(|_| make_hq_bug!("variable offset out of bounds"))?;
            if assume_type.is_none() {
                vec![
                    LocalSet(local!(I64)),
                    LocalSet(local!(I32)),
                    I32Const(0),
                    LocalGet(local!(I32)),
                    I32Store(MemArg {
                        offset: var_offset,
                        align: 2,
                        memory_index: 0,
                    }),
                    I32Const(0),
                    LocalGet(local!(I64)),
                    I64Store(MemArg {
                        offset: var_offset + 8,
                        align: 3,
                        memory_index: 0,
                    }),
                ]
            } else {
                match assume_type
                    .as_ref()
                    .unwrap()
                    .least_restrictive_concrete_type()
                {
                    InputType::Float => vec![
                        LocalSet(local!(F64)),
                        I32Const(0),
                        LocalGet(local!(F64)),
                        F64Store(MemArg {
                            offset: var_offset + 8,
                            align: 3,
                            memory_index: 0,
                        }),
                    ],
                    InputType::Boolean => vec![
                        LocalSet(local!(I32)),
                        I32Const(0),
                        LocalGet(local!(I32)),
                        I32Store(MemArg {
                            offset: var_offset + 8,
                            align: 2,
                            memory_index: 0,
                        }),
                    ],
                    InputType::ConcreteInteger => vec![
                        LocalSet(local!(I64)),
                        I32Const(0),
                        LocalGet(local!(I64)),
                        I64Store(MemArg {
                            offset: var_offset + 8,
                            align: 3,
                            memory_index: 0,
                        }),
                    ],
                    InputType::String => vec![GlobalSet(
                        BUILTIN_GLOBALS
                            + TryInto::<u32>::try_into(var_index)
                                .map_err(|_| make_hq_bug!("var index out of bounds"))?,
                    )],
                    other => hq_bug!("unexpected concrete type {:?}", other),
                }
            }
        }
        data_teevariable {
            VARIABLE,
            assume_type,
        } => {
            let var_index = context
                .vars
                .borrow()
                .iter()
                .position(|var| VARIABLE == var.id())
                .ok_or(make_hq_bug!("couldn't find variable index"))?;
            let var_offset: u64 = (usize::try_from(byte_offset::VARS)
                .map_err(|_| make_hq_bug!("variable offset out of bounds"))?
                + VAR_INFO_LEN as usize * var_index)
                .try_into()
                .map_err(|_| make_hq_bug!("variable offset out of bounds"))?;
            if assume_type.is_none() {
                vec![
                    LocalSet(local!(I64)),
                    LocalSet(local!(I32)),
                    I32Const(0),
                    LocalGet(local!(I32)),
                    I32Store(MemArg {
                        offset: var_offset,
                        align: 2,
                        memory_index: 0,
                    }),
                    I32Const(0),
                    LocalGet(local!(I64)),
                    I64Store(MemArg {
                        offset: var_offset + 8,
                        align: 3,
                        memory_index: 0,
                    }),
                    LocalGet(local!(I32)),
                    LocalGet(local!(I64)),
                ]
            } else {
                match assume_type
                    .as_ref()
                    .unwrap()
                    .least_restrictive_concrete_type()
                {
                    InputType::Float => vec![
                        LocalTee(local!(F64)),
                        I32Const(0),
                        LocalGet(local!(F64)),
                        F64Store(MemArg {
                            offset: var_offset + 8,
                            align: 3,
                            memory_index: 0,
                        }),
                    ],
                    InputType::Boolean => vec![
                        LocalTee(local!(I32)),
                        I32Const(0),
                        LocalGet(local!(I32)),
                        I32Store(MemArg {
                            offset: var_offset + 8,
                            align: 2,
                            memory_index: 0,
                        }),
                    ],
                    InputType::ConcreteInteger => vec![
                        LocalTee(local!(I64)),
                        I32Const(0),
                        LocalGet(local!(I64)),
                        I64Store(MemArg {
                            offset: var_offset + 8,
                            align: 3,
                            memory_index: 0,
                        }),
                    ],
                    InputType::String => vec![
                        LocalTee(local!(EXTERNREF)),
                        GlobalSet(
                            BUILTIN_GLOBALS
                                + TryInto::<u32>::try_into(var_index)
                                    .map_err(|_| make_hq_bug!("var index out of bounds"))?,
                        ),
                        LocalGet(local!(EXTERNREF)),
                    ],
                    other => hq_bug!("unexpected concrete type {:?}", other),
                }
            }
        }
        operator_lt => {
            if InputType::Integer.includes(input_types.first().unwrap())
                && InputType::Integer.includes(input_types.get(1).unwrap())
            {
                vec![I64LtS]
            } else if InputType::Integer.includes(input_types.get(1).unwrap()) {
                vec![F64ConvertI64S, F64Lt]
            } else if InputType::Integer.includes(input_types.first().unwrap()) {
                vec![
                    LocalSet(local!(F64)),
                    F64ConvertI64S,
                    LocalGet(local!(F64)),
                    F64Lt,
                ]
            } else {
                vec![F64Lt]
            }
        }
        operator_gt => {
            if InputType::Integer.includes(input_types.first().unwrap())
                && InputType::Integer.includes(input_types.get(1).unwrap())
            {
                vec![I64GtS]
            } else if InputType::Integer.includes(input_types.get(1).unwrap()) {
                vec![F64ConvertI64S, F64Gt]
            } else if InputType::Integer.includes(input_types.first().unwrap()) {
                vec![
                    LocalSet(local!(F64)),
                    F64ConvertI64S,
                    LocalGet(local!(F64)),
                    F64Gt,
                ]
            } else {
                vec![F64Gt]
            }
        }
        operator_and => vec![I32And],
        operator_or => vec![I32Or],
        operator_not => vec![I32Eqz],
        operator_equals => match (
            input_types.first().unwrap().loosen_to([
                InputType::Integer,
                InputType::Float,
                InputType::String,
                InputType::Unknown,
            ])?,
            input_types.get(1).unwrap().loosen_to([
                InputType::Integer,
                InputType::Float,
                InputType::String,
                InputType::Unknown,
            ])?,
        ) {
            (InputType::Integer, InputType::Integer) => vec![I64Eq],
            (InputType::Float, InputType::Float) => vec![F64Eq],
            (InputType::String, InputType::String) => vec![Call(func_indices::STRING_EQUALS)],
            (InputType::Float, InputType::Integer) => vec![F64ConvertI64S, F64Eq],
            (InputType::Integer, InputType::Float) => vec![
                LocalSet(local!(F64)),
                F64ConvertI64S,
                LocalGet(local!(F64)),
                F64Eq,
            ],
            (InputType::String, InputType::Unknown) => vec![
                Call(func_indices::CAST_ANY_STRING),
                Call(func_indices::STRING_EQUALS),
            ],
            (InputType::Unknown, InputType::String) => vec![
                LocalSet(local!(EXTERNREF)),
                Call(func_indices::CAST_ANY_STRING),
                LocalGet(local!(EXTERNREF)),
                Call(func_indices::STRING_EQUALS),
            ],
            (a, b) => hq_todo!("({:?}, {:?}) input types for operator_equals", a, b),
        },
        operator_random => vec![Call(func_indices::OPERATOR_RANDOM)],
        operator_join => vec![Call(func_indices::OPERATOR_JOIN)],
        operator_letter_of => vec![
            LocalSet(local!(EXTERNREF)),
            I32WrapI64,
            LocalGet(local!(EXTERNREF)),
            Call(func_indices::OPERATOR_LETTEROF),
        ],
        operator_length => vec![Call(func_indices::OPERATOR_LENGTH), I64ExtendI32U],
        operator_contains => vec![Call(func_indices::OPERATOR_CONTAINS)],
        operator_mathop { OPERATOR } => match OPERATOR.as_str() {
            "abs" => vec![F64Abs],
            "floor" => {
                if InputType::Integer.includes(input_types.first().unwrap()) {
                    vec![]
                } else {
                    vec![F64Floor, I64TruncF64S]
                }
            }
            "ceiling" => {
                if InputType::Integer.includes(input_types.first().unwrap()) {
                    vec![]
                } else {
                    vec![F64Ceil, I64TruncF64S]
                }
            }
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
            other => hq_bad_proj!("invalid OPERATOR field \"{}\"", other),
        },
        sensing_timer => vec![Call(func_indices::SENSING_TIMER)],
        sensing_resettimer => vec![Call(func_indices::SENSING_RESETTIMER)],
        sensing_dayssince2000 => vec![Call(func_indices::SENSING_DAYSSINCE2000)],
        pen_clear => vec![Call(func_indices::PEN_CLEAR)],
        pen_stamp => hq_todo!(""),
        pen_penDown => vec![
            I32Const(0),
            I32Const(0),
            I32Store8(MemArg {
                offset: 0,
                align: 0,
                memory_index: 0,
            }),
            I32Const(0),
            I32Const(1),
            I32Store8(MemArg {
                offset: (context.target_index - 1) as u64 * u64::try_from(SPRITE_INFO_LEN).unwrap()
                    + u64::try_from(byte_offset::VARS).map_err(|_| make_hq_bug!(""))?
                    + u64::try_from(context.vars.borrow().len()).map_err(|_| make_hq_bug!(""))?
                        * VAR_INFO_LEN
                    + u64::try_from(sprite_info_offsets::PEN_DOWN).map_err(|_| make_hq_bug!(""))?,
                align: 0,
                memory_index: 0,
            }),
            I32Const(0),
            F64Load(MemArg {
                offset: (context.target_index - 1) as u64 * u64::try_from(SPRITE_INFO_LEN).unwrap()
                    + u64::try_from(byte_offset::VARS).map_err(|_| make_hq_bug!(""))?
                    + u64::try_from(context.vars.borrow().len()).map_err(|_| make_hq_bug!(""))?
                        * VAR_INFO_LEN
                    + u64::try_from(sprite_info_offsets::PEN_SIZE).map_err(|_| make_hq_bug!(""))?,
                align: 3,
                memory_index: 0,
            }),
            I32Const(0),
            F64Load(MemArg {
                offset: (context.target_index - 1) as u64
                    * u64::try_from(SPRITE_INFO_LEN).map_err(|_| make_hq_bug!(""))?
                    + u64::try_from(byte_offset::VARS).map_err(|_| make_hq_bug!(""))?
                    + u64::try_from(context.vars.borrow().len()).map_err(|_| make_hq_bug!(""))?
                        * VAR_INFO_LEN
                    + u64::try_from(sprite_info_offsets::X_POS).map_err(|_| make_hq_bug!(""))?,
                align: 3,
                memory_index: 0,
            }),
            I32Const(0),
            F64Load(MemArg {
                offset: (context.target_index - 1) as u64
                    * u64::try_from(SPRITE_INFO_LEN).map_err(|_| make_hq_bug!(""))?
                    + u64::try_from(byte_offset::VARS).map_err(|_| make_hq_bug!(""))?
                    + u64::try_from(context.vars.borrow().len()).map_err(|_| make_hq_bug!(""))?
                        * VAR_INFO_LEN
                    + u64::try_from(sprite_info_offsets::Y_POS).map_err(|_| make_hq_bug!(""))?,
                align: 3,
                memory_index: 0,
            }),
            I32Const(0),
            F32Load(MemArg {
                offset: (context.target_index - 1) as u64
                    * u64::try_from(SPRITE_INFO_LEN).map_err(|_| make_hq_bug!(""))?
                    + u64::try_from(byte_offset::VARS).map_err(|_| make_hq_bug!(""))?
                    + u64::try_from(context.vars.borrow().len()).map_err(|_| make_hq_bug!(""))?
                        * VAR_INFO_LEN
                    + u64::try_from(sprite_info_offsets::PEN_R).map_err(|_| make_hq_bug!(""))?,
                align: 2,
                memory_index: 0,
            }),
            I32Const(0),
            F32Load(MemArg {
                offset: (context.target_index - 1) as u64
                    * u64::try_from(SPRITE_INFO_LEN).map_err(|_| make_hq_bug!(""))?
                    + u64::try_from(byte_offset::VARS).map_err(|_| make_hq_bug!(""))?
                    + u64::try_from(context.vars.borrow().len()).map_err(|_| make_hq_bug!(""))?
                        * VAR_INFO_LEN
                    + u64::try_from(sprite_info_offsets::PEN_G).map_err(|_| make_hq_bug!(""))?,
                align: 2,
                memory_index: 0,
            }),
            I32Const(0),
            F32Load(MemArg {
                offset: (context.target_index - 1) as u64
                    * u64::try_from(SPRITE_INFO_LEN).map_err(|_| make_hq_bug!(""))?
                    + u64::try_from(byte_offset::VARS).map_err(|_| make_hq_bug!(""))?
                    + u64::try_from(context.vars.borrow().len()).map_err(|_| make_hq_bug!(""))?
                        * VAR_INFO_LEN
                    + u64::try_from(sprite_info_offsets::PEN_B).map_err(|_| make_hq_bug!(""))?,
                align: 2,
                memory_index: 0,
            }),
            I32Const(0),
            F32Load(MemArg {
                offset: (context.target_index - 1) as u64
                    * u64::try_from(SPRITE_INFO_LEN).map_err(|_| make_hq_bug!(""))?
                    + u64::try_from(byte_offset::VARS).map_err(|_| make_hq_bug!(""))?
                    + u64::try_from(context.vars.borrow().len()).map_err(|_| make_hq_bug!(""))?
                        * VAR_INFO_LEN
                    + u64::try_from(sprite_info_offsets::PEN_A).map_err(|_| make_hq_bug!(""))?,
                align: 2,
                memory_index: 0,
            }),
            Call(func_indices::PEN_DOWN),
        ],
        motion_gotoxy => vec![
            I32Const(0),
            I32Const(0),
            I32Store8(MemArg {
                offset: 0,
                align: 0,
                memory_index: 0,
            }),
            LocalSet(local!(F64)),   // y
            LocalSet(local!(F64_2)), // x
            I32Const(0),
            I32Load8S(MemArg {
                offset: (context.target_index - 1) as u64
                    * u64::try_from(SPRITE_INFO_LEN).map_err(|_| make_hq_bug!(""))?
                    + u64::try_from(byte_offset::VARS).map_err(|_| make_hq_bug!(""))?
                    + u64::try_from(context.vars.borrow().len()).map_err(|_| make_hq_bug!(""))?
                        * VAR_INFO_LEN
                    + u64::try_from(sprite_info_offsets::PEN_DOWN).map_err(|_| make_hq_bug!(""))?,
                align: 0,
                memory_index: 0,
            }),
            If(WasmBlockType::Empty),
            I32Const(0),
            F64Load(MemArg {
                offset: (context.target_index - 1) as u64
                    * u64::try_from(SPRITE_INFO_LEN).map_err(|_| make_hq_bug!(""))?
                    + u64::try_from(byte_offset::VARS).map_err(|_| make_hq_bug!(""))?
                    + u64::try_from(context.vars.borrow().len()).map_err(|_| make_hq_bug!(""))?
                        * VAR_INFO_LEN
                    + u64::try_from(sprite_info_offsets::PEN_SIZE).map_err(|_| make_hq_bug!(""))?,
                align: 3,
                memory_index: 0,
            }),
            I32Const(0),
            F64Load(MemArg {
                offset: (context.target_index - 1) as u64
                    * u64::try_from(SPRITE_INFO_LEN).map_err(|_| make_hq_bug!(""))?
                    + u64::try_from(byte_offset::VARS).map_err(|_| make_hq_bug!(""))?
                    + u64::try_from(context.vars.borrow().len()).map_err(|_| make_hq_bug!(""))?
                        * VAR_INFO_LEN
                    + u64::try_from(sprite_info_offsets::X_POS).map_err(|_| make_hq_bug!(""))?,
                align: 3,
                memory_index: 0,
            }),
            I32Const(0),
            F64Load(MemArg {
                offset: (context.target_index - 1) as u64
                    * u64::try_from(SPRITE_INFO_LEN).map_err(|_| make_hq_bug!(""))?
                    + u64::try_from(byte_offset::VARS).map_err(|_| make_hq_bug!(""))?
                    + u64::try_from(context.vars.borrow().len()).map_err(|_| make_hq_bug!(""))?
                        * VAR_INFO_LEN
                    + u64::try_from(sprite_info_offsets::Y_POS).map_err(|_| make_hq_bug!(""))?,
                align: 3,
                memory_index: 0,
            }),
            LocalGet(local!(F64_2)),
            LocalGet(local!(F64)),
            I32Const(0),
            F32Load(MemArg {
                offset: (context.target_index - 1) as u64
                    * u64::try_from(SPRITE_INFO_LEN).map_err(|_| make_hq_bug!(""))?
                    + u64::try_from(byte_offset::VARS).map_err(|_| make_hq_bug!(""))?
                    + u64::try_from(context.vars.borrow().len()).map_err(|_| make_hq_bug!(""))?
                        * VAR_INFO_LEN
                    + u64::try_from(sprite_info_offsets::PEN_R).map_err(|_| make_hq_bug!(""))?,
                align: 2,
                memory_index: 0,
            }),
            I32Const(0),
            F32Load(MemArg {
                offset: (context.target_index - 1) as u64
                    * u64::try_from(SPRITE_INFO_LEN).map_err(|_| make_hq_bug!(""))?
                    + u64::try_from(byte_offset::VARS).map_err(|_| make_hq_bug!(""))?
                    + u64::try_from(context.vars.borrow().len()).map_err(|_| make_hq_bug!(""))?
                        * VAR_INFO_LEN
                    + u64::try_from(sprite_info_offsets::PEN_G).map_err(|_| make_hq_bug!(""))?,
                align: 2,
                memory_index: 0,
            }),
            I32Const(0),
            F32Load(MemArg {
                offset: (context.target_index - 1) as u64
                    * u64::try_from(SPRITE_INFO_LEN).map_err(|_| make_hq_bug!(""))?
                    + u64::try_from(byte_offset::VARS).map_err(|_| make_hq_bug!(""))?
                    + u64::try_from(context.vars.borrow().len()).map_err(|_| make_hq_bug!(""))?
                        * VAR_INFO_LEN
                    + u64::try_from(sprite_info_offsets::PEN_B).map_err(|_| make_hq_bug!(""))?,
                align: 2,
                memory_index: 0,
            }),
            I32Const(0),
            F32Load(MemArg {
                offset: (context.target_index - 1) as u64
                    * u64::try_from(SPRITE_INFO_LEN).map_err(|_| make_hq_bug!(""))?
                    + u64::try_from(byte_offset::VARS).map_err(|_| make_hq_bug!(""))?
                    + u64::try_from(context.vars.borrow().len()).map_err(|_| make_hq_bug!(""))?
                        * VAR_INFO_LEN
                    + u64::try_from(sprite_info_offsets::PEN_A).map_err(|_| make_hq_bug!(""))?,
                align: 2,
                memory_index: 0,
            }),
            Call(func_indices::PEN_LINETO),
            End,
            I32Const(0),
            LocalGet(local!(F64_2)),
            F64Store(MemArg {
                offset: (context.target_index - 1) as u64
                    * u64::try_from(SPRITE_INFO_LEN).map_err(|_| make_hq_bug!(""))?
                    + u64::try_from(byte_offset::VARS).map_err(|_| make_hq_bug!(""))?
                    + u64::try_from(context.vars.borrow().len()).map_err(|_| make_hq_bug!(""))?
                        * VAR_INFO_LEN
                    + u64::try_from(sprite_info_offsets::X_POS).map_err(|_| make_hq_bug!(""))?,
                align: 3,
                memory_index: 0,
            }),
            I32Const(0),
            LocalGet(local!(F64)),
            F64Store(MemArg {
                offset: (context.target_index - 1) as u64
                    * u64::try_from(SPRITE_INFO_LEN).map_err(|_| make_hq_bug!(""))?
                    + u64::try_from(byte_offset::VARS).map_err(|_| make_hq_bug!(""))?
                    + u64::try_from(context.vars.borrow().len()).map_err(|_| make_hq_bug!(""))?
                        * VAR_INFO_LEN
                    + u64::try_from(sprite_info_offsets::Y_POS).map_err(|_| make_hq_bug!(""))?,
                align: 3,
                memory_index: 0,
            }),
            I32Const(
                context
                    .target_index
                    .try_into()
                    .map_err(|_| make_hq_bug!(""))?,
            ),
            Call(func_indices::EMIT_SPRITE_POS_CHANGE),
        ],
        pen_penUp => vec![
            I32Const(0),
            I32Const(0),
            I32Store8(MemArg {
                offset: (context.target_index - 1) as u64
                    * u64::try_from(SPRITE_INFO_LEN).map_err(|_| make_hq_bug!(""))?
                    + u64::try_from(byte_offset::VARS).map_err(|_| make_hq_bug!(""))?
                    + u64::try_from(context.vars.borrow().len()).map_err(|_| make_hq_bug!(""))?
                        * VAR_INFO_LEN
                    + u64::try_from(sprite_info_offsets::PEN_DOWN).map_err(|_| make_hq_bug!(""))?,
                align: 0,
                memory_index: 0,
            }),
        ],
        pen_setPenColorToColor => vec![
            I32Const(
                context
                    .target_index
                    .try_into()
                    .map_err(|_| make_hq_bug!("target index out of bounds"))?,
            ),
            Call(func_indices::PEN_SETCOLOR),
        ],
        pen_changePenColorParamBy => vec![
            I32Const(
                context
                    .target_index
                    .try_into()
                    .map_err(|_| make_hq_bug!("target index out of bounds"))?,
            ),
            Call(func_indices::PEN_CHANGECOLORPARAM),
        ],
        pen_setPenColorParamTo => vec![
            I32Const(
                context
                    .target_index
                    .try_into()
                    .map_err(|_| make_hq_bug!("target index out of bounds"))?,
            ),
            Call(func_indices::PEN_SETCOLORPARAM),
        ],
        pen_changePenSizeBy => vec![
            I32Const(
                context
                    .target_index
                    .try_into()
                    .map_err(|_| make_hq_bug!("target index out of bounds"))?,
            ),
            Call(func_indices::PEN_CHANGESIZE),
        ],
        pen_setPenSizeTo => vec![
            LocalSet(local!(F64)),
            I32Const(0),
            LocalGet(local!(F64)),
            F64Store(MemArg {
                offset: (context.target_index - 1) as u64
                    * u64::try_from(SPRITE_INFO_LEN).map_err(|_| make_hq_bug!(""))?
                    + u64::try_from(byte_offset::VARS).map_err(|_| make_hq_bug!(""))?
                    + u64::try_from(context.vars.borrow().len()).map_err(|_| make_hq_bug!(""))?
                        * VAR_INFO_LEN
                    + u64::try_from(sprite_info_offsets::PEN_SIZE).map_err(|_| make_hq_bug!(""))?,
                align: 3,
                memory_index: 0,
            }),
        ],
        pen_setPenShadeToNumber => hq_todo!(""),
        pen_changePenShadeBy => hq_todo!(""),
        pen_setPenHueToNumber => vec![
            I32Const(
                context
                    .target_index
                    .try_into()
                    .map_err(|_| make_hq_bug!("target index out of bounds"))?,
            ),
            Call(func_indices::PEN_SETHUE),
        ],
        pen_changePenHueBy => vec![
            I32Const(
                context
                    .target_index
                    .try_into()
                    .map_err(|_| make_hq_bug!("target index out of bounds"))?,
            ),
            Call(func_indices::PEN_CHANGEHUE),
        ],
        looks_size => vec![
            I32Const(0),
            F64Load(MemArg {
                offset: (context.target_index - 1) as u64 * u64::try_from(SPRITE_INFO_LEN).unwrap()
                    + u64::try_from(byte_offset::VARS).map_err(|_| make_hq_bug!(""))?
                    + u64::try_from(context.vars.borrow().len()).map_err(|_| make_hq_bug!(""))?
                        * VAR_INFO_LEN
                    + u64::try_from(sprite_info_offsets::SIZE).map_err(|_| make_hq_bug!(""))?,
                align: 3,
                memory_index: 0,
            }),
        ],
        looks_setsizeto => vec![
            I32Const(0),
            I32Const(0),
            I32Store8(MemArg {
                offset: 0,
                align: 0,
                memory_index: 0,
            }),
            LocalSet(local!(F64)),
            I32Const(0),
            LocalGet(local!(F64)),
            F64Store(MemArg {
                offset: (context.target_index - 1) as u64
                    * u64::try_from(SPRITE_INFO_LEN).map_err(|_| make_hq_bug!(""))?
                    + u64::try_from(byte_offset::VARS).map_err(|_| make_hq_bug!(""))?
                    + u64::try_from(context.vars.borrow().len()).map_err(|_| make_hq_bug!(""))?
                        * VAR_INFO_LEN
                    + u64::try_from(sprite_info_offsets::SIZE).map_err(|_| make_hq_bug!(""))?,
                align: 3,
                memory_index: 0,
            }),
            I32Const(
                context
                    .target_index
                    .try_into()
                    .map_err(|_| make_hq_bug!(""))?,
            ),
            Call(func_indices::EMIT_SPRITE_SIZE_CHANGE),
        ],
        motion_turnleft => vec![
            I32Const(0),
            I32Const(0),
            I32Store8(MemArg {
                offset: 0,
                align: 0,
                memory_index: 0,
            }),
            LocalSet(local!(F64)),
            I32Const(0),
            F64Load(MemArg {
                offset: (context.target_index - 1) as u64
                    * u64::try_from(SPRITE_INFO_LEN).map_err(|_| make_hq_bug!(""))?
                    + u64::try_from(byte_offset::VARS).map_err(|_| make_hq_bug!(""))?
                    + u64::try_from(context.vars.borrow().len()).map_err(|_| make_hq_bug!(""))?
                        * VAR_INFO_LEN
                    + u64::try_from(sprite_info_offsets::ROTATION).map_err(|_| make_hq_bug!(""))?,
                align: 3,
                memory_index: 0,
            }),
            LocalGet(local!(F64)),
            F64Sub,
            LocalSet(local!(F64)),
            I32Const(0),
            LocalGet(local!(F64)),
            F64Store(MemArg {
                offset: (context.target_index - 1) as u64
                    * u64::try_from(SPRITE_INFO_LEN).map_err(|_| make_hq_bug!(""))?
                    + u64::try_from(byte_offset::VARS).map_err(|_| make_hq_bug!(""))?
                    + u64::try_from(context.vars.borrow().len()).map_err(|_| make_hq_bug!(""))?
                        * VAR_INFO_LEN
                    + u64::try_from(sprite_info_offsets::ROTATION).map_err(|_| make_hq_bug!(""))?,
                align: 3,
                memory_index: 0,
            }),
            I32Const(
                context
                    .target_index
                    .try_into()
                    .map_err(|_| make_hq_bug!(""))?,
            ),
            Call(func_indices::EMIT_SPRITE_ROTATION_CHANGE),
        ],
        motion_turnright => vec![
            I32Const(0),
            I32Const(0),
            I32Store8(MemArg {
                offset: 0,
                align: 0,
                memory_index: 0,
            }),
            LocalSet(local!(F64)),
            I32Const(0),
            F64Load(MemArg {
                offset: (context.target_index - 1) as u64
                    * u64::try_from(SPRITE_INFO_LEN).map_err(|_| make_hq_bug!(""))?
                    + u64::try_from(byte_offset::VARS).map_err(|_| make_hq_bug!(""))?
                    + u64::try_from(context.vars.borrow().len()).map_err(|_| make_hq_bug!(""))?
                        * VAR_INFO_LEN
                    + u64::try_from(sprite_info_offsets::ROTATION).map_err(|_| make_hq_bug!(""))?,
                align: 3,
                memory_index: 0,
            }),
            LocalGet(local!(F64)),
            F64Add,
            LocalSet(local!(F64)),
            I32Const(0),
            LocalGet(local!(F64)),
            F64Store(MemArg {
                offset: (context.target_index - 1) as u64
                    * u64::try_from(SPRITE_INFO_LEN).map_err(|_| make_hq_bug!(""))?
                    + u64::try_from(byte_offset::VARS).map_err(|_| make_hq_bug!(""))?
                    + u64::try_from(context.vars.borrow().len()).map_err(|_| make_hq_bug!(""))?
                        * VAR_INFO_LEN
                    + u64::try_from(sprite_info_offsets::ROTATION).map_err(|_| make_hq_bug!(""))?,
                align: 3,
                memory_index: 0,
            }),
            I32Const(
                context
                    .target_index
                    .try_into()
                    .map_err(|_| make_hq_bug!(""))?,
            ),
            Call(func_indices::EMIT_SPRITE_ROTATION_CHANGE),
        ],
        looks_changesizeby => vec![
            I32Const(0),
            I32Const(0),
            I32Store8(MemArg {
                offset: 0,
                align: 0,
                memory_index: 0,
            }),
            LocalSet(local!(F64)),
            I32Const(0),
            F64Load(MemArg {
                offset: (context.target_index - 1) as u64
                    * u64::try_from(SPRITE_INFO_LEN).map_err(|_| make_hq_bug!(""))?
                    + u64::try_from(byte_offset::VARS).map_err(|_| make_hq_bug!(""))?
                    + u64::try_from(context.vars.borrow().len()).map_err(|_| make_hq_bug!(""))?
                        * VAR_INFO_LEN
                    + u64::try_from(sprite_info_offsets::SIZE).map_err(|_| make_hq_bug!(""))?,
                align: 3,
                memory_index: 0,
            }),
            LocalGet(local!(F64)),
            F64Add,
            LocalSet(local!(F64)),
            I32Const(0),
            LocalGet(local!(F64)),
            F64Store(MemArg {
                offset: (context.target_index - 1) as u64
                    * u64::try_from(SPRITE_INFO_LEN).map_err(|_| make_hq_bug!(""))?
                    + u64::try_from(byte_offset::VARS).map_err(|_| make_hq_bug!(""))?
                    + u64::try_from(context.vars.borrow().len()).map_err(|_| make_hq_bug!(""))?
                        * VAR_INFO_LEN
                    + u64::try_from(sprite_info_offsets::SIZE).map_err(|_| make_hq_bug!(""))?,
                align: 3,
                memory_index: 0,
            }),
            I32Const(
                context
                    .target_index
                    .try_into()
                    .map_err(|_| make_hq_bug!(""))?,
            ),
            Call(func_indices::EMIT_SPRITE_SIZE_CHANGE),
        ],
        looks_switchcostumeto => vec![
            I32Const(0),
            I32Const(0),
            I32Store8(MemArg {
                offset: 0,
                align: 0,
                memory_index: 0,
            }),
            LocalSet(local!(I64)),
            I32Const(0),
            LocalGet(local!(I64)),
            I32WrapI64,
            I32Store(MemArg {
                offset: (context.target_index - 1) as u64
                    * u64::try_from(SPRITE_INFO_LEN).map_err(|_| make_hq_bug!(""))?
                    + u64::try_from(byte_offset::VARS).map_err(|_| make_hq_bug!(""))?
                    + u64::try_from(context.vars.borrow().len()).map_err(|_| make_hq_bug!(""))?
                        * VAR_INFO_LEN
                    + u64::try_from(sprite_info_offsets::COSTUME).map_err(|_| make_hq_bug!(""))?,
                align: 2,
                memory_index: 0,
            }),
            I32Const(
                context
                    .target_index
                    .try_into()
                    .map_err(|_| make_hq_bug!(""))?,
            ),
            Call(func_indices::EMIT_SPRITE_COSTUME_CHANGE),
        ],
        hq_drop(n) => vec![Drop; 2 * *n],
        hq_goto { step: None, .. } => {
            let threads_offset: i32 = (byte_offset::VARS as usize
                + VAR_INFO_LEN as usize * context.vars.borrow().len()
                + usize::try_from(SPRITE_INFO_LEN).map_err(|_| make_hq_bug!(""))?
                    * (context.target_num - 1))
                .try_into()
                .map_err(|_| make_hq_bug!("thread_offset out of bounds"))?;
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
                        .map_err(|_| make_hq_bug!("THREAD_NUM out of bounds"))?,
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
                        .map_err(|_| make_hq_bug!("THREAD_NUM out of bounds"))?,
                    align: 2,
                    memory_index: 0,
                }),
                I32Const(1),
                I32Sub,
                I32Store(MemArg {
                    offset: byte_offset::THREAD_NUM
                        .try_into()
                        .map_err(|_| make_hq_bug!("THREAD_NUM out of bounds"))?,
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
            let next_step_index = steps.get_index_of(next_step_id).ok_or(make_hq_bug!(""))?;
            let threads_offset: u64 = (byte_offset::VARS as usize
                + VAR_INFO_LEN as usize * context.vars.borrow().len()
                + usize::try_from(SPRITE_INFO_LEN).map_err(|_| make_hq_bug!(""))?
                    * (context.target_num - 1))
                .try_into()
                .map_err(|_| make_hq_bug!("threads_offset length out of bounds"))?;
            vec![
                LocalGet(0),
                I32Const(
                    next_step_index
                        .try_into()
                        .map_err(|_| make_hq_bug!("step index out of bounds"))?,
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
            let next_step_index = steps.get_index_of(next_step_id).ok_or(make_hq_bug!(""))?;
            vec![
                LocalGet(local!(MEM_LOCATION)),
                ReturnCall(
                    BUILTIN_FUNCS
                        + u32::try_from(next_step_index)
                            .map_err(|_| make_hq_bug!("next_step_index out of bounds"))?,
                ),
            ]
        }
        hq_goto_if { step: None, .. } => {
            let threads_offset: i32 = (byte_offset::VARS as usize
                + VAR_INFO_LEN as usize * context.vars.borrow().len()
                + usize::try_from(SPRITE_INFO_LEN).map_err(|_| make_hq_bug!(""))?
                    * (context.target_num - 1))
                .try_into()
                .map_err(|_| make_hq_bug!("thread_offset out of bounds"))?;
            vec![
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
                        .map_err(|_| make_hq_bug!("THREAD_NUM out of bounds"))?,
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
                        .map_err(|_| make_hq_bug!("THREAD_NUM out of bounds"))?,
                    align: 2,
                    memory_index: 0,
                }),
                I32Const(1),
                I32Sub,
                I32Store(MemArg {
                    offset: byte_offset::THREAD_NUM
                        .try_into()
                        .map_err(|_| make_hq_bug!("THREAD_NUM out of bounds"))?,
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
            let next_step_index = steps.get_index_of(next_step_id).ok_or(make_hq_bug!(""))?;
            let threads_offset: u64 = (byte_offset::VARS as usize
                + VAR_INFO_LEN as usize * context.vars.borrow().len()
                + usize::try_from(SPRITE_INFO_LEN).map_err(|_| make_hq_bug!(""))?
                    * (context.target_num - 1))
                .try_into()
                .map_err(|_| make_hq_bug!("threads_offset length out of bounds"))?;
            vec![
                If(WasmBlockType::Empty),
                LocalGet(0),
                I32Const(
                    next_step_index
                        .try_into()
                        .map_err(|_| make_hq_bug!("step index out of bounds"))?,
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
            let next_step_index = steps.get_index_of(next_step_id).ok_or(make_hq_bug!(""))?;
            vec![
                If(WasmBlockType::Empty),
                LocalGet(local!(MEM_LOCATION)),
                ReturnCall(
                    BUILTIN_FUNCS
                        + u32::try_from(next_step_index)
                            .map_err(|_| make_hq_bug!("next_step_index out of bounds"))?,
                ),
                End,
            ]
        }
        hq_cast(from, to) => match (from.clone(), to.least_restrictive_concrete_type()) {
            // cast from type should always be a concrete type
            (String, Float) => vec![Call(func_indices::CAST_PRIMITIVE_STRING_FLOAT)],
            (String, Boolean) => vec![Call(func_indices::CAST_PRIMITIVE_STRING_BOOL)],
            (String, Unknown) => vec![
                LocalSet(local!(EXTERNREF)),
                I32Const(hq_value_types::EXTERN_STRING_REF64),
                LocalGet(local!(EXTERNREF)),
                Call(func_indices::TABLE_ADD_STRING),
                I64ExtendI32U,
            ],
            (Boolean, Float) => vec![Call(func_indices::CAST_BOOL_FLOAT)],
            (Boolean, String) => vec![Call(func_indices::CAST_BOOL_STRING)],
            (Boolean, Unknown) => vec![
                I64ExtendI32S,
                LocalSet(local!(I64)),
                I32Const(hq_value_types::BOOL64),
                LocalGet(local!(I64)),
            ],
            (Float, String) => vec![Call(func_indices::CAST_PRIMITIVE_FLOAT_STRING)],
            (Float, Boolean) => vec![Call(func_indices::CAST_FLOAT_BOOL)],
            (Float, Unknown) => vec![
                LocalSet(local!(F64)),
                I32Const(hq_value_types::FLOAT64),
                LocalGet(local!(F64)),
                I64ReinterpretF64,
            ],
            (ConcreteInteger, Unknown) => vec![
                //I64ExtendI32S,
                LocalSet(local!(I64)),
                I32Const(hq_value_types::INT64),
                LocalGet(local!(I64)),
            ],
            (Unknown, String) => vec![Call(func_indices::CAST_ANY_STRING)],
            (Unknown, Float) => vec![Call(func_indices::CAST_ANY_FLOAT)],
            (Unknown, Boolean) => vec![Call(func_indices::CAST_ANY_BOOL)],
            (Unknown, ConcreteInteger) => vec![Call(func_indices::CAST_ANY_INT)],
            _ => hq_todo!("unimplemented cast: {:?} -> {:?} at {:?}", from, to, op),
        },
        hq_launch_procedure(procedure) => {
            let step_tuple = (procedure.target_id.clone(), procedure.first_step.clone());
            let step_idx = steps
                .get_index_of(&step_tuple)
                .ok_or(make_hq_bug!("couldn't find step"))?;
            if procedure.warp {
                vec![
                    LocalGet(local!(MEM_LOCATION)),
                    Call(
                        BUILTIN_FUNCS
                            + u32::try_from(step_idx)
                                .map_err(|_| make_hq_bug!("step_idx out of bounds"))?,
                    ),
                ]
            } else {
                hq_todo!("non-warping procedure")
            }
        }
        other => hq_todo!("missing WASM impl for {:?}", other),
    };
    if op.does_request_redraw()
        && !context.proc.clone().is_some_and(|p| p.warp)
        && !(*op.opcode() == looks_say && context.dbg)
    {
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
    Ok(instructions)
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
    ) -> Result<u32, HQError>;
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
    ) -> Result<u32, HQError> {
        if step_funcs.contains_key(&Some(self.0.clone())) {
            return u32::try_from(
                step_funcs
                    .get_index_of(&Some(self.0.clone()))
                    .ok_or(make_hq_bug!(""))?,
            )
            .map_err(|_| make_hq_bug!("IndexMap index out of bounds"));
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
        for (i, op) in self.1.opcodes().iter().enumerate() {
            let arity = op.opcode().expected_inputs()?.len();
            let input_types = (0..arity)
                .map(|j| {
                    Ok(self
                        .1
                        .opcodes()
                        .get(i - 1)
                        .ok_or(make_hq_bug!(""))?
                        //.unwrap()
                        .type_stack
                        .get(arity - 1 - j)
                        .borrow()
                        .clone()
                        .ok_or(make_hq_bug!("{:?}", op.opcode()))?
                        //.unwrap()
                        .1)
                })
                .collect::<Result<Vec<_>, _>>()?;
            let instrs = instructions(op, self.1.context(), string_consts, steps, input_types)?;
            for instr in instrs {
                func.instruction(&instr);
            }
        }
        func.instruction(&Instruction::End);
        step_funcs.insert(Some(self.0.clone()), func);
        u32::try_from(step_funcs.len() - 1)
            .map_err(|_| make_hq_bug!("step_funcs length out of bounds"))
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
    pub const STRING_EQUALS: u32 = 8;
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
    pub const SENSING_DAYSSINCE2000: u32 = 26;
    pub const PEN_CLEAR: u32 = 27;
    pub const PEN_DOWN: u32 = 28;
    pub const PEN_LINETO: u32 = 29;
    pub const PEN_SETCOLOR: u32 = 30;
    pub const PEN_CHANGECOLORPARAM: u32 = 31;
    pub const PEN_SETCOLORPARAM: u32 = 32;
    pub const PEN_CHANGESIZE: u32 = 33;
    pub const PEN_SETHUE: u32 = 34;
    pub const PEN_CHANGEHUE: u32 = 35;
    pub const EMIT_SPRITE_POS_CHANGE: u32 = 36;
    pub const EMIT_SPRITE_SIZE_CHANGE: u32 = 37;
    pub const EMIT_SPRITE_COSTUME_CHANGE: u32 = 38;
    pub const EMIT_SPRITE_X_CHANGE: u32 = 39;
    pub const EMIT_SPRITE_Y_CHANGE: u32 = 40;
    pub const EMIT_SPRITE_ROTATION_CHANGE: u32 = 41;
    pub const EMIT_SPRITE_VISIBILITY_CHANGE: u32 = 42;

    /* wasm funcs */
    pub const UNREACHABLE: u32 = 43;
    pub const FMOD: u32 = 44;
    pub const CAST_FLOAT_BOOL: u32 = 45;
    pub const CAST_BOOL_FLOAT: u32 = 46;
    pub const CAST_BOOL_STRING: u32 = 47;
    pub const CAST_ANY_STRING: u32 = 48;
    pub const CAST_ANY_FLOAT: u32 = 49;
    pub const CAST_ANY_BOOL: u32 = 50;
    pub const CAST_ANY_INT: u32 = 51;
    pub const TABLE_ADD_STRING: u32 = 52;
    pub const SPRITE_UPDATE_PEN_COLOR: u32 = 53;
}
pub const BUILTIN_FUNCS: u32 = 54;
pub const IMPORTED_FUNCS: u32 = 43;

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
    pub const I32I64I32I64_I32: u32 = 30;
    pub const F64I32I64_EXTERNREF: u32 = 31;
    pub const I32I64I32I64_EXTERNREF: u32 = 32;
    pub const F64_F64: u32 = 33;
    pub const NOPARAM_I32: u32 = 34;
    pub const I32x2_NORESULT: u32 = 35;
    pub const EXTERNREFF64I32_NORESULT: u32 = 36;
    pub const F64x3F32x4_NORESULT: u32 = 37;
    pub const F64x5F32x4_NORESULT: u32 = 38;
    pub const EXTERNREFx2_EXTERNREF: u32 = 39;
    pub const EXTERNREFx2_I32: u32 = 40;
    pub const F64EXTERNREF_EXTERNREF: u32 = 41;
    pub const I32EXTERNREF_EXTERNREF: u32 = 42;
    pub const I32_EXTERNREF: u32 = 43;
    pub const I32I64_I32: u32 = 44;
    pub const EXTERNREFx2_REFEXTERN: u32 = 45;
}

pub mod table_indices {
    pub const STEP_FUNCS: u32 = 0;
    pub const STRINGS: u32 = 1;
}

pub mod hq_value_types {
    pub const FLOAT64: i32 = 0;
    pub const BOOL64: i32 = 1;
    pub const EXTERN_STRING_REF64: i32 = 2;
    pub const INT64: i32 = 3;
}

// the number of bytes that one step takes up in linear memory
pub const THREAD_BYTE_LEN: i32 = 4;

pub mod byte_offset {
    pub const REDRAW_REQUESTED: i32 = 0;
    pub const THREAD_NUM: i32 = 4;
    pub const VARS: i32 = 8;
}

pub const SPRITE_INFO_LEN: i32 = 80;

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
    pub const VISIBLE: i32 = 57;
    //pub const RESERVED1: i32 = 58;
    pub const COSTUME: i32 = 60;
    pub const SIZE: i32 = 64;
    pub const ROTATION: i32 = 72;
}

pub const VAR_INFO_LEN: u64 = 16;

pub mod var_info_offsets {
    pub const VAR_TYPE: i32 = 0;
    pub const VAR_VAL: i32 = 8;
}

pub const BUILTIN_GLOBALS: u32 = 3;

impl TryFrom<IrProject> for WasmProject {
    type Error = HQError;

    fn try_from(project: IrProject) -> Result<Self, Self::Error> {
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
            page_size_log2: None,
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
            [ValType::I32],
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
            [
                ValType::F64,
                ValType::F64,
                ValType::F64,
                ValType::F32,
                ValType::F32,
                ValType::F32,
                ValType::F32,
            ],
            [],
        );
        types.function(
            [
                ValType::F64,
                ValType::F64,
                ValType::F64,
                ValType::F64,
                ValType::F64,
                ValType::F32,
                ValType::F32,
                ValType::F32,
                ValType::F32,
            ],
            [],
        );
        types.function(
            [
                ValType::Ref(RefType::EXTERNREF),
                ValType::Ref(RefType::EXTERNREF),
            ],
            [ValType::Ref(RefType::EXTERNREF)],
        );
        types.function(
            [
                ValType::Ref(RefType::EXTERNREF),
                ValType::Ref(RefType::EXTERNREF),
            ],
            [ValType::I32],
        );
        types.function(
            [ValType::F64, ValType::Ref(RefType::EXTERNREF)],
            [ValType::Ref(RefType::EXTERNREF)],
        );
        types.function(
            [ValType::I32, ValType::Ref(RefType::EXTERNREF)],
            [ValType::Ref(RefType::EXTERNREF)],
        );
        types.function([ValType::I32], [ValType::Ref(RefType::EXTERNREF)]);
        types.function([ValType::I32, ValType::I64], [ValType::I32]);
        types.function(
            [
                ValType::Ref(RefType::EXTERNREF),
                ValType::Ref(RefType::EXTERNREF),
            ],
            [ValType::Ref(RefType {
                nullable: false,
                heap_type: HeapType::Abstract {
                    shared: false, // what does this even mean?!
                    ty: AbstractHeapType::Extern,
                },
            })],
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
            "wasm:js-string",
            "equals",
            EntityType::Function(types::EXTERNREFx2_I32),
        );
        imports.import(
            "runtime",
            "operator_random",
            EntityType::Function(types::F64x2_F64),
        );
        imports.import(
            "wasm:js-string",
            "concat",
            EntityType::Function(types::EXTERNREFx2_REFEXTERN),
        );
        imports.import(
            "runtime",
            "operator_letterof",
            EntityType::Function(types::I32EXTERNREF_EXTERNREF),
        );
        imports.import(
            "wasm:js-string",
            "length",
            EntityType::Function(types::EXTERNREF_I32),
        );
        imports.import(
            "runtime",
            "operator_contains",
            EntityType::Function(types::EXTERNREFx2_I32),
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
            "sensing_dayssince2000",
            EntityType::Function(types::NOPARAM_F64),
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
        imports.import(
            "runtime",
            "emit_sprite_pos_change",
            EntityType::Function(types::I32_NORESULT),
        );
        imports.import(
            "runtime",
            "emit_sprite_size_change",
            EntityType::Function(types::I32_NORESULT),
        );
        imports.import(
            "runtime",
            "emit_sprite_costume_change",
            EntityType::Function(types::I32_NORESULT),
        );
        imports.import(
            "runtime",
            "emit_sprite_x_change",
            EntityType::Function(types::I32_NORESULT),
        );
        imports.import(
            "runtime",
            "emit_sprite_y_change",
            EntityType::Function(types::I32_NORESULT),
        );
        imports.import(
            "runtime",
            "emit_sprite_rotation_change",
            EntityType::Function(types::I32_NORESULT),
        );
        imports.import(
            "runtime",
            "emit_sprite_visibility_change",
            EntityType::Function(types::I32_NORESULT),
        );

        // used to expose the wasm module to devtools
        functions.function(types::NOPARAM_NORESULT);
        let mut unreachable_func = Function::new(vec![]);
        unreachable_func.instruction(&Instruction::Unreachable);
        unreachable_func.instruction(&Instruction::End);
        code.function(&unreachable_func);
        exports.export(
            "unreachable_dbg",
            ExportKind::Func,
            func_indices::UNREACHABLE,
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

        functions.function(types::F64_I32);
        let mut float2bool_func = Function::new(vec![]);
        float2bool_func.instruction(&Instruction::LocalGet(0));
        float2bool_func.instruction(&Instruction::F64Abs);
        float2bool_func.instruction(&Instruction::F64Const(0.0));
        float2bool_func.instruction(&Instruction::F64Eq);
        float2bool_func.instruction(&Instruction::End);
        code.function(&float2bool_func);

        functions.function(types::I32_F64);
        let mut bool2float_func = Function::new(vec![]);
        bool2float_func.instruction(&Instruction::LocalGet(0));
        bool2float_func.instruction(&Instruction::F64ConvertI32S);
        bool2float_func.instruction(&Instruction::End);
        code.function(&bool2float_func);

        functions.function(types::I32_EXTERNREF);
        let mut bool2string_func = Function::new(vec![]);
        bool2string_func.instruction(&Instruction::LocalGet(0));
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
        any2string_func.instruction(&Instruction::I32WrapI64);
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
        any2string_func.instruction(&Instruction::LocalGet(0));
        any2string_func.instruction(&Instruction::I32Const(hq_value_types::INT64));
        any2string_func.instruction(&Instruction::I32Eq);
        any2string_func.instruction(&Instruction::If(WasmBlockType::FunctionType(
            types::NOPARAM_EXTERNREF,
        )));
        any2string_func.instruction(&Instruction::LocalGet(1));
        any2string_func.instruction(&Instruction::F64ConvertI64S); // just convert to a float and then to a string, seeing as all valid scratch integers are valid floats. hopefully it won't break.
        any2string_func.instruction(&Instruction::Call(
            func_indices::CAST_PRIMITIVE_FLOAT_STRING,
        ));
        any2string_func.instruction(&Instruction::Else);
        any2string_func.instruction(&Instruction::Unreachable);
        any2string_func.instruction(&Instruction::End);
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
        any2float_func.instruction(&Instruction::I32WrapI64);
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
        any2float_func.instruction(&Instruction::LocalGet(0));
        any2float_func.instruction(&Instruction::I32Const(hq_value_types::INT64));
        any2float_func.instruction(&Instruction::I32Eq);
        any2float_func.instruction(&Instruction::If(WasmBlockType::FunctionType(
            types::NOPARAM_F64,
        )));
        any2float_func.instruction(&Instruction::LocalGet(1));
        any2float_func.instruction(&Instruction::F64ConvertI64S);
        any2float_func.instruction(&Instruction::Else);
        any2float_func.instruction(&Instruction::Unreachable);
        any2float_func.instruction(&Instruction::End);
        any2float_func.instruction(&Instruction::End);
        any2float_func.instruction(&Instruction::End);
        any2float_func.instruction(&Instruction::End);
        any2float_func.instruction(&Instruction::End);
        code.function(&any2float_func);

        functions.function(types::I32I64_I32);
        let mut any2bool_func = Function::new(vec![]);
        any2bool_func.instruction(&Instruction::LocalGet(0));
        any2bool_func.instruction(&Instruction::I32Const(hq_value_types::EXTERN_STRING_REF64));
        any2bool_func.instruction(&Instruction::I32Eq);
        any2bool_func.instruction(&Instruction::If(WasmBlockType::FunctionType(
            types::NOPARAM_I32,
        )));
        any2bool_func.instruction(&Instruction::LocalGet(1));
        any2bool_func.instruction(&Instruction::I32WrapI64);
        any2bool_func.instruction(&Instruction::TableGet(table_indices::STRINGS));
        any2bool_func.instruction(&Instruction::Call(func_indices::CAST_PRIMITIVE_STRING_BOOL));
        any2bool_func.instruction(&Instruction::Else);
        any2bool_func.instruction(&Instruction::LocalGet(0));
        any2bool_func.instruction(&Instruction::I32Const(hq_value_types::FLOAT64));
        any2bool_func.instruction(&Instruction::I32Eq);
        any2bool_func.instruction(&Instruction::If(WasmBlockType::FunctionType(
            types::NOPARAM_I32,
        )));
        any2bool_func.instruction(&Instruction::LocalGet(1));
        any2bool_func.instruction(&Instruction::F64ReinterpretI64);
        any2bool_func.instruction(&Instruction::Call(func_indices::CAST_FLOAT_BOOL));
        any2bool_func.instruction(&Instruction::Else);
        any2bool_func.instruction(&Instruction::LocalGet(0));
        any2bool_func.instruction(&Instruction::I32Const(hq_value_types::BOOL64));
        any2bool_func.instruction(&Instruction::I32Eq);
        any2bool_func.instruction(&Instruction::If(WasmBlockType::FunctionType(
            types::NOPARAM_I32,
        )));
        any2bool_func.instruction(&Instruction::LocalGet(1));
        any2bool_func.instruction(&Instruction::I32WrapI64);
        any2bool_func.instruction(&Instruction::Else);
        any2bool_func.instruction(&Instruction::Unreachable);
        any2bool_func.instruction(&Instruction::End);
        any2bool_func.instruction(&Instruction::End);
        any2bool_func.instruction(&Instruction::End);
        any2bool_func.instruction(&Instruction::End);
        code.function(&any2bool_func);

        functions.function(types::I32I64_I64);
        let mut any2int_func = Function::new(vec![]);
        any2int_func.instruction(&Instruction::Unreachable);
        any2int_func.instruction(&Instruction::End);
        code.function(&any2int_func);

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
            (byte_offset::VARS as usize + project.vars.borrow().len() * VAR_INFO_LEN as usize)
                .try_into()
                .map_err(|_| make_hq_bug!(""))?,
        ));
        sprite_update_pen_color_func.instruction(&Instruction::I32Add);
        sprite_update_pen_color_func.instruction(&Instruction::LocalTee(supc_locals::MEM_POS)); // position in memory of sprite info
        sprite_update_pen_color_func.instruction(&Instruction::F32Load(MemArg {
            offset: u64::try_from(sprite_info_offsets::PEN_COLOR).map_err(|_| make_hq_bug!(""))?,
            align: 2,
            memory_index: 0,
        }));
        sprite_update_pen_color_func.instruction(&Instruction::F32Const(2.55));
        sprite_update_pen_color_func.instruction(&Instruction::F32Mul);
        sprite_update_pen_color_func.instruction(&Instruction::I32TruncF32S);
        sprite_update_pen_color_func.instruction(&Instruction::LocalSet(supc_locals::HUE)); // hue
        sprite_update_pen_color_func.instruction(&Instruction::LocalGet(supc_locals::MEM_POS));
        sprite_update_pen_color_func.instruction(&Instruction::F32Load(MemArg {
            offset: u64::try_from(sprite_info_offsets::PEN_SATURATION)
                .map_err(|_| make_hq_bug!(""))?,
            align: 2,
            memory_index: 0,
        }));
        sprite_update_pen_color_func.instruction(&Instruction::F32Const(2.55));
        sprite_update_pen_color_func.instruction(&Instruction::F32Mul);
        sprite_update_pen_color_func.instruction(&Instruction::I32TruncF32S);
        sprite_update_pen_color_func.instruction(&Instruction::LocalSet(supc_locals::SAT)); // saturation ∈ [0, 256)
        sprite_update_pen_color_func.instruction(&Instruction::LocalGet(supc_locals::MEM_POS));
        sprite_update_pen_color_func.instruction(&Instruction::F32Load(MemArg {
            offset: u64::try_from(sprite_info_offsets::PEN_VALUE).map_err(|_| make_hq_bug!(""))?,
            align: 2,
            memory_index: 0,
        }));
        sprite_update_pen_color_func.instruction(&Instruction::F32Const(2.55));
        sprite_update_pen_color_func.instruction(&Instruction::F32Mul);
        sprite_update_pen_color_func.instruction(&Instruction::I32TruncF32S);
        sprite_update_pen_color_func.instruction(&Instruction::LocalSet(supc_locals::VAL)); // value ∈ [0, 256)
        sprite_update_pen_color_func.instruction(&Instruction::LocalGet(supc_locals::MEM_POS));
        sprite_update_pen_color_func.instruction(&Instruction::F32Const(100.0));
        sprite_update_pen_color_func.instruction(&Instruction::LocalGet(supc_locals::MEM_POS));
        sprite_update_pen_color_func.instruction(&Instruction::F32Load(MemArg {
            offset: u64::try_from(sprite_info_offsets::PEN_TRANSPARENCY)
                .map_err(|_| make_hq_bug!(""))?,
            align: 2,
            memory_index: 0,
        })); // transparency ∈ [0, 100]
        sprite_update_pen_color_func.instruction(&Instruction::F32Sub);
        sprite_update_pen_color_func.instruction(&Instruction::F32Const(100.0));
        sprite_update_pen_color_func.instruction(&Instruction::F32Div); // alpha ∈ [0, 1]
        sprite_update_pen_color_func.instruction(&Instruction::F32Store(MemArg {
            offset: u64::try_from(sprite_info_offsets::PEN_A).map_err(|_| make_hq_bug!(""))?,
            align: 2,
            memory_index: 0,
        }));
        sprite_update_pen_color_func.instruction(&Instruction::LocalGet(supc_locals::MEM_POS));
        sprite_update_pen_color_func.instruction(&Instruction::F32Load(MemArg {
            offset: u64::try_from(sprite_info_offsets::PEN_A).map_err(|_| make_hq_bug!(""))?,
            align: 2,
            memory_index: 0,
        }));
        sprite_update_pen_color_func.instruction(&Instruction::F32Const(0.01));
        sprite_update_pen_color_func.instruction(&Instruction::F32Lt);
        sprite_update_pen_color_func.instruction(&Instruction::If(WasmBlockType::Empty));
        sprite_update_pen_color_func.instruction(&Instruction::I32Const(
            supc_locals::MEM_POS
                .try_into()
                .map_err(|_| make_hq_bug!(""))?,
        ));
        sprite_update_pen_color_func.instruction(&Instruction::F32Const(0.0));
        sprite_update_pen_color_func.instruction(&Instruction::F32Store(MemArg {
            offset: u64::try_from(sprite_info_offsets::PEN_A).map_err(|_| make_hq_bug!(""))?,
            align: 2,
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
            offset: u64::try_from(sprite_info_offsets::PEN_R).map_err(|_| make_hq_bug!(""))?,
            align: 2,
            memory_index: 0,
        }));
        sprite_update_pen_color_func.instruction(&Instruction::LocalGet(supc_locals::MEM_POS));
        sprite_update_pen_color_func.instruction(&Instruction::LocalGet(supc_locals::VAL_F));
        sprite_update_pen_color_func.instruction(&Instruction::F32Store(MemArg {
            offset: u64::try_from(sprite_info_offsets::PEN_G).map_err(|_| make_hq_bug!(""))?,
            align: 2,
            memory_index: 0,
        }));
        sprite_update_pen_color_func.instruction(&Instruction::LocalGet(supc_locals::MEM_POS));
        sprite_update_pen_color_func.instruction(&Instruction::LocalGet(supc_locals::VAL_F));
        sprite_update_pen_color_func.instruction(&Instruction::F32Store(MemArg {
            offset: u64::try_from(sprite_info_offsets::PEN_B).map_err(|_| make_hq_bug!(""))?,
            align: 2,
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
        sprite_update_pen_color_func.instruction(&Instruction::F32ConvertI32S);
        sprite_update_pen_color_func.instruction(&Instruction::F32Const(255.0));
        sprite_update_pen_color_func.instruction(&Instruction::F32Div);
        sprite_update_pen_color_func.instruction(&Instruction::F32Store(MemArg {
            offset: u64::try_from(sprite_info_offsets::PEN_R).map_err(|_| make_hq_bug!(""))?,
            align: 2,
            memory_index: 0,
        }));
        sprite_update_pen_color_func.instruction(&Instruction::LocalGet(supc_locals::MEM_POS));
        sprite_update_pen_color_func.instruction(&Instruction::LocalGet(supc_locals::G));
        sprite_update_pen_color_func.instruction(&Instruction::F32ConvertI32S);
        sprite_update_pen_color_func.instruction(&Instruction::F32Const(255.0));
        sprite_update_pen_color_func.instruction(&Instruction::F32Div);
        sprite_update_pen_color_func.instruction(&Instruction::F32Store(MemArg {
            offset: u64::try_from(sprite_info_offsets::PEN_G).map_err(|_| make_hq_bug!(""))?,
            align: 2,
            memory_index: 0,
        }));
        sprite_update_pen_color_func.instruction(&Instruction::LocalGet(supc_locals::MEM_POS));
        sprite_update_pen_color_func.instruction(&Instruction::LocalGet(supc_locals::B));
        sprite_update_pen_color_func.instruction(&Instruction::F32ConvertI32S);
        sprite_update_pen_color_func.instruction(&Instruction::F32Const(255.0));
        sprite_update_pen_color_func.instruction(&Instruction::F32Div);
        sprite_update_pen_color_func.instruction(&Instruction::F32Store(MemArg {
            offset: u64::try_from(sprite_info_offsets::PEN_B).map_err(|_| make_hq_bug!(""))?,
            align: 2,
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
            step.compile_wasm(&mut step_funcs, &mut string_consts, &project.steps)?;
        }

        for thread in project.threads {
            let first_idx = project
                .steps
                .get_index_of(&(thread.target_id().clone(), thread.first_step().clone()))
                .ok_or(make_hq_bug!(""))?
                .try_into()
                .map_err(|_| make_hq_bug!("step index out of bounds"))?;
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

        for (maybe_step, func) in &step_funcs {
            code.function(func);
            if maybe_step.is_none() {
                functions.function(types::I32_I32);
                continue;
            }
            functions.function(
                match project
                    .steps
                    .get(&maybe_step.clone().unwrap())
                    .ok_or(make_hq_bug!("missing step"))?
                    .context
                    .proc
                {
                    None => types::I32_I32,
                    Some(Procedure {
                        warp,
                        ref arg_types,
                        ..
                    }) => {
                        if !warp {
                            hq_todo!("non-warp procedure")
                        } else {
                            if !arg_types.is_empty() {
                                hq_todo!("proc args")
                            }
                            types::I32_I32
                        }
                    }
                },
            );
        }

        for (start_type, index) in thread_indices {
            let func = func_for_thread_start!(start_type);
            func.instruction(&Instruction::I32Const(0));
            func.instruction(&Instruction::I32Load(MemArg {
                offset: byte_offset::THREAD_NUM
                    .try_into()
                    .map_err(|_| make_hq_bug!("THREAD_NUM out of bounds"))?,
                align: 2,
                memory_index: 0,
            }));
            func.instruction(&Instruction::I32Const(THREAD_BYTE_LEN));
            func.instruction(&Instruction::I32Mul);
            let thread_start_count: i32 = (*thread_start_counts.get(&start_type).unwrap_or(&0))
                .try_into()
                .map_err(|_| make_hq_bug!("start_type count out of bounds"))?;
            func.instruction(&Instruction::I32Const(thread_start_count * THREAD_BYTE_LEN));
            func.instruction(&Instruction::I32Add);
            func.instruction(&Instruction::I32Const(
                index
                    .try_into()
                    .map_err(|_| make_hq_bug!("step func index out of bounds"))?,
            ));
            func.instruction(&Instruction::I32Store(MemArg {
                offset: (byte_offset::VARS as usize
                    + VAR_INFO_LEN as usize * project.vars.borrow().len()
                    + usize::try_from(SPRITE_INFO_LEN).map_err(|_| make_hq_bug!(""))?
                        * (project.target_names.len() - 1))
                    .try_into()
                    .map_err(|_| make_hq_bug!("i32.store offset out of bounds"))?,
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
                    .map_err(|_| make_hq_bug!("THREAD_NUM out of bounds"))?,
                align: 2,
                memory_index: 0,
            }));
            func.instruction(&Instruction::I32Const(
                count
                    .try_into()
                    .map_err(|_| make_hq_bug!("thread_start count out of bounds"))?,
            ));
            func.instruction(&Instruction::I32Add);
            func.instruction(&Instruction::I32Store(MemArg {
                offset: byte_offset::THREAD_NUM
                    .try_into()
                    .map_err(|_| make_hq_bug!("THREAD_NUM out of bounds"))?,
                align: 2,
                memory_index: 0,
            }));
        }

        {
            tick_func.instruction(&Instruction::I32Const(0));
            tick_func.instruction(&Instruction::I32Load(MemArg {
                offset: byte_offset::THREAD_NUM
                    .try_into()
                    .map_err(|_| make_hq_bug!("THREAD_NUM out of bounds"))?,
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
                    + VAR_INFO_LEN as usize * project.vars.borrow().len()
                    + usize::try_from(SPRITE_INFO_LEN).map_err(|_| make_hq_bug!(""))?
                        * (project.target_names.len() - 1))
                    .try_into()
                    .map_err(|_| make_hq_bug!("i32.store offset out of bounds"))?,
                align: 2, // 2 ** 2 = 4 (bytes)
                memory_index: 0,
            }));
            tick_func.instruction(&Instruction::CallIndirect {
                type_index: types::I32_I32,
                table_index: table_indices::STEP_FUNCS,
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
                .map_err(|_| make_hq_bug!("step_funcs length out of bounds"))?,
            maximum: Some(
                step_funcs
                    .len()
                    .try_into()
                    .map_err(|_| make_hq_bug!("step_funcs length out of bounds"))?,
            ),
            table64: false,
            shared: false,
        });

        globals.global(
            GlobalType {
                val_type: ValType::I32,
                mutable: false,
                shared: false,
            },
            &ConstExpr::i32_const(byte_offset::REDRAW_REQUESTED),
        );
        globals.global(
            GlobalType {
                val_type: ValType::I32,
                mutable: false,
                shared: false,
            },
            &ConstExpr::i32_const(byte_offset::THREAD_NUM),
        );
        globals.global(
            GlobalType {
                val_type: ValType::I32,
                mutable: false,
                shared: false,
            },
            &ConstExpr::i32_const(
                project
                    .vars
                    .borrow()
                    .len()
                    .try_into()
                    .map_err(|_| make_hq_bug!("vars length out of bounds"))?,
            ),
        );
        for _ in 0..project.vars.borrow().len() {
            // TODO: only create globals for variables that are at some point assumed to be strings
            globals.global(
                GlobalType {
                    val_type: ValType::Ref(RefType::EXTERNREF),
                    mutable: true,
                    shared: false,
                },
                &ConstExpr::ref_null(HeapType::Abstract {
                    shared: false,
                    ty: AbstractHeapType::Extern,
                }),
            );
        }

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

        let mut data = DataSection::new();

        let mut default_data: Vec<u8> = Vec::with_capacity(
            8 + 16 * project.vars.borrow().len() + 80 * (project.target_names.len() - 1),
        );

        default_data.extend([0; 8]);

        //project.vars;

        for var in project.vars.take() {
            match var.initial_value() {
                VarVal::Float(float) => {
                    default_data.extend([0; 8]);
                    default_data.extend(float.to_le_bytes());
                }
                VarVal::Bool(boolean) => {
                    default_data.extend(1i64.to_le_bytes());
                    default_data.extend([0; 4]);
                    default_data.extend((*boolean as i64).to_le_bytes());
                }
                VarVal::String(string) => {
                    default_data.extend(2i64.to_le_bytes());
                    default_data.extend([0; 4]);
                    let index = string_consts.len();
                    string_consts.push(string.clone());
                    default_data.extend((index as u64).to_le_bytes());
                }
            }
        }

        for target in project.sb3.targets {
            if target.is_stage {
                continue;
            }
            default_data.extend(target.x.to_le_bytes());
            default_data.extend(target.y.to_le_bytes());
            default_data.extend([0; 41]);
            default_data.push(target.visible as u8);
            default_data.extend([0; 2]);
            default_data.extend(target.current_costume.to_le_bytes());
            default_data.extend(target.size.to_le_bytes());
            default_data.extend(target.direction.to_le_bytes());
        }

        data.active(0, &ConstExpr::i32_const(0), default_data);

        tables.table(TableType {
            element_type: RefType::EXTERNREF,
            minimum: string_consts
                .len()
                .try_into()
                .map_err(|_| make_hq_bug!("string_consts len out of bounds"))?,
            maximum: None,
            table64: false,
            shared: false,
        });

        let step_indices = (BUILTIN_FUNCS
            ..(u32::try_from(step_funcs.len())
                .map_err(|_| make_hq_bug!("step_funcs length out of bounds"))?
                + BUILTIN_FUNCS))
            .collect::<Vec<_>>();
        let step_func_indices = Elements::Functions(&step_indices[..]);
        elements.active(
            Some(table_indices::STEP_FUNCS),
            &ConstExpr::i32_const(0),
            step_func_indices,
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
            .section(&code)
            .section(&data);

        let wasm_bytes = module.finish();
        Ok(Self {
            target_names: project.target_names.clone(),
            wasm_bytes,
            string_consts,
        })
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::process::{Command, Stdio};

    #[test]
    fn make_wasm() -> Result<(), HQError> {
        use crate::sb3::Sb3Project;
        use std::fs;
        let proj: Sb3Project = fs::read_to_string("./benchmark (3.1).json")
            .expect("couldn't read hq-test.project.json")
            .try_into()
            .unwrap();
        let ir: IrProject = proj.try_into()?;
        let wasm: WasmProject = ir.try_into()?;
        Ok(())
        /*fs::write("./bad.wasm", wasm.wasm_bytes()).expect("failed to write to bad.wasm");
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
        }*/
    }
}
