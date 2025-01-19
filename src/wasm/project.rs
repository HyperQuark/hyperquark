use super::{ExternalEnvironment, Registries};
use crate::ir::{Event, IrProject, Step, Type as IrType};
use crate::prelude::*;
use crate::wasm::{StepFunc, WasmFlags};
use wasm_bindgen::prelude::*;
use wasm_encoder::{
    BlockType as WasmBlockType, CodeSection, ConstExpr, DataCountSection, DataSection,
    ElementSection, Elements, ExportKind, ExportSection, Function, FunctionSection, ImportSection,
    Instruction, MemArg, MemorySection, MemoryType, Module, RefType, TableSection, TableType,
    TypeSection, ValType,
};

pub mod byte_offset {
    pub const REDRAW_REQUESTED: i32 = 0;
    pub const THREAD_NUM: i32 = 4;
    pub const THREADS: i32 = 8;
}

/// A respresentation of a WASM representation of a project. Cannot be created directly;
/// use `TryFrom<IrProject>`.
pub struct WasmProject {
    flags: WasmFlags,
    step_funcs: Box<[StepFunc]>,
    /// maps an event to a list of *step_func* indices (NOT function indices) which are
    /// triggered by that event.
    events: BTreeMap<Event, Vec<u32>>,
    registries: Rc<Registries>,
    environment: ExternalEnvironment,
}

impl WasmProject {
    pub fn new(flags: WasmFlags, environment: ExternalEnvironment) -> Self {
        WasmProject {
            flags,
            step_funcs: Box::new([]),
            events: Default::default(),
            environment,
            registries: Rc::new(Registries::default()),
        }
    }

    pub fn registries(&self) -> Rc<Registries> {
        Rc::clone(&self.registries)
    }

    pub fn environment(&self) -> ExternalEnvironment {
        self.environment
    }

    pub fn step_funcs(&self) -> &[StepFunc] {
        self.step_funcs.borrow()
    }

    /// maps a broad IR type to a WASM type
    pub fn ir_type_to_wasm(ir_type: IrType) -> HQResult<ValType> {
        Ok(if IrType::Float.contains(ir_type) {
            ValType::F64
        } else if IrType::QuasiInt.contains(ir_type) {
            ValType::I32
        } else if IrType::String.contains(ir_type) {
            ValType::EXTERNREF
        } else if IrType::Color.contains(ir_type) {
            hq_todo!() //ValType::V128 // f32x4
        } else {
            ValType::I64 // NaN boxed value... let's worry about colors later
        })
    }

    pub fn finish(self) -> HQResult<FinishedWasm> {
        let mut module = Module::new();

        let mut memories = MemorySection::new();
        let mut imports = ImportSection::new();
        let mut types = TypeSection::new();
        let mut functions = FunctionSection::new();
        let mut codes = CodeSection::new();
        let mut tables = TableSection::new();
        let mut exports = ExportSection::new();
        let mut elements = ElementSection::new();
        let mut data = DataSection::new();

        memories.memory(MemoryType {
            minimum: 1,
            maximum: None,
            memory64: false,
            shared: false,
            page_size_log2: None,
        });

        let step_indices = (self.imported_func_count()?
            ..(u32::try_from(self.step_funcs().len())
                .map_err(|_| make_hq_bug!("step_funcs length out of bounds"))?
                + self.imported_func_count()?))
            .collect::<Vec<_>>();
        let step_func_table_idx = self.registries().tables().register(
            "step_funcs".into(),
            (
                RefType::FUNCREF,
                u64::try_from(step_indices.len())
                    .map_err(|_| make_hq_bug!("step indices length out of bounds"))?,
            ),
        )?;
        let step_func_indices = Elements::Functions(step_indices.into());
        elements.active(
            Some(step_func_table_idx),
            &ConstExpr::i32_const(0),
            step_func_indices,
        );

        self.registries()
            .external_functions()
            .clone()
            .finish(&mut imports, self.registries().types())?;
        for step_func in self.step_funcs().iter().cloned() {
            step_func.finish(&mut functions, &mut codes)?;
        }

        self.tick_func(&mut functions, &mut codes, &mut exports)?;

        self.finish_events(&mut functions, &mut codes, &mut exports, &mut data)?;

        self.unreachable_dbg_func(&mut functions, &mut codes, &mut exports)?;

        self.registries().types().clone().finish(&mut types);

        self.registries().tables().clone().finish(&mut tables);

        let data_count = DataCountSection { count: data.len() };

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
            .section(&data_count)
            .section(&codes)
            .section(&data);

        let wasm_bytes = module.finish();

        Ok(FinishedWasm {
            wasm_bytes: wasm_bytes.into_boxed_slice(),
            strings: self.registries().strings().clone().finish(),
        })
    }

