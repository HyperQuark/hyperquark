use crate::sb3::*;
use alloc::collections::BTreeMap;
use alloc::string::String;
use alloc::vec::Vec;
use wasm_encoder::{
    CodeSection, EntityType, ExportKind, ExportSection, Function, FunctionSection, ImportSection, Instruction, Module, TypeSection, ValType, TableSection, TableType, Elements, ElementSection, MemorySection, MemoryType, ConstExpr, MemArg, BlockType, HeapType, RefType,
};

#[derive(Debug, Clone, PartialEq, PartialOrd, Eq, Ord)]
enum ThreadStart {
    GreenFlag,
}

#[derive(Debug, Clone, PartialEq)]
struct Step {
    opcodes: Vec<BlockOpcodeWithField>,
    index: u32,
}

impl Step {
    fn new(opcodes: Vec<BlockOpcodeWithField>, index: u32) -> Self {
        Step { opcodes, index }
    }
    fn opcodes(&self) -> &Vec<BlockOpcodeWithField> {
        &self.opcodes
    }
    fn index(&self) -> &u32 {
        &self.index
    }
    fn into_function(&self, context: &Context, next_step_index: u32) -> Function {
        let locals = vec![];
        #[cfg(test)]
        println!("next step is {}", next_step_index);
        let mut func = Function::new(locals);
        //func.instruction(&Instruction::LocalGet(0));
        //func.instruction(&Instruction::F64ConvertI32S);
        //func.instruction(&Instruction::F64ReinterpretI64);
        //func.instruction(&Instruction::Call(func_indices::DBG_LOG));
        for op in self.opcodes() {
            for instr in instructions(op, context) {
                func.instruction(&instr);
            }
        }
        let thread_indices: u64 = byte_offset::THREADS.try_into().expect("THREAD_INDICES out of bounds (E018)");
        if next_step_index > 0 {
            func.instruction(&Instruction::LocalGet(0));
            func.instruction(&Instruction::I32Const(next_step_index.try_into().expect("step index out of bounds (E001)")));
            func.instruction(&Instruction::I32Store(MemArg { offset: thread_indices, align: 2, memory_index: 0 }));
        } else {
            func.instruction(&Instruction::LocalGet(0));
            func.instruction(&Instruction::I32Const(byte_offset::THREADS));
            func.instruction(&Instruction::I32Add);
            func.instruction(&Instruction::LocalGet(0));
            func.instruction(&Instruction::I32Const(byte_offset::THREADS + 4));
            func.instruction(&Instruction::I32Add);
            func.instruction(&Instruction::I32Const(0));
            func.instruction(&Instruction::I32Load(MemArg { offset: byte_offset::THREAD_NUM.try_into().expect("THREAD_NUM out of bounds (E009)"), align: 2, memory_index: 0, }));
            func.instruction(&Instruction::I32Const(4));
            func.instruction(&Instruction::I32Mul);
            func.instruction(&Instruction::I32Const(byte_offset::THREADS));
            func.instruction(&Instruction::I32Add);
            func.instruction(&Instruction::LocalGet(0));
            func.instruction(&Instruction::I32Sub);
            func.instruction(&Instruction::I32Const(4));
            func.instruction(&Instruction::I32Sub);
            func.instruction(&Instruction::MemoryCopy {
                src_mem: 0,
                dst_mem: 0,
            });
            func.instruction(&Instruction::I32Const(0));
            func.instruction(&Instruction::I32Const(0));
            func.instruction(&Instruction::I32Load(MemArg { offset: byte_offset::THREAD_NUM.try_into().expect("THREAD_NUM out of bounds (E010)"), align: 2, memory_index: 0, }));
            func.instruction(&Instruction::I32Const(1));
            func.instruction(&Instruction::I32Sub);
            func.instruction(&Instruction::I32Store(MemArg { offset: byte_offset::THREAD_NUM.try_into().expect("THREAD_NUM out of bounds (E011)"), align: 2, memory_index: 0, }));
        }
        //func.instruction(&Instruction::End);
        func
    }
}

