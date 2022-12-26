use crate::sb3::*;
use alloc::collections::BTreeMap;
use alloc::string::String;
use alloc::vec::Vec;
use wasm_encoder::{
    CodeSection, EntityType, ExportKind, ExportSection, Function, FunctionSection, ImportSection, Instruction, Module, TypeSection, ValType, TableSection, TableType, Elements, ElementSection, MemorySection, MemoryType, ConstExpr, MemArg,
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
    fn into_function(&self, context: &Context) -> Function {
        let locals = vec![];
        let mut func = Function::new(locals);
        for op in self.opcodes() {
            for instr in instructions(op, context) {
                func.instruction(&instr);
            }
        }
        func.instruction(&Instruction::End);
        func
    }
}

mod funcs {
    pub const DBG_LOG: u32 = 0;
    pub const DBG_ASSERT: u32 = 1;
    pub const LOOKS_SAY: u32 = 2;
    pub const LOOKS_THINK: u32 = 3;
    pub const FMOD: u32 = 4;
}
const BUILTIN_FUNCS: u32 = 5;

fn instructions(op: &BlockOpcodeWithField, context: &Context) -> Vec<Instruction<'static>> {
    use BlockOpcodeWithField::*;
    use Instruction::*;
    match op {
        looks_think => {
            if context.dbg {
                vec![Call(funcs::DBG_ASSERT)]
            } else {
                vec![I32Const(context.target_index.try_into().expect("target index out of bounds")), Call(funcs::LOOKS_THINK)]
            }
        },
        looks_say => {
            if context.dbg {
                vec![Call(funcs::DBG_LOG)]
            } else {
                vec![I32Const(context.target_index.try_into().expect("target index out of bounds")), Call(funcs::LOOKS_SAY)]
            }
        },
        operator_add => vec![F64Add],
        operator_subtract => vec![F64Sub],
        operator_divide => vec![F64Div],
        operator_multiply => vec![F64Mul],
        operator_mod => vec![Call(funcs::FMOD)],
        operator_round => vec![F64Nearest],
        math_number { NUM }
        | math_integer { NUM }
        | math_angle { NUM }
        | math_whole_number { NUM }
        | math_positive_number { NUM } => vec![F64Const(NUM.clone())],
        _ => todo!(),
    }
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
    fn from_hat(hat: Block, blocks: BTreeMap<String, Block>, first_func_index: u32) -> Self {
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
        Self::new(start_type, vec![Step::new(ops, first_func_index)])
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

mod types {
    pub const F64_NORESULT: u32 = 0;
    pub const NOPARAM_NORESULT: u32 = 1;
    #[allow(non_upper_case_globals)]
    pub const F64x2_F64: u32 = 2;
    pub const F64I32_NORESULT: u32 = 3;
}

mod tables {
    pub const STEP_FUNCS: u32 = 0;
}

struct Context {
    target_index: u32,
    dbg: bool,
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
        
        imports.import("dbg", "log", EntityType::Function(types::F64_NORESULT));
        imports.import("dbg", "assert", EntityType::Function(types::F64_NORESULT));
        imports.import("runtime", "looks_say", EntityType::Function(types::F64I32_NORESULT));
        imports.import("runtime", "looks_think", EntityType::Function(types::F64I32_NORESULT));
        
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
        
        let mut gf_func = Function::new(vec![]);
        let mut tick_func = Function::new(vec![]);
        
        let mut thread_indices: BTreeMap<ThreadStart, Vec<u32>> = BTreeMap::from([
            (ThreadStart::GreenFlag, vec![]),
        ]);
        
        let mut step_indices: Vec<u32> = vec![];
        
        let mut step_func_count = 0u32;
        
        for (target_index, target) in project.targets.iter().enumerate() {
            for (id, block) in target.blocks.clone().iter().filter(|(_id, b)| match b.block_info() {
                Some(block_info) => block_info.top_level && matches!(block_info.opcode, BlockOpcode::event_whenflagclicked),
                None => false,
            }) {
                let context = Context { target_index: target_index.try_into().unwrap(), dbg: matches!(target.comments.clone().iter().find(|(_id, comment)| matches!(comment.block_id.clone(), Some(d) if &d == id) && comment.text.clone() == String::from("hq-dbg")), Some(_)) };
                let thread = Thread::from_hat(block.clone(), target.blocks.clone(), step_func_count);
                let first_index = thread.steps()[0].index;
                thread_indices.entry(thread.start().clone()).and_modify(|v| v.push(first_index));
                for step in thread.steps() {
                    let func = step.into_function(&context);
                    functions.function(types::NOPARAM_NORESULT);
                    code.function(&func);
                    step_func_count += 1;
                    step_indices.push(*step.index());
                }
            }
        }
        
        let mut thread_count = 0;
        for (start_type, indices) in thread_indices {
            match start_type {
                ThreadStart::GreenFlag => {
                    for index in indices {
                        gf_func.instruction(&Instruction::I32Const(0));
                        gf_func.instruction(&Instruction::I32Const(index.try_into().expect("step func index out of bounds")));
                        gf_func.instruction(&Instruction::I32Store(MemArg {
                            offset: thread_count * 4,
                            align: 2, // 2 ** 2 = 4 (bytes)
                            memory_index: 0,
                        }));
                        
                        //tick_func.instruction(&Instruction::F64Const((42+thread_count) as f64));
                        //tick_func.instruction(&Instruction::Call(funcs::DBG_LOG));
                        tick_func.instruction(&Instruction::I32Const(0));
                        tick_func.instruction(&Instruction::I32Load(MemArg {
                            offset: thread_count * 4,
                            align: 2, // 2 ** 2 = 4 (bytes)
                            memory_index: 0,
                        }));
                        tick_func.instruction(&Instruction::CallIndirect {
                            ty: types::NOPARAM_NORESULT,
                            table: 0,
                        });
                        
                        thread_count += 1;
                    }
                }
            }
        }
        
        gf_func.instruction(&Instruction::End);
        functions.function(types::NOPARAM_NORESULT);
        code.function(&gf_func);
        exports.export("green_flag", ExportKind::Func, code.len() + 3);
        
        tick_func.instruction(&Instruction::End);
        functions.function(types::NOPARAM_NORESULT);
        code.function(&tick_func);
        exports.export("tick", ExportKind::Func, code.len() + 3);
        
        tables.table(TableType {
            element_type: ValType::FuncRef,
            minimum: step_indices.len().try_into().expect("step indices length out of bounds"),
            maximum: Some(step_indices.len().try_into().expect("step indices length out of bounds")),
        });
        
        for i in &mut step_indices {
            *i += BUILTIN_FUNCS;
        }
        let step_func_indices = Elements::Functions(&step_indices[..]);
        elements.active(Some(tables::STEP_FUNCS), &ConstExpr::i32_const(0), ValType::FuncRef, step_func_indices);
        
        exports.export("step_funcs", ExportKind::Table, 0);
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
        const wasm_output = f64 => {{
            console.log(f64)
            last_output = f64;
        }};
        const assert_output = f64 => assert.equal(last_output, f64);
        const targetOutput = (targetIndex, verb, text) => {{
            let targetName = {:?}[targetIndex];
            console.log(`${{targetName}} ${{verb}}: ${{text}}`)
        }};
        const importObject = {{
            dbg: {{
                log: wasm_output,
                assert: assert_output,
            }},
            runtime: {{
                looks_say: (text, targetIndex) => targetOutput(targetIndex, 'says', text),
                looks_think: (text, targetIndex) => targetOutput(targetIndex, 'thinks', text),
            }}
        }};
        const buf = new Uint8Array({:?});
        try {{
            assert(WebAssembly.validate(buf));
        }} catch {{
            console.error('invalid WASM module');
            process.exit(1);
        }}
        WebAssembly.instantiate(buf, importObject).then(({{ instance }}) => {{
            const {{ green_flag, tick }} = instance.exports;
            green_flag();
            tick();
        }}).catch((e) => {{
            console.error('error when instantiating module: ' + e.message);
            process.exit(1);
        }});
        ", project.targets.iter().map(|t| t.name.clone()).collect::<Vec<_>>(), &wasm_bytes)/*String::from("console.log(5)")*/, wasm_bytes }
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
        let output = Command::new("node")
            .arg("-e")
            .arg(wasm.js_string())
            .stdout(Stdio::inherit())
            .output()
            .expect("failed to execute process");
        println!("{:}", String::from_utf8(output.stdout).expect("failed to convert stdout from utf8"));
        println!("{:}", String::from_utf8(output.stderr).expect("failed to convert stderr from utf8"));
        if !output.status.success() {
            panic!();
        }
    }
}