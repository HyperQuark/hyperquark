use super::{ExternalEnvironment, ExternalFunctionMap};
use crate::ir::{Event, IrProject, Step, Type as IrType};
use crate::prelude::*;
use crate::wasm::{StepFunc, TypeRegistry, WasmFlags};
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

mod table_indices {
    pub const STEP_FUNCS: u32 = 0;
    pub const STRINGS: u32 = 1;
}

/// A respresentation of a WASM representation of a project. Cannot be created directly;
/// use `TryFrom<IrProject>`.
pub struct WasmProject {
    flags: WasmFlags,
    step_funcs: Box<[StepFunc]>,
    /// maps an event to a list of *step_func* indices (NOT function indices) which are
    /// triggered by that event.
    events: BTreeMap<Event, Vec<u32>>,
    type_registry: Rc<TypeRegistry>,
    external_functions: Rc<ExternalFunctionMap>,
    environment: ExternalEnvironment,
}

impl WasmProject {
    pub fn new(flags: WasmFlags, environment: ExternalEnvironment) -> Self {
        WasmProject {
            flags,
            step_funcs: Box::new([]),
            events: Default::default(),
            environment,
            type_registry: Rc::new(TypeRegistry::new()),
            external_functions: Rc::new(ExternalFunctionMap::new()),
        }
    }

    pub fn type_registry(&self) -> Rc<TypeRegistry> {
        Rc::clone(&self.type_registry)
    }

    pub fn external_functions(&self) -> Rc<ExternalFunctionMap> {
        Rc::clone(&self.external_functions)
    }

    pub fn flags(&self) -> &WasmFlags {
        &self.flags
    }

    pub fn environment(&self) -> ExternalEnvironment {
        self.environment
    }

    pub fn step_funcs(&self) -> &[StepFunc] {
        self.step_funcs.borrow()
    }

    /// maps a broad IR type to a WASM type
    pub fn ir_type_to_wasm(&self, ir_type: IrType) -> HQResult<ValType> {
        Ok(if IrType::Float.contains(ir_type) {
            ValType::F64
        } else if IrType::QuasiInt.contains(ir_type) {
            ValType::I64
        } else if IrType::String.contains(ir_type) {
            ValType::EXTERNREF
        } else if IrType::Color.contains(ir_type) {
            hq_todo!() //ValType::V128 // f32x4
        } else {
            ValType::I64 // NaN boxed value... let's worry about colors later
        })
    }

    pub fn finish(self) -> HQResult<Vec<u8>> {
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

        tables.table(TableType {
            element_type: RefType::FUNCREF,
            minimum: self
                .step_funcs()
                .len()
                .try_into()
                .map_err(|_| make_hq_bug!("step_funcs length out of bounds"))?,
            maximum: Some(
                self.step_funcs()
                    .len()
                    .try_into()
                    .map_err(|_| make_hq_bug!("step_funcs length out of bounds"))?,
            ),
            table64: false,
            shared: false,
        });

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
        let step_func_indices = Elements::Functions(step_indices.into());
        elements.active(
            Some(table_indices::STEP_FUNCS),
            &ConstExpr::i32_const(0),
            step_func_indices,
        );

        Rc::unwrap_or_clone(self.external_functions())
            .finish(&mut imports, self.type_registry())?;
        for step_func in self.step_funcs().iter().cloned() {
            step_func.finish(&mut functions, &mut codes)?;
        }

        self.tick_func(&mut functions, &mut codes, &mut exports)?;

        self.finish_events(&mut functions, &mut codes, &mut exports, &mut data)?;

        Rc::unwrap_or_clone(self.type_registry()).finish(&mut types);

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

        Ok(wasm_bytes)
    }

    fn imported_func_count(&self) -> HQResult<u32> {
        self.external_functions()
            .get_map()
            .borrow()
            .len()
            .try_into()
            .map_err(|_| make_hq_bug!("external function map len out of bounds"))
    }

    fn compile_step(
        step: Rc<Step>,
        steps: &RefCell<IndexMap<Rc<Step>, StepFunc>>,
        type_registry: Rc<TypeRegistry>,
        external_funcs: Rc<ExternalFunctionMap>,
    ) -> HQResult<()> {
        if steps.borrow().contains_key(&step) {
            return Ok(());
        }
        let step_func = StepFunc::new(type_registry, external_funcs);
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
        instrs.push(Instruction::I32Const(0)); // TEMPORARY
        step_func.add_instructions(instrs);
        steps.borrow_mut().insert(step, step_func);
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

        funcs.function(self.type_registry().type_index(vec![], vec![])?);
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
                    .type_registry()
                    .type_index(vec![ValType::I32], vec![ValType::I32])?,
                table_index: table_indices::STEP_FUNCS,
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
        funcs.function(self.type_registry().type_index(vec![], vec![])?);
        codes.function(&tick_func);
        exports.export(
            "tick",
            ExportKind::Func,
            funcs.len() + self.imported_func_count()? - 1,
        );
        Ok(())
    }
}

impl TryFrom<Rc<IrProject>> for WasmProject {
    type Error = HQError;

    fn try_from(ir_project: Rc<IrProject>) -> HQResult<WasmProject> {
        let steps: RefCell<IndexMap<Rc<Step>, StepFunc>> = Default::default();
        let type_registry = Rc::new(TypeRegistry::new());
        let external_functions = Rc::new(ExternalFunctionMap::new());
        let mut events: BTreeMap<Event, Vec<u32>> = Default::default();
        WasmProject::compile_step(
            Rc::new(Step::new_empty()),
            &steps,
            Rc::clone(&type_registry),
            Rc::clone(&external_functions),
        )?;
        for thread in ir_project.threads().borrow().iter() {
            let step = thread.first_step().get_rc();
            WasmProject::compile_step(
                step,
                &steps,
                Rc::clone(&type_registry),
                Rc::clone(&external_functions),
            )?;
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
            flags: Default::default(),
            step_funcs: steps.take().values().cloned().collect(),
            events,
            type_registry,
            external_functions,
            environment: ExternalEnvironment::WebBrowser,
        })
    }
}

#[cfg(test)]
mod tests {
    use wasm_encoder::Instruction;

    use super::WasmProject;
    use crate::ir::Event;
    use crate::prelude::*;
    use crate::wasm::{ExternalEnvironment, ExternalFunctionMap, StepFunc, TypeRegistry};

    #[test]
    fn empty_project_is_valid_wasm() {
        let proj = WasmProject::new(Default::default(), ExternalEnvironment::WebBrowser);
        let wasm_bytes = proj.finish().unwrap();
        wasmparser::validate(&wasm_bytes).unwrap();
    }

    #[test]
    fn project_with_one_empty_step_is_valid_wasm() {
        let types = Rc::new(TypeRegistry::new());
        let external_funcs = Rc::new(ExternalFunctionMap::new());
        let step_func = StepFunc::new(Rc::clone(&types), Rc::clone(&external_funcs));
        step_func.add_instructions(vec![Instruction::I32Const(0)]); // this is handled by compile_step in a non-test environment
        let project = WasmProject {
            flags: Default::default(),
            step_funcs: Box::new([step_func]),
            events: BTreeMap::from_iter(vec![(Event::FlagCLicked, vec![0])].into_iter()),
            environment: ExternalEnvironment::WebBrowser,
            type_registry: types,
            external_functions: external_funcs,
        };
        let wasm_bytes = project.finish().unwrap();
        wasmparser::validate(&wasm_bytes).unwrap();
    }
}