    fn imported_func_count(&self) -> HQResult<u32> {
        self.registries()
            .external_functions()
            .registry()
            .borrow()
            .len()
            .try_into()
            .map_err(|_| make_hq_bug!("external function map len out of bounds"))
    }

    fn compile_step(
        step: Rc<Step>,
        steps: &RefCell<IndexMap<Rc<Step>, StepFunc>>,
        registries: Rc<Registries>,
        flags: WasmFlags,
    ) -> HQResult<()> {
        if steps.borrow().contains_key(&step) {
            return Ok(());
        }
        let step_func = StepFunc::new(registries, flags);
        let mut instrs = vec![];
        let mut type_stack = vec![];
        for opcode in step.opcodes() {
            let inputs = type_stack
                .splice((type_stack.len() - opcode.acceptable_inputs().len()).., [])
                .collect();
            instrs.append(&mut opcode.wasm(&step_func, Rc::clone(&inputs))?);
            if let Some(output) = opcode.output_type(inputs)? {
                type_stack.push(output);
            }
        }
        step_func.add_instructions(instrs);
        steps.borrow_mut().insert(step, step_func);
        Ok(())
    }

    fn unreachable_dbg_func(
        &self,
        functions: &mut FunctionSection,
        codes: &mut CodeSection,
        exports: &mut ExportSection,
    ) -> HQResult<()> {
        let mut func = Function::new(vec![]);
        func.instruction(&Instruction::Unreachable);
        func.instruction(&Instruction::End);
        codes.function(&func);
        functions.function(
            self.registries()
                .types()
                .register_default((vec![], vec![]))?,
        );
        exports.export(
            "unreachable_dbg",
            ExportKind::Func,
            self.imported_func_count()? + functions.len() - 1,
        );

        Ok(())
    }

    fn finish_event(
        &self,
        export_name: &str,
        indices: &[u32],
        funcs: &mut FunctionSection,
        codes: &mut CodeSection,
        exports: &mut ExportSection,
        data: &mut DataSection,
    ) -> HQResult<()> {
        data.passive(Vec::from(unsafe {
            // I don't like using unsafe code but a random person on stackoverflow claims that it's ok,
            // therefore it must be fine. Right? https://stackoverflow.com/a/29042896
            // TODO: can we run miri or something on this to make sure it's actually safe? Not that I
            // don't trust that person on SO, but it's best to be sure.
            core::slice::from_raw_parts(
                indices.as_ptr() as *const u8,
                indices.len() * core::mem::size_of::<i32>(),
            )
        }));

        let mut func = Function::new(vec![]);

        let instrs = vec![
            // first, add the step indices into memory
            Instruction::I32Const(0), // [i32]
            Instruction::I32Load(MemArg {
                offset: byte_offset::THREAD_NUM
                    .try_into()
                    .map_err(|_| make_hq_bug!("THREAD_NUM out of bounds"))?,
                align: 2,
                memory_index: 0,
            }), // [i32]
            Instruction::I32Const(WasmProject::THREAD_BYTE_LEN), // [i32, i32]
            Instruction::I32Mul,      // [i32]
            Instruction::I32Const(byte_offset::THREADS), // [i32, i32]
            Instruction::I32Add,      // memory offset; [i32]
            Instruction::I32Const(0), // offset in data segment; [i32, i32]
            Instruction::I32Const(
                i32::try_from(indices.len())
                    .map_err(|_| make_hq_bug!("start_type count out of bounds"))?
                    * WasmProject::THREAD_BYTE_LEN,
            ), // segment length; [i32, i32, i32]
            Instruction::MemoryInit {
                mem: 0,
                data_index: data.len() - 1,
            }, // []
            // then, increment the number of active threads
            Instruction::I32Const(0), // stack: [i32]
            Instruction::I32Const(0), // stack: [i32, i32]
            Instruction::I32Load(MemArg {
                offset: byte_offset::THREAD_NUM
                    .try_into()
                    .map_err(|_| make_hq_bug!("THREAD_NUM out of bounds"))?,
                align: 2,
                memory_index: 0,
            }), // [i32, i32]
            Instruction::I32Const(
                indices
                    .len()
                    .try_into()
                    .map_err(|_| make_hq_bug!("indices len out of bounds"))?,
            ), // [i32, i32, i32]
            Instruction::I32Add,      // [i32, i32]
            Instruction::I32Store(MemArg {
                offset: byte_offset::THREAD_NUM
                    .try_into()
                    .map_err(|_| make_hq_bug!("THREAD_NUM out of bounds"))?,
                align: 2,
                memory_index: 0,
            }), // []
            Instruction::End,
        ];

        for instruction in instrs {
            func.instruction(&instruction);
        }

        funcs.function(
            self.registries()
                .types()
                .register_default((vec![], vec![]))?,
        );
        codes.function(&func);
        exports.export(
            export_name,
            ExportKind::Func,
            self.imported_func_count()? + funcs.len() - 1,
        );

        Ok(())
    }