fn instructions(op: &BlockOpcodeWithField, context: &Context) -> Vec<Instruction<'static>> {
    use BlockOpcodeWithField::*;
    use Instruction::*;
    let mut instructions = match op {
        looks_think => 
            if context.dbg {
                vec![Call(func_indices::DBG_ASSERT)]
            } else {
                vec![I32Const(context.target_index.try_into().expect("target index out of bounds (E002)")), Call(func_indices::LOOKS_THINK)]
            },
        looks_say =>
            if context.dbg {
                vec![Call(func_indices::DBG_LOG)]
            } else {
                vec![I32Const(context.target_index.try_into().expect("target index out of bounds (E003)")), Call(func_indices::LOOKS_SAY)]
            },
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
        | math_positive_number { NUM } => vec![F64Const(NUM.clone())],
        _ => todo!(),
    };
    if op.does_request_redraw() && !(*op == looks_say && context.dbg) {
        instructions.append(&mut vec![I32Const(byte_offset::REDRAW_REQUESTED), I32Const(1), I32Store8(MemArg { offset: 0, align: 0, memory_index: 0 })]);
    }
    instructions
}

#[derive(Debug, Clone, PartialEq)]
struct Thread {
    start: ThreadStart,
    steps: Vec<Step>,
}

impl Thread {
    fn new(start: ThreadStart, steps: Vec<Step>) -> Self {
        Thread { start, steps }
    }
    fn start(&self) -> &ThreadStart {
        &self.start
    }
    fn steps(&self) -> &Vec<Step> {
        &self.steps
    }
    fn from_hat(hat: Block, blocks: BTreeMap<String, Block>, first_func_index: u32, context: &Context) -> Self {
        let mut ops: Vec<BlockOpcodeWithField> = vec![];
        fn add_block(block: Block, blocks: &BTreeMap<String, Block>, ops: &mut Vec<BlockOpcodeWithField>) {
            match block {
                Block::Normal { block_info, .. } => {
                    for (_name, input) in block_info.inputs {
                        match input {
                            Input::Shadow(_, maybe_block, _) | Input::NoShadow(_, maybe_block) => {
                                if let Some(block) = maybe_block {
                                    match block {
                                        BlockArrayOrId::Id(id) => {
                                            if let Some(actual_block) = blocks.get(&id) {
                                                add_block(actual_block.clone(), blocks, ops);
                                            }
                                        },
                                        BlockArrayOrId::Array(arr) => {
                                            add_block(Block::Special(arr), blocks, ops);
                                        }
                                    }
                                }
                            }
                        }
                    }
                    
                    ops.push(match block_info.opcode {
                        BlockOpcode::looks_say => BlockOpcodeWithField::looks_say,
                        BlockOpcode::looks_think => BlockOpcodeWithField::looks_think,
                        BlockOpcode::operator_add => BlockOpcodeWithField::operator_add,
                        BlockOpcode::operator_subtract => BlockOpcodeWithField::operator_subtract,
                        BlockOpcode::operator_multiply => BlockOpcodeWithField::operator_multiply,
                        BlockOpcode::operator_divide => BlockOpcodeWithField::operator_divide,
                        BlockOpcode::operator_mod => BlockOpcodeWithField::operator_mod,
                        BlockOpcode::operator_round => BlockOpcodeWithField::operator_round,
                        _ => todo!(),
                    });
                    
                    if let Some(next_id) = &block_info.next {
                        if let Some(next_block) = blocks.get(next_id) {
                            add_block(next_block.clone(), blocks, ops);
                        }
                    }
                    
                },
                Block::Special(a) => match a {
                    BlockArray::NumberOrAngle(ty, value) => ops.push(match ty {
                        4 => BlockOpcodeWithField::math_number { NUM: value.clone() },
                        5 => BlockOpcodeWithField::math_positive_number { NUM: value.clone() },
                        6 => BlockOpcodeWithField::math_whole_number { NUM: value.clone() },
                        7 => BlockOpcodeWithField::math_integer { NUM: value.clone() },
                        8 => BlockOpcodeWithField::math_angle { NUM: value.clone() },
                        _ => panic!("bad project json (block array of type ({}, u32))", ty),
                    }),
                    BlockArray::ColorOrString(ty, value) => ops.push(match ty {
                        4 => BlockOpcodeWithField::math_number { NUM: value.parse().unwrap() },
                        5 => BlockOpcodeWithField::math_positive_number { NUM: value.parse().unwrap() },
                        6 => BlockOpcodeWithField::math_whole_number { NUM: value.parse().unwrap() },
                        7 => BlockOpcodeWithField::math_integer { NUM: value.parse().unwrap() },
                        8 => BlockOpcodeWithField::math_angle { NUM: value.parse().unwrap() },
                        9 => todo!(),
                        10 => BlockOpcodeWithField::math_number { NUM: value.parse().unwrap() }, // this is for testing purposes, will change later
                        _ => panic!("bad project json (block array of type ({}, string))", ty),
                    }),
                    BlockArray::Broadcast(ty, _name, _id) => match ty {
                        _ => todo!(),
                    },
                    BlockArray::VariableOrList(ty, _name, _id, _pos_x, _pos_y) => match ty {
                        _ => todo!(),
                    },
                },
            };
        }
        if let Block::Normal { block_info, .. } = &hat {
            if let Some(next_id) = &block_info.next {
                if let Some(next_block) = blocks.get(next_id) {
                    add_block(next_block.clone(), &blocks, &mut ops);
                }
            }
        }
        let start_type = if let Block::Normal { block_info, .. } = &hat {
            match block_info.opcode {
                BlockOpcode::event_whenflagclicked => ThreadStart::GreenFlag,
                _ => todo!(),
            }
        } else {
            unreachable!()
        };
        let mut steps: Vec<Step> = vec![];
        //let mut lastOpRequestedRedraw = false;
        let mut this_step_ops: Vec<BlockOpcodeWithField> = vec![];
        for op in ops {
            this_step_ops.push(op.clone());
            if op.does_request_redraw() && !(op == BlockOpcodeWithField::looks_say && context.dbg) {
                let steps_len: u32 = steps.len().try_into().expect("step count out of bounds (E004)");
                steps.push(Step::new(this_step_ops.clone(), first_func_index + steps_len));
                this_step_ops = vec![];
            }
        }
        Self::new(start_type, steps)
    }
}

