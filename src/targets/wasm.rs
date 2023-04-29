use crate::ir::{
    BlockType as IrBlockType, IrBlock, IrOpcode, IrProject, Step, ThreadContext, ThreadStart,
};
use alloc::collections::BTreeMap;
use alloc::rc::Rc;
use alloc::string::String;
use alloc::vec::Vec;
use core::cell::Cell;
use wasm_encoder::{
    BlockType, CodeSection, ConstExpr, ElementSection, Elements, EntityType, ExportKind,
    ExportSection, Function, FunctionSection, ImportSection, Instruction, MemArg, MemorySection,
    MemoryType, Module, RefType, TableSection, TableType, TypeSection, ValType,
};

impl Step {
    fn as_function(&self, step_counter: &mut dyn FnMut() -> u32, string_consts: &mut Vec<String>) -> Function
    {
        let locals = vec![
            ValType::Ref(RefType::EXTERNREF),
            ValType::F64,
            ValType::I64,
            ValType::I32,
            ValType::I32,
        ];
        let next_step_index = step_counter();
        #[cfg(test)]
        println!("next step is {}", next_step_index);
        let mut func = Function::new_with_locals_types(locals);
        //func.instruction(&Instruction::LocalGet(0));
        //func.instruction(&Instruction::F64ConvertI32S);
        //func.instruction(&Instruction::F64ReinterpretI64);
        //func.instruction(&Instruction::Call(func_indices::DBG_LOG));
        for op in self.opcodes() {
            for instr in instructions(op, self.context(), string_consts) {
                func.instruction(&instr);
            }
        }
        let thread_indices: u64 = byte_offset::THREADS
            .try_into()
            .expect("THREAD_INDICES out of bounds (E018)");
        let vars_num: i32 = self
            .context()
            .vars
            .len()
            .try_into()
            .expect("vars.len() out of bounds (E032)");
        if next_step_index > 0 {
            func.instruction(&Instruction::LocalGet(0));
            func.instruction(&Instruction::I32Const(
                next_step_index
                    .try_into()
                    .expect("step index out of bounds (E001)"),
            ));
            func.instruction(&Instruction::I32Store(MemArg {
                offset: thread_indices,
                align: 2,
                memory_index: 0,
            }));
            func.instruction(&Instruction::I32Const(1));
        } else {
            func.instruction(&Instruction::LocalGet(0));
            func.instruction(&Instruction::I32Const(byte_offset::THREADS));
            func.instruction(&Instruction::I32Add); // destination (= current thread pos in memory)
            func.instruction(&Instruction::LocalGet(0));
            func.instruction(&Instruction::I32Const(byte_offset::THREADS + 4));
            func.instruction(&Instruction::I32Add); // source (= current thread pos + 4)
            func.instruction(&Instruction::I32Const(0));
            func.instruction(&Instruction::I32Load(MemArg {
                offset: byte_offset::THREAD_NUM
                    .try_into()
                    .expect("THREAD_NUM out of bounds (E009)"),
                align: 2,
                memory_index: 0,
            }));
            func.instruction(&Instruction::I32Const(4));
            func.instruction(&Instruction::I32Mul);
            func.instruction(&Instruction::I32Const(
                byte_offset::THREADS - 4 + vars_num * 12,
            ));
            func.instruction(&Instruction::I32Add);
            func.instruction(&Instruction::LocalGet(0));
            func.instruction(&Instruction::I32Sub); // length (threadnum * 4 + THREADS offset - 4 + number of variables - current thread pos)
            func.instruction(&Instruction::MemoryCopy {
                src_mem: 0,
                dst_mem: 0,
            });
            func.instruction(&Instruction::I32Const(0));
            func.instruction(&Instruction::I32Const(0));
            func.instruction(&Instruction::I32Load(MemArg {
                offset: byte_offset::THREAD_NUM
                    .try_into()
                    .expect("THREAD_NUM out of bounds (E010)"),
                align: 2,
                memory_index: 0,
            }));
            func.instruction(&Instruction::I32Const(1));
            func.instruction(&Instruction::I32Sub);
            func.instruction(&Instruction::I32Store(MemArg {
                offset: byte_offset::THREAD_NUM
                    .try_into()
                    .expect("THREAD_NUM out of bounds (E011)"),
                align: 2,
                memory_index: 0,
            }));
            func.instruction(&Instruction::I32Const(0));
        }
        //func.instruction(&Instruction::End);
        func
    }
}