    fn finish_events(
        &self,
        funcs: &mut FunctionSection,
        codes: &mut CodeSection,
        exports: &mut ExportSection,
        data: &mut DataSection,
    ) -> HQResult<()> {
        for (event, indices) in self.events.iter() {
            self.finish_event(
                match event {
                    Event::FlagCLicked => "flag_clicked",
                },
                indices,
                funcs,
                codes,
                exports,
                data,
            )?;
        }

        Ok(())
    }

    const THREAD_BYTE_LEN: i32 = 4;

    fn tick_func(
        &self,
        funcs: &mut FunctionSection,
        codes: &mut CodeSection,
        exports: &mut ExportSection,
    ) -> HQResult<()> {
        let mut tick_func = Function::new(vec![(2, ValType::I32)]);

        let instructions = vec![
            Instruction::I32Const(0),
            Instruction::I32Load(MemArg {
                offset: byte_offset::THREAD_NUM
                    .try_into()
                    .map_err(|_| make_hq_bug!("THREAD_NUM out of bounds"))?,
                align: 2,
                memory_index: 0,
            }),
            Instruction::LocalTee(1),
            Instruction::I32Eqz,
            Instruction::BrIf(0),
            Instruction::LocalGet(1),
            Instruction::I32Const(WasmProject::THREAD_BYTE_LEN),
            Instruction::I32Mul,
            Instruction::I32Const(WasmProject::THREAD_BYTE_LEN),
            Instruction::I32Sub,
            Instruction::LocalSet(1),
            Instruction::Loop(WasmBlockType::Empty),
            Instruction::LocalGet(0),
            Instruction::LocalGet(0),
            Instruction::I32Load(MemArg {
                offset: /*(byte_offset::VARS as usize
                    + VAR_INFO_LEN as usize * project.vars.borrow().len()
                    + usize::try_from(SPRITE_INFO_LEN).map_err(|_| make_hq_bug!(""))?
                        * (project.target_names.len() - 1))
                    .try_into()
                    .map_err(|_| make_hq_bug!("i32.store offset out of bounds"))?*/
                    byte_offset::THREADS.try_into().map_err(|_| make_hq_bug!("i32.store offset out of bounds"))?,
                align: 2, // 2 ** 2 = 4 (bytes)
                memory_index: 0,
            }),
            Instruction::CallIndirect {
                type_index: self
                    .registries()
                    .types()
                    .register_default((vec![ValType::I32], vec![ValType::I32]))?,
                table_index: self
                    .registries()
                    .tables()
                    .register("step_funcs".into(), (RefType::FUNCREF, 0))?,
            },
            Instruction::If(WasmBlockType::Empty),
            Instruction::LocalGet(0),
            Instruction::I32Const(WasmProject::THREAD_BYTE_LEN),
            Instruction::I32Add,
            Instruction::LocalSet(0),
            Instruction::Else,
            Instruction::LocalGet(1),
            Instruction::I32Const(WasmProject::THREAD_BYTE_LEN),
            Instruction::I32Sub,
            Instruction::LocalSet(1),
            Instruction::End,
            Instruction::LocalGet(0),
            Instruction::LocalGet(1),
            Instruction::I32LeS,
            Instruction::BrIf(0),
            Instruction::End,
        ];
        for instr in instructions {
            tick_func.instruction(&instr);
        }
        tick_func.instruction(&Instruction::End);
        funcs.function(
            self.registries()
                .types()
                .register_default((vec![], vec![]))?,
        );
        codes.function(&tick_func);
        exports.export(
            "tick",
            ExportKind::Func,
            funcs.len() + self.imported_func_count()? - 1,
        );
        Ok(())
    }