pub struct WebWasmFile {
    js_string: String,
    wasm_bytes: Vec<u8>,
}

impl WebWasmFile {
    fn wasm_bytes(&self) -> &Vec<u8> {
        &self.wasm_bytes
    }
    fn js_string(&self) -> &String {
        &self.js_string
    }
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
    /* wasm funcs */
    pub const FMOD: u32 = 7;
    pub const CAST_FLOAT_BOOL: u32 = 8;
    pub const CAST_BOOL_FLOAT: u32 = 9;
    pub const CAST_BOOL_STRING: u32 = 10;
    pub const CAST_ANY_STRING: u32 = 11;
    pub const CAST_ANY_FLOAT: u32 = 12;
    pub const CAST_ANY_BOOL: u32 = 13;
    pub const CAST_FLOAT_STRING: u32 = 14;
    pub const CAST_STRING_FLOAT: u32 = 15;
    pub const CAST_STRING_BOOL: u32 = 16;
    pub const CAST_FLOAT_ANY: u32 = 17; // `any`s have to be i64s and not f64s :(
}
pub const BUILTIN_FUNCS: u32 = 18;
pub const IMPORTED_FUNCS: u32 = 7;

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

pub struct Context {
    pub target_index: u32,
    pub dbg: bool,
}

pub mod byte_offset {
    pub const REDRAW_REQUESTED: i32 = 0;
    pub const THREAD_NUM: i32 = 4;
    pub const THREADS: i32 = 8;
}

impl From<Sb3Project> for WebWasmFile {
    fn from(project: Sb3Project) -> Self {
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
        
        imports.import("dbg", "log", EntityType::Function(types::F64_NORESULT));
        imports.import("dbg", "assert", EntityType::Function(types::F64_NORESULT));
        imports.import("runtime", "looks_say", EntityType::Function(types::F64I32_NORESULT));
        imports.import("runtime", "looks_think", EntityType::Function(types::F64I32_NORESULT));
        imports.import("cast", "floattostring", EntityType::Function(types::F64_EXTERNREF));
        imports.import("cast", "stringtofloat", EntityType::Function(types::EXTERNREF_F64));
        imports.import("cast", "stringtobool", EntityType::Function(types::EXTERNREF_I32));
        
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
        
        functions.function(types::I32F64_I32I64);
        let mut float2bool_func = Function::new(vec![]);
        float2bool_func.instruction(&Instruction::I32Const(hq_value_types::BOOL64));
        float2bool_func.instruction(&Instruction::LocalGet(1));
        float2bool_func.instruction(&Instruction::F64Const(0.0));
        float2bool_func.instruction(&Instruction::F64Eq);
        float2bool_func.instruction(&Instruction::I64ExtendI32S);
        float2bool_func.instruction(&Instruction::End);
        code.function(&float2bool_func);
        
        functions.function(types::I32I64_I32F64);
        let mut bool2float_func = Function::new(vec![]);
        bool2float_func.instruction(&Instruction::I32Const(hq_value_types::FLOAT64));
        bool2float_func.instruction(&Instruction::LocalGet(1));
        bool2float_func.instruction(&Instruction::F64ConvertI64S);
        bool2float_func.instruction(&Instruction::End);
        code.function(&bool2float_func);
        
        functions.function(types::I32I64_I32I64);
        let mut bool2string_func = Function::new(vec![]);
        bool2string_func.instruction(&Instruction::I32Const(hq_value_types::EXTERN_STRING_REF64));
        bool2string_func.instruction(&Instruction::LocalGet(1));
        bool2string_func.instruction(&Instruction::End);
        code.function(&bool2string_func);
        
        functions.function(types::I32I64_I32I64);
        let mut any2string_func = Function::new(vec![]);
        any2string_func.instruction(&Instruction::LocalGet(0));
        any2string_func.instruction(&Instruction::I32Const(hq_value_types::BOOL64));
        any2string_func.instruction(&Instruction::I32Eq);
        any2string_func.instruction(&Instruction::If(BlockType::FunctionType(types::NOPARAM_I32I64)));
        any2string_func.instruction(&Instruction::LocalGet(0));
        any2string_func.instruction(&Instruction::LocalGet(1));
        any2string_func.instruction(&Instruction::Call(func_indices::CAST_BOOL_STRING));
        any2string_func.instruction(&Instruction::Else);
        any2string_func.instruction(&Instruction::LocalGet(0));
        any2string_func.instruction(&Instruction::I32Const(hq_value_types::FLOAT64));
        any2string_func.instruction(&Instruction::I32Eq);
        any2string_func.instruction(&Instruction::If(BlockType::FunctionType(types::NOPARAM_I32I64)));
        any2string_func.instruction(&Instruction::LocalGet(0));
        any2string_func.instruction(&Instruction::LocalGet(1));
        any2string_func.instruction(&Instruction::F64ReinterpretI64);
        any2string_func.instruction(&Instruction::Call(func_indices::CAST_FLOAT_STRING));
        any2string_func.instruction(&Instruction::Else);
        any2string_func.instruction(&Instruction::LocalGet(0));
        any2string_func.instruction(&Instruction::I32Const(hq_value_types::EXTERN_STRING_REF64));
        any2string_func.instruction(&Instruction::I32Eq);
        any2string_func.instruction(&Instruction::If(BlockType::FunctionType(types::NOPARAM_I32I64)));
        any2string_func.instruction(&Instruction::LocalGet(0));
        any2string_func.instruction(&Instruction::LocalGet(1));
        any2string_func.instruction(&Instruction::Else);
        any2string_func.instruction(&Instruction::Unreachable);
        any2string_func.instruction(&Instruction::End);
        any2string_func.instruction(&Instruction::End);
        any2string_func.instruction(&Instruction::End);
        any2string_func.instruction(&Instruction::End);
        code.function(&any2string_func);
        
        functions.function(types::I32I64_I32F64);
        let mut any2float_func = Function::new(vec![]);
        any2float_func.instruction(&Instruction::LocalGet(0));
        any2float_func.instruction(&Instruction::I32Const(hq_value_types::BOOL64));
        any2float_func.instruction(&Instruction::I32Eq);
        any2float_func.instruction(&Instruction::If(BlockType::FunctionType(types::NOPARAM_I32F64)));
        any2float_func.instruction(&Instruction::LocalGet(0));
        any2float_func.instruction(&Instruction::LocalGet(1));
        any2float_func.instruction(&Instruction::Call(func_indices::CAST_BOOL_FLOAT));
        any2float_func.instruction(&Instruction::Else);
        any2float_func.instruction(&Instruction::LocalGet(0));
        any2float_func.instruction(&Instruction::I32Const(hq_value_types::EXTERN_STRING_REF64));
        any2float_func.instruction(&Instruction::I32Eq);
        any2float_func.instruction(&Instruction::If(BlockType::FunctionType(types::NOPARAM_I32F64)));
        any2float_func.instruction(&Instruction::LocalGet(0));
        any2float_func.instruction(&Instruction::LocalGet(1));
        any2float_func.instruction(&Instruction::Call(func_indices::CAST_STRING_FLOAT));
        any2float_func.instruction(&Instruction::Else);
        any2float_func.instruction(&Instruction::LocalGet(0));
        any2float_func.instruction(&Instruction::I32Const(hq_value_types::FLOAT64));
        any2float_func.instruction(&Instruction::I32Eq);
        any2float_func.instruction(&Instruction::If(BlockType::FunctionType(types::NOPARAM_I32F64)));
        any2float_func.instruction(&Instruction::LocalGet(0));
        any2float_func.instruction(&Instruction::LocalGet(1));
        any2float_func.instruction(&Instruction::F64ReinterpretI64);
        any2float_func.instruction(&Instruction::Else);
        any2float_func.instruction(&Instruction::Unreachable);
        any2float_func.instruction(&Instruction::End);
        any2float_func.instruction(&Instruction::End);
        any2float_func.instruction(&Instruction::End);
        any2float_func.instruction(&Instruction::End);
        code.function(&any2float_func);
        
        functions.function(types::I32I64_I32I64);
        let mut any2bool_func = Function::new(vec![]);
        any2bool_func.instruction(&Instruction::LocalGet(0));
        any2bool_func.instruction(&Instruction::I32Const(hq_value_types::EXTERN_STRING_REF64));
        any2bool_func.instruction(&Instruction::I32Eq);
        any2bool_func.instruction(&Instruction::If(BlockType::FunctionType(types::NOPARAM_I32I64)));
        any2bool_func.instruction(&Instruction::LocalGet(0));
        any2bool_func.instruction(&Instruction::LocalGet(1));
        any2bool_func.instruction(&Instruction::Call(func_indices::CAST_STRING_BOOL));
        any2bool_func.instruction(&Instruction::Else);
        any2bool_func.instruction(&Instruction::LocalGet(0));
        any2bool_func.instruction(&Instruction::I32Const(hq_value_types::FLOAT64));
        any2bool_func.instruction(&Instruction::I32Eq);
        any2bool_func.instruction(&Instruction::If(BlockType::FunctionType(types::NOPARAM_I32I64)));
        any2bool_func.instruction(&Instruction::LocalGet(0));
        any2bool_func.instruction(&Instruction::LocalGet(1));
        any2bool_func.instruction(&Instruction::F64ReinterpretI64);
        any2bool_func.instruction(&Instruction::Call(func_indices::CAST_FLOAT_BOOL));
        any2bool_func.instruction(&Instruction::Else);
        any2bool_func.instruction(&Instruction::LocalGet(0));
        any2bool_func.instruction(&Instruction::I32Const(hq_value_types::BOOL64));
        any2bool_func.instruction(&Instruction::I32Eq);
        any2bool_func.instruction(&Instruction::If(BlockType::FunctionType(types::NOPARAM_I32I64)));
        any2bool_func.instruction(&Instruction::LocalGet(0));
        any2bool_func.instruction(&Instruction::LocalGet(1));
        any2bool_func.instruction(&Instruction::Else);
        any2bool_func.instruction(&Instruction::Unreachable);
        any2bool_func.instruction(&Instruction::End);
        any2bool_func.instruction(&Instruction::End);
        any2bool_func.instruction(&Instruction::End);
        any2bool_func.instruction(&Instruction::End);
        code.function(&any2bool_func);
        
        functions.function(types::I32F64_I32I64);
        let mut float2string_func = Function::new(vec![(1, ValType::I32)]);
        float2string_func.instruction(&Instruction::RefNull(HeapType::Extern));
        float2string_func.instruction(&Instruction::I32Const(1));
        float2string_func.instruction(&Instruction::TableGrow(table_indices::STRINGS));
        float2string_func.instruction(&Instruction::LocalTee(2));
        float2string_func.instruction(&Instruction::I32Const(-1));
        float2string_func.instruction(&Instruction::I32Eq);
        float2string_func.instruction(&Instruction::If(BlockType::Empty));
        float2string_func.instruction(&Instruction::Unreachable);
        float2string_func.instruction(&Instruction::End);
        float2string_func.instruction(&Instruction::LocalGet(2));
        float2string_func.instruction(&Instruction::LocalGet(1));
        float2string_func.instruction(&Instruction::Call(func_indices::CAST_PRIMITIVE_FLOAT_STRING));
        float2string_func.instruction(&Instruction::TableSet(table_indices::STRINGS));
        float2string_func.instruction(&Instruction::I32Const(hq_value_types::EXTERN_STRING_REF64));
        float2string_func.instruction(&Instruction::LocalGet(2));
        float2string_func.instruction(&Instruction::I64ExtendI32S);
        float2string_func.instruction(&Instruction::End);
        code.function(&float2string_func);
        
        functions.function(types::I32I64_I32F64);
        let mut string2float_func = Function::new(vec![]);
        string2float_func.instruction(&Instruction::I32Const(hq_value_types::FLOAT64));
        string2float_func.instruction(&Instruction::LocalGet(1));
        string2float_func.instruction(&Instruction::I32WrapI64);
        string2float_func.instruction(&Instruction::TableGet(table_indices::STRINGS));
        string2float_func.instruction(&Instruction::Call(func_indices::CAST_PRIMITIVE_STRING_FLOAT));
        string2float_func.instruction(&Instruction::End);
        code.function(&string2float_func);
        
        functions.function(types::I32I64_I32I64);
        let mut string2bool_func = Function::new(vec![]);
        string2bool_func.instruction(&Instruction::I32Const(hq_value_types::BOOL64));
        string2bool_func.instruction(&Instruction::LocalGet(1));
        string2bool_func.instruction(&Instruction::I32WrapI64);
        string2bool_func.instruction(&Instruction::TableGet(table_indices::STRINGS));
        string2bool_func.instruction(&Instruction::Call(func_indices::CAST_PRIMITIVE_STRING_BOOL));
        string2bool_func.instruction(&Instruction::I64ExtendI32S);
        string2bool_func.instruction(&Instruction::End);
        code.function(&string2bool_func);
        
        functions.function(types::I32F64_I32I64);
        let mut float2any_func = Function::new(vec![]);
        float2any_func.instruction(&Instruction::I32Const(hq_value_types::FLOAT64));
        float2any_func.instruction(&Instruction::LocalGet(1));
        float2any_func.instruction(&Instruction::I64ReinterpretF64);
        float2any_func.instruction(&Instruction::End);
        code.function(&float2any_func);
        
        
        let mut gf_func = Function::new(vec![]);
        let mut tick_func = Function::new(vec![(2, ValType::I32)]);
        
        /*tick_func.instruction(&Instruction::I32Const(0));
        tick_func.instruction(&Instruction::I32Const(0));
        tick_func.instruction(&Instruction::I32Store8(MemArg { offset: 0, align: 0, memory_index: 0 }));*/
        
        let mut noop_func = Function::new(vec![]);
        noop_func.instruction(&Instruction::End);
        functions.function(types::I32_NORESULT);
        code.function(&noop_func);
        
        /*let mut thread_indices: BTreeMap<ThreadStart, Vec<u32>> = BTreeMap::from([
            (ThreadStart::GreenFlag, vec![]),
        ]);*/
        let mut thread_indices: Vec<(ThreadStart, u32)> = vec![]; // (start type, first step index)
        
        let mut step_indices: Vec<u32> = vec![0];
        
        let mut step_func_count = 1u32;
        
        for (target_index, target) in project.targets.iter().enumerate() {
            for (id, block) in target.blocks.clone().iter().filter(|(_id, b)| match b.block_info() {
                Some(block_info) => block_info.top_level && matches!(block_info.opcode, BlockOpcode::event_whenflagclicked),
                None => false,
            }) {
                let context = Context { target_index: target_index.try_into().unwrap(), dbg: matches!(target.comments.clone().iter().find(|(_id, comment)| matches!(comment.block_id.clone(), Some(d) if &d == id) && comment.text.clone() == String::from("hq-dbg")), Some(_)) };
                let thread = Thread::from_hat(block.clone(), target.blocks.clone(), step_func_count, &context);
                let first_index = thread.steps()[0].index;
                thread_indices.push((thread.start().clone(), first_index));
                for (i, step) in thread.steps().iter().enumerate() {
                    let mut func = step.into_function(&context, (step.index + 1) * (i < thread.steps.len() - 1) as u32);
                    func.instruction(&Instruction::End);
                    functions.function(types::I32_NORESULT);
                    code.function(&func);
                    step_func_count += 1;
                    step_indices.push(*step.index());
                }
            }
        }
        
        let mut thread_start_counts: BTreeMap<ThreadStart, u32> = Default::default();
        
        macro_rules! func_for_thread_start {
            ($start_type:ident) => {
                match $start_type {
                    ThreadStart::GreenFlag => &mut gf_func,
                }
            }
        }
        
        for (start_type, index) in thread_indices {
            let mut func = func_for_thread_start!(start_type);
            func.instruction(&Instruction::I32Const(0));
            func.instruction(&Instruction::I32Load(MemArg {
                offset: byte_offset::THREAD_NUM.try_into().expect("THREAD_NUM out of bounds (E015)"),
                align: 2,
                memory_index: 0,
            }));
            func.instruction(&Instruction::I32Const(4));
            func.instruction(&Instruction::I32Mul);
            let thread_start_count: i32 = (*thread_start_counts.get(&start_type).unwrap_or(&0)).try_into().expect("start_type count out of bounds (E017)");
            func.instruction(&Instruction::I32Const(thread_start_count * 4));
            func.instruction(&Instruction::I32Add);
            func.instruction(&Instruction::I32Const(index.try_into().expect("step func index out of bounds (E006)")));
            func.instruction(&Instruction::I32Store(MemArg {
                offset: (byte_offset::THREADS) as u64,
                align: 2, // 2 ** 2 = 4 (bytes)
                memory_index: 0,
            }));
            
            thread_start_counts.entry(start_type).and_modify(|u| *u += 1).or_insert(1);
        }
        
        for (start_type, count) in thread_start_counts {
            let mut func = func_for_thread_start!(start_type);
            func.instruction(&Instruction::I32Const(0));
            func.instruction(&Instruction::I32Const(0));
            func.instruction(&Instruction::I32Load(MemArg {
                offset: byte_offset::THREAD_NUM.try_into().expect("THREAD_NUM out of bounds (E012)"),
                align: 2,
                memory_index: 0,
            }));
            func.instruction(&Instruction::I32Const(count.try_into().expect("thread_start count out of bounds (E014)")));
            func.instruction(&Instruction::I32Add);
            func.instruction(&Instruction::I32Store(MemArg {
                offset: byte_offset::THREAD_NUM.try_into().expect("THREAD_NUM out of bounds (E016)"),
                align: 2,
                memory_index: 0,
            }));
        }
        
        {
          tick_func.instruction(&Instruction::I32Const(0));
          tick_func.instruction(&Instruction::I32Load(MemArg {
              offset: byte_offset::THREAD_NUM.try_into().expect("THREAD_NUM out of bounds (E013)"),
              align: 2,
              memory_index: 0,
          }));
          tick_func.instruction(&Instruction::I32Const(4));
          tick_func.instruction(&Instruction::I32Mul);
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
              ty: types::I32_NORESULT,
              table: table_indices::STEP_FUNCS,
          });
          