fn instructions(
    op: &IrBlock,
    context: Rc<ThreadContext>,
    string_consts: &mut Vec<String>,
) -> Vec<Instruction<'static>> {
    use Instruction::*;
    use IrBlockType::*;
    use IrOpcode::*;
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
        | math_positive_number { NUM } => vec![F64Const(*NUM)],
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
            .expect("string index out of bounds (E022)");
            vec![I32Const(str_idx), TableGet(table_indices::STRINGS)]
        }
        data_variable { VARIABLE } => {
            let var_index: i32 = context
                .vars
                .iter()
                .position(|var| VARIABLE == var.id())
                .expect("couldn't find variable index (E033)")
                .try_into()
                .expect("variable index out of bounds (E034)");
            let var_offset: u64 = (byte_offset::THREADS + 12 * var_index)
                .try_into()
                .expect("variable offset out of bounds (E035)");
            vec![
                I32Const(0),
                I32Load(MemArg {
                    offset: byte_offset::THREAD_NUM
                        .try_into()
                        .expect("THREAD_NUM put of bounds (E036)"),
                    align: 2,
                    memory_index: 0,
                }),
                LocalTee(step_func_locals::I32),
                I32Const(THREAD_BYTE_LEN),
                I32Mul,
                I32Load(MemArg {
                    offset: var_offset,
                    align: 2,
                    memory_index: 0,
                }),
                LocalGet(step_func_locals::I32),
                I32Const(THREAD_BYTE_LEN),
                I32Mul,
                I64Load(MemArg {
                    offset: var_offset + 4,
                    align: 2,
                    memory_index: 0,
                }),
            ]
        }
        data_setvariableto { VARIABLE } => {
            //vec![Drop,Drop]
            let var_index: i32 = context
                .vars
                .iter()
                .position(|var| VARIABLE == var.id())
                .expect("couldn't find variable index (E033)")
                .try_into()
                .expect("variable index out of bounds (E034)");
            let var_offset: u64 = (byte_offset::THREADS + 12 * var_index)
                .try_into()
                .expect("variable offset out of bounds (E035)");
            vec![
                LocalSet(step_func_locals::I64),
                LocalSet(step_func_locals::I32),
                I32Const(0),
                I32Load(MemArg {
                    offset: byte_offset::THREAD_NUM
                        .try_into()
                        .expect("THREAD_NUM out of bounds (E036)"),
                    align: 2,
                    memory_index: 0,
                }),
                LocalTee(step_func_locals::I32_2),
                I32Const(THREAD_BYTE_LEN),
                I32Mul,
                LocalGet(step_func_locals::I32),
                I32Store(MemArg {
                    offset: var_offset,
                    align: 2,
                    memory_index: 0,
                }),
                LocalGet(step_func_locals::I32_2),
                I32Const(THREAD_BYTE_LEN),
                I32Mul,
                LocalGet(step_func_locals::I64),
                I64Store(MemArg {
                    offset: var_offset + 4,
                    align: 2,
                    memory_index: 0,
                }),
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
        control_if { SUBSTACK } => {
            let mut instrs = vec![I32WrapI64, If(BlockType::Empty)];
            instrs.append(&mut SUBSTACK[0].opcodes().iter().flat_map(|o| instructions(o, Rc::clone(&context), string_consts)).collect());
            instrs.push(End);
            instrs
            //vec![Drop]
        },
        control_if_else { SUBSTACK, SUBSTACK2 } => {
            let mut instrs = vec![I32WrapI64, If(BlockType::Empty)];
            instrs.append(&mut SUBSTACK[0].opcodes().iter().flat_map(|o| instructions(o, Rc::clone(&context), string_consts)).collect());
            instrs.push(Else);
            instrs.append(&mut SUBSTACK2[0].opcodes().iter().flat_map(|o| instructions(o, Rc::clone(&context), string_consts)).collect());
            instrs.push(End);
            instrs
           // vec![Drop]
        },
        _ => todo!(),
    };
    instructions.append(&mut match (op.actual_output(), op.expected_output()) {
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

pub struct WebWasmFile {
    js_string: String,
    wasm_bytes: Vec<u8>,
}

impl WebWasmFile {
    pub fn wasm_bytes(&self) -> &Vec<u8> {
        &self.wasm_bytes
    }
    pub fn js_string(&self) -> &String {
        &self.js_string
    }
}

pub mod step_func_locals {
    pub const MEM_LOCATION: u32 = 0;
    pub const EXTERNREF: u32 = 1;
    pub const F64: u32 = 2;
    pub const I64: u32 = 3;
    pub const I32: u32 = 4;
    pub const I32_2: u32 = 5;
}

pub mod func_indices {
    /* imported funcs */
    pub const DBG_LOG: u32 = 0;
    pub const DBG_ASSERT: u32 = 1;
    pub const LOOKS_SAY: u32 = 2;
    pub const LOOKS_THINK: u32 = 3;
    pub const CAST_PRIMITIVE_FLOAT_STRING: u32 = 4; // js functions can only rwturm 1 value so need wrapper functions for casting
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

    /* wasm funcs */
    pub const FMOD: u32 = 24;
    pub const CAST_FLOAT_BOOL: u32 = 25;
    pub const CAST_BOOL_FLOAT: u32 = 26;
    pub const CAST_BOOL_STRING: u32 = 27;
    pub const CAST_ANY_STRING: u32 = 28;
    pub const CAST_ANY_FLOAT: u32 = 29;
    pub const CAST_ANY_BOOL: u32 = 30;
    pub const TABLE_ADD_STRING: u32 = 31;
}
pub const BUILTIN_FUNCS: u32 = 32;
pub const IMPORTED_FUNCS: u32 = 24;

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
    pub const THREADS: i32 = 8;
}

impl From<IrProject> for WebWasmFile {
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
        any2string_func.instruction(&Instruction::If(BlockType::FunctionType(
            types::NOPARAM_EXTERNREF,
        )));
        any2string_func.instruction(&Instruction::LocalGet(1));
        any2string_func.instruction(&Instruction::Call(func_indices::CAST_BOOL_STRING));
        any2string_func.instruction(&Instruction::Else);
        any2string_func.instruction(&Instruction::LocalGet(0));
        any2string_func.instruction(&Instruction::I32Const(hq_value_types::FLOAT64));
        any2string_func.instruction(&Instruction::I32Eq);
        any2string_func.instruction(&Instruction::If(BlockType::FunctionType(
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
        any2string_func.instruction(&Instruction::If(BlockType::FunctionType(
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
        any2float_func.instruction(&Instruction::If(BlockType::FunctionType(
            types::NOPARAM_F64,
        )));
        any2float_func.instruction(&Instruction::LocalGet(1));
        any2float_func.instruction(&Instruction::Call(func_indices::CAST_BOOL_FLOAT));
        any2float_func.instruction(&Instruction::Else);
        any2float_func.instruction(&Instruction::LocalGet(0));
        any2float_func.instruction(&Instruction::I32Const(hq_value_types::EXTERN_STRING_REF64));
        any2float_func.instruction(&Instruction::I32Eq);
        any2float_func.instruction(&Instruction::If(BlockType::FunctionType(
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
        any2float_func.instruction(&Instruction::If(BlockType::FunctionType(
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
        any2bool_func.instruction(&Instruction::If(BlockType::FunctionType(
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
        any2bool_func.instruction(&Instruction::If(BlockType::FunctionType(
            types::NOPARAM_I64,
        )));
        any2bool_func.instruction(&Instruction::LocalGet(1));
        any2bool_func.instruction(&Instruction::F64ReinterpretI64);
        any2bool_func.instruction(&Instruction::Call(func_indices::CAST_FLOAT_BOOL));
        any2bool_func.instruction(&Instruction::Else);
        any2bool_func.instruction(&Instruction::LocalGet(0));
        any2bool_func.instruction(&Instruction::I32Const(hq_value_types::BOOL64));
        any2bool_func.instruction(&Instruction::I32Eq);
        any2bool_func.instruction(&Instruction::If(BlockType::FunctionType(
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
        tbl_add_string_func.instruction(&Instruction::If(BlockType::Empty));
        tbl_add_string_func.instruction(&Instruction::Unreachable);
        tbl_add_string_func.instruction(&Instruction::End);
        tbl_add_string_func.instruction(&Instruction::LocalGet(1));
        tbl_add_string_func.instruction(&Instruction::End);
        code.function(&tbl_add_string_func);

        let mut gf_func = Function::new(vec![]);
        let mut tick_func = Function::new(vec![(2, ValType::I32)]);

        /*tick_func.instruction(&Instruction::I32Const(0));
        tick_func.instruction(&Instruction::I32Const(0));
        tick_func.instruction(&Instruction::I32Store8(MemArg { offset: 0, align: 0, memory_index: 0 }));*/

        functions.function(types::I32_I32);
        let mut noop_func = Function::new(vec![]);
        noop_func.instruction(&Instruction::I32Const(1));
        noop_func.instruction(&Instruction::End);
        code.function(&noop_func);

        /*let mut thread_indices: BTreeMap<ThreadStart, Vec<u32>> = BTreeMap::from([
            (ThreadStart::GreenFlag, vec![]),
        ]);*/
        let mut thread_indices: Vec<(ThreadStart, u32)> = vec![]; // (start type, first step index)

        let mut step_indices: Vec<u32> = vec![0];

        let mut string_consts = vec![String::from("false"), String::from("true")];

        let step_index = Cell::new(0);
        let mut next_step_index = || {
            step_index.set(step_index.get() + 1);
            step_index.get()
        };
        let mut zero_func = || 0;

        for thread in project.threads {
            let first_index = step_index.get() + 1;
            thread_indices.push((thread.start().clone(), first_index));
            for (i, step) in thread.steps().iter().enumerate() {
                let mut func = step.as_function(
                    if i < thread.steps().len() - 1 {
                        &mut next_step_index
                    } else {
                        dbg!("zero");
                        &mut zero_func
                    },
                    &mut string_consts,
                );
                func.instruction(&Instruction::End);
                functions.function(types::I32_I32);
                code.function(&func);
                step_indices.push(step_index.get());
            }
        }

        let mut thread_start_counts: BTreeMap<ThreadStart, u32> = Default::default();

        macro_rules! func_for_thread_start {
            ($start_type:ident) => {
                match $start_type {
                    ThreadStart::GreenFlag => &mut gf_func,
                }
            };
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
                offset: (byte_offset::THREADS) as u64,
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
            tick_func.instruction(&Instruction::I32Const(THREAD_BYTE_LEN));
            tick_func.instruction(&Instruction::I32Mul);
            tick_func.instruction(&Instruction::I32Const(THREAD_BYTE_LEN));
            tick_func.instruction(&Instruction::I32Sub);
            tick_func.instruction(&Instruction::LocalSet(1));
            tick_func.instruction(&Instruction::Loop(BlockType::Empty));

            tick_func.instruction(&Instruction::LocalGet(0));
            tick_func.instruction(&Instruction::LocalGet(0));
            tick_func.instruction(&Instruction::I32Load(MemArg {
                offset: (byte_offset::THREADS) as u64,
                align: 2, // 2 ** 2 = 4 (bytes)
                memory_index: 0,
            }));
            tick_func.instruction(&Instruction::CallIndirect {
                ty: types::I32_I32,
                table: table_indices::STEP_FUNCS,
            });

            tick_func.instruction(&Instruction::If(BlockType::Empty));
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
            minimum: step_indices
                .len()
                .try_into()
                .expect("step indices length out of bounds (E007)"),
            maximum: Some(
                step_indices
                    .len()
                    .try_into()
                    .expect("step indices length out of bounds (E008)"),
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

        for i in &mut step_indices {
            *i += BUILTIN_FUNCS;
        }
        let step_func_indices = Elements::Functions(&step_indices[..]);
        elements.active(
            Some(table_indices::STEP_FUNCS),
            &ConstExpr::i32_const(0),
            RefType::FUNCREF,
            step_func_indices,
        );

        exports.export("step_funcs", ExportKind::Table, table_indices::STEP_FUNCS);
        exports.export("strings", ExportKind::Table, table_indices::STRINGS);
        exports.export("memory", ExportKind::Memory, 0);

        module
            .section(&types)
            .section(&imports)
            .section(&functions)
            .section(&tables)
            .section(&memories)
            // globals
            .section(&exports)
            // start
            .section(&elements)
            // datacount
            .section(&code);
        // data

        let wasm_bytes = module.finish();
        Self { js_string: format!("const assert = require('node:assert')/*.strict*/;
        let last_output;
        let strings_tbl;
        const wasm_val_to_js = (type, value_i64) => {{
            return type === 0 ? new Float64Array(new BigInt64Array([value_i64]).buffer)[0] : (type === 1 ? Boolean(value_i64) : (type === 2 ? strings_tbl.get(Number(value_i64)) : null));
        }};
        const wasm_output = (...args) => {{
            const val = wasm_val_to_js(...args);
            console.log('output: \\x1b[34m%s\\x1b[0m', val);
            last_output = val;
        }};
        const assert_output = (...args) => {{
            /*assert.equal(last_output, wasm_val_to_js(...args));*/
            const val = wasm_val_to_js(...args);
            console.log('assert: \\x1b[34m%s\\x1b[0m', val);
        }}
        const targetOutput = (targetIndex, verb, text) => {{
            let targetName = {target_names:?}[targetIndex];
            console.log(`\\x1b[1;32m${{targetName}} ${{verb}}:\\x1b[0m \\x1b[35m${{text}}\\x1b[0m`);
        }};
        const importObject = {{
            dbg: {{
                log: wasm_output,
                assert: assert_output,
                logi32 (i32) {{
                    console.log('logi32: \\x1b[33m%d\\x1b[0m', i32);
                    return i32;
                }},
            }},
            runtime: {{
                looks_say: (ty, val, targetIndex) => targetOutput(targetIndex, 'says', wasm_val_to_js(ty, val)),
                looks_think: (ty, val, targetIndex) => targetOutput(targetIndex, 'thinks', wasm_val_to_js(ty, val)),
                operator_equals: (ty1, val1, ty2, val2) => wasm_val_to_js(ty1, val1) == wasm_val_to_js(ty2, val2),
                operator_random: (lower, upper) => Math.random() * (upper - lower) + lower,
                operator_join: (ty1, val1, ty2, val2) => wasm_val_to_js(ty1, val1).toString() + wasm_val_to_js(ty2, val2).toString(),
                operator_letterof: (idx, ty, val) => wasm_val_to_js(ty, val).toString()[idx - 1] ?? '',
                operator_length: (ty, val) => wasm_val_to_js(ty, val).toString().length,
                operator_contains: (ty1, val1, ty2, val2) => wasm_val_to_js(ty1, val1).toString().includes(wasm_val_to_js(ty2, val2).toString()),
                mathop_sin: (n) => parseFloat(Math.sin((Math.PI * n) / 180).toFixed(10)),
                mathop_cos: (n) => parseFloat(Math.cos((Math.PI * n) / 180).toFixed(10)),
                mathop_tan: (n) => {{
                    /* https://github.com/scratchfoundation/scratch-vm/blob/f1f10e0aa856fef6596a622af72b49e2f491f937/src/util/math-util.js#L53-65 */
                    n = n % 360;
                    switch (n) {{
                        case -270:
                        case 90:
                            return Infinity;
                        case -90:
                        case 270:
                            return -Infinity;
                        default:
                            return parseFloat(Math.tan((Math.PI * n) / 180).toFixed(10));
                    }}
                }},
                mathop_asin: (n) => (Math.asin(n) * 180) / Math.PI,
                mathop_acos: (n) => (Math.acos(n) * 180) / Math.PI,
                mathop_atan: (n) => (Math.atan(n) * 180) / Math.PI,
                mathop_ln: (n) => Math.log(n),
                mathop_log: (n) => Math.log(n) / Math.LN10,
                mathop_pow_e: (n) => Math.exp(n),
                mathop_pow10: (n) => Math.pow(10, n),
            }},
            cast: {{
              stringtofloat: parseFloat,
              stringtobool: Boolean,
              floattostring: Number.prototype.toString,
            }},
        }};
        const buf = new Uint8Array({buf:?});
        try {{
            assert(WebAssembly.validate(buf));
        }} catch {{
            try {{
                new WebAssembly.Module(buf);
                console.error('invalid WASM module');
                process.exit(1);
            }} catch (e) {{
                console.error('invalid WASM module: ' + e.message);
                process.exit(1);
            }}
        }}
        function sleep(ms) {{
            return new Promise((resolve) => {{
                setTimeout(resolve, ms);
            }});
        }}
        WebAssembly.instantiate(buf, importObject).then(async ({{ instance }}) => {{
            const {{ green_flag, tick, memory, strings }} = instance.exports;
            for (const [i, str] of Object.entries({string_consts:?})) {{
              strings.set(i, str);
            }}
            strings_tbl = strings;
            green_flag();
            $outertickloop: while (true) {{
                console.log('outer')
                const startTime = Date.now();
                $innertickloop: while (Date.now() - startTime < 23 && new Uint8Array(memory.buffer)[{rr_offset}] === 0) {{
                    console.log('inner')
                    tick();
                    if (!new Uint32Array(memory.buffer).slice({threads_offset}/4, {threads_offset}/4 + new Uint32Array(memory.buffer)[{thn_offset}/4] + 1).some(x => x > 0)) {{
                        break $outertickloop;
                    }}
                }}
                new Uint8Array(memory.buffer)[{rr_offset}] = 0;
                await sleep(30 - (Date.now() - startTime));
            }}
        }}).catch((e) => {{
            console.error('error when instantiating module:\\n' + e.stack);
            process.exit(1);
        }});
        ", target_names=&project.targets, buf=&wasm_bytes, rr_offset=byte_offset::REDRAW_REQUESTED, threads_offset=byte_offset::THREADS, thn_offset=byte_offset::THREAD_NUM), wasm_bytes }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::process::{Command, Stdio};

    /*#[test]
    fn make_thread() {
        use IrOpcode::*;
        let proj: Sb3Project = test_project_id("771449498").try_into().unwrap();
        let thread = Thread::from_hat(proj.targets[0].blocks.iter().filter(|(_id, b)| match b {
            Block::Normal { block_info, .. } => block_info.opcode() == BlockOpcode::event_whenflagclicked,
            Block::Special(_) => false,
        }).next().unwrap().1.clone(), proj.targets[0].blocks.clone());
        assert_eq!(thread.start(), &ThreadStart::GreenFlag);
        assert_eq!(thread.steps(), &vec![
            Step::new(vec![
                looks_say,
                operator_add,
                math_number { NUM: 4.0 },
                math_number { NUM: 1.0 },
                looks_think,
                math_number { NUM: 5.0 },
                looks_say,
                operator_subtract,
                math_number { NUM: 3.0 },
                math_number { NUM: 1.0 },
                looks_think,
                math_number { NUM: 2.0 },
            ])
        ]);
    }*/

    #[test]
    fn run_wasm() {
        use crate::sb3::Sb3Project;
        use std::fs;
        let proj: Sb3Project = fs::read_to_string("./hq-test.project.json")
            .expect("couldn't read hq-test.project.json")
            .try_into()
            .unwrap();
        let ir: IrProject = proj.into();
        let wasm: WebWasmFile = ir.into();
        println!("{}", wasm.js_string());
        let output2 = Command::new("node")
            .arg("-e")
            .arg(format!(
                "fs.writeFileSync('bad.wasm', Buffer.from({buf:?}), 'binary');",
                buf = wasm.wasm_bytes()
            ))
            .stdout(Stdio::inherit())
            .output()
            .expect("failed to execute process");
        if !output2.status.success() {
            panic!("failed to write to bad.wasm");
        }
        let output = Command::new("node")
            .arg("-e")
            .arg(wasm.js_string())
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
    }
}