    pub fn from_ir(ir_project: Rc<IrProject>, flags: WasmFlags) -> HQResult<WasmProject> {
        let steps: RefCell<IndexMap<Rc<Step>, StepFunc>> = Default::default();
        let registries = Rc::new(Registries::default());
        let mut events: BTreeMap<Event, Vec<u32>> = Default::default();
        WasmProject::compile_step(
            Rc::new(Step::new_empty()),
            &steps,
            Rc::clone(&registries),
            flags,
        )?;
        for thread in ir_project.threads().borrow().iter() {
            let step = thread.first_step().get_rc();
            WasmProject::compile_step(step, &steps, Rc::clone(&registries), flags)?;
            events.entry(thread.event()).or_default().push(
                u32::try_from(
                    ir_project
                        .steps()
                        .borrow()
                        .get_index_of(&thread.first_step().get_rc())
                        .ok_or(make_hq_bug!(
                            "Thread's first_step wasn't found in Thread::steps()"
                        ))?,
                )
                .map_err(|_| make_hq_bug!("step func index out of bounds"))?
                    + 1, // we add 1 to account for the noop step
            );
        }
        Ok(WasmProject {
            flags,
            step_funcs: steps.take().values().cloned().collect(),
            events,
            registries,
            environment: ExternalEnvironment::WebBrowser,
        })
    }
}

#[wasm_bindgen]
#[derive(Clone)]
pub struct FinishedWasm {
    #[wasm_bindgen(getter_with_clone)]
    pub wasm_bytes: Box<[u8]>,
    #[wasm_bindgen(getter_with_clone)]
    pub strings: Vec<String>,
}

#[cfg(test)]
mod tests {
    use wasm_encoder::Instruction;

    use super::{Registries, WasmProject};
    use crate::ir::Event;
    use crate::prelude::*;
    use crate::wasm::{ExternalEnvironment, StepFunc};

    #[test]
    fn empty_project_is_valid_wasm() {
        let proj = WasmProject::new(Default::default(), ExternalEnvironment::WebBrowser);
        let wasm_bytes = proj.finish().unwrap().wasm_bytes;
        wasmparser::validate(&wasm_bytes).unwrap();
    }

    #[test]
    fn project_with_one_empty_step_is_valid_wasm() {
        let registries = Rc::new(Registries::default());
        let step_func = StepFunc::new(Rc::clone(&registries), Default::default());
        step_func.add_instructions(vec![Instruction::I32Const(0)]); // this is handled by compile_step in a non-test environment
        let project = WasmProject {
            flags: Default::default(),
            step_funcs: Box::new([step_func]),
            events: BTreeMap::from_iter(vec![(Event::FlagCLicked, vec![0])].into_iter()),
            environment: ExternalEnvironment::WebBrowser,
            registries,
        };
        let wasm_bytes = project.finish().unwrap().wasm_bytes;
        wasmparser::validate(&wasm_bytes).unwrap();
    }
}