          tick_func.instruction(&Instruction::LocalGet(0));
          tick_func.instruction(&Instruction::I32Const(4));
          tick_func.instruction(&Instruction::I32Add);
          tick_func.instruction(&Instruction::LocalTee(0));
          tick_func.instruction(&Instruction::LocalGet(1));
          tick_func.instruction(&Instruction::I32LeS);
          tick_func.instruction(&Instruction::BrIf(0));
          tick_func.instruction(&Instruction::End);
        }
        
        
        gf_func.instruction(&Instruction::End);
        functions.function(types::NOPARAM_NORESULT);
        code.function(&gf_func);
        exports.export("green_flag", ExportKind::Func, code.len() + IMPORTED_FUNCS - 1);
        
        tick_func.instruction(&Instruction::End);
        functions.function(types::NOPARAM_NORESULT);
        code.function(&tick_func);
        exports.export("tick", ExportKind::Func, code.len() + IMPORTED_FUNCS - 1);
        
        tables.table(TableType {
            element_type: RefType::FUNCREF,
            minimum: step_indices.len().try_into().expect("step indices length out of bounds (E007)"),
            maximum: Some(step_indices.len().try_into().expect("step indices length out of bounds (E008)")),
        });
        
        tables.table(TableType {
          element_type: RefType::EXTERNREF,
          minimum: 2,
          maximum: None,
        });
        
        for i in &mut step_indices {
            *i += BUILTIN_FUNCS;
        }
        let step_func_indices = Elements::Functions(&step_indices[..]);
        elements.active(Some(table_indices::STEP_FUNCS), &ConstExpr::i32_const(0), RefType::FUNCREF, step_func_indices);
        
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
        Self { js_string: format!("const assert = require('node:assert').strict;
        let last_output;
        let strings_tbl;
        conat wasm_val_to_string = (type, value_i64) =>
        const wasm_output = f64 => {{
            console.log(f64);
            last_output = f64;
        }};
        const assert_output = out => assert.equal(last_output, out);
        const targetOutput = (targetIndex, verb, text) => {{
            let targetName = {target_names:?}[targetIndex];
            console.log(`${{targetName}} ${{verb}}: ${{text}}`);
        }};
        const importObject = {{
            dbg: {{
                log: wasm_output,
                assert: assert_output,
            }},
            runtime: {{
                looks_say: (text, targetIndex) => targetOutput(targetIndex, 'says', text),
                looks_think: (text, targetIndex) => targetOutput(targetIndex, 'thinks', text),
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
            strings.set(0, 'false');
            strings.set(1, 'true');
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
            console.error('error when instantiating module: ' + e.message);
            process.exit(1);
        }});
        ", target_names=project.targets.iter().map(|t| t.name.clone()).collect::<Vec<_>>(), buf=&wasm_bytes, rr_offset=byte_offset::REDRAW_REQUESTED, threads_offset=byte_offset::THREADS, thn_offset=byte_offset::THREAD_NUM), wasm_bytes }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::sb3::tests::test_project_id;
    use std::process::{Command, Stdio};
    
    /*#[test]
    fn make_thread() {
        use BlockOpcodeWithField::*;
        let proj: Sb3Project = test_project_id("771449498").try_into().unwrap();
        let thread = Thread::from_hat(proj.targets[0].blocks.iter().filter(|(_id, b)| match b {
            Block::Normal { block_info, .. } => block_info.opcode == BlockOpcode::event_whenflagclicked,
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
        let proj: Sb3Project = test_project_id("771449498").try_into().unwrap();
        let wasm: WebWasmFile = proj.into();
        println!("{}", wasm.js_string());
        let output2 = Command::new("node")
            .arg("-e")
            .arg(format!("fs.writeFileSync('bad.wasm', Buffer.from({buf:?}), 'binary');", buf=wasm.wasm_bytes()))
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
        println!("{:}", String::from_utf8(output.stdout).expect("failed to convert stdout from utf8"));
        println!("{:}", String::from_utf8(output.stderr).expect("failed to convert stderr from utf8"));
        if !output.status.success() {
            panic!("couldn't run wasm");
        }
    }
}
