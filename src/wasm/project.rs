use super::flags::Scheduler;
use super::{ExternalEnvironment, GlobalExportable, GlobalMutable, Registries};
use crate::ir::{Event, IrProject, Step, Target as IrTarget, Type as IrType};
use crate::prelude::*;
use crate::wasm::{StepFunc, StepsTable, ThreadsTable, WasmFlags};
use itertools::Itertools;
use wasm_bindgen::prelude::*;
use wasm_encoder::{
    BlockType as WasmBlockType, CodeSection, ConstExpr, ElementSection, Elements, ExportKind,
    ExportSection, Function, FunctionSection, GlobalSection, ImportSection, Instruction, MemArg,
    MemorySection, MemoryType, Module, TableSection, TypeSection, ValType,
};
use wasm_gen::wasm;

/// A respresentation of a WASM representation of a project. Cannot be created directly;
/// use `TryFrom<IrProject>`.
pub struct WasmProject {
    flags: WasmFlags,
    steps: Rc<RefCell<IndexMap<Rc<Step>, StepFunc>>>,
    /// maps an event to a list of *`step_func`* indices (NOT function indices) which are
    /// triggered by that event.
    events: BTreeMap<Event, Vec<u32>>,
    registries: Rc<Registries>,
    target_names: Vec<Box<str>>,
    environment: ExternalEnvironment,
}

impl WasmProject {
    #![expect(
        clippy::allow_attributes,
        reason = "can't use expect because false in test mode"
    )]
    #[allow(dead_code, reason = "not dead in test mode")]
    #[must_use]
    pub fn new(flags: WasmFlags, environment: ExternalEnvironment) -> Self {
        Self {
            flags,
            steps: Rc::new(RefCell::new(IndexMap::default())),
            events: BTreeMap::default(),
            environment,
            registries: Rc::new(Registries::default()),
            target_names: vec![],
        }
    }

    #[must_use]
    pub fn registries(&self) -> Rc<Registries> {
        Rc::clone(&self.registries)
    }

    #[must_use]
    pub const fn environment(&self) -> ExternalEnvironment {
        self.environment
    }

    #[must_use]
    pub const fn steps(&self) -> &Rc<RefCell<IndexMap<Rc<Step>, StepFunc>>> {
        &self.steps
    }

    /// maps a broad IR type to a WASM type
    pub fn ir_type_to_wasm(ir_type: IrType) -> HQResult<ValType> {
        let base = ir_type.base_type();
        Ok(match base {
            Some(IrType::Float) => ValType::F64,
            Some(IrType::QuasiInt) => ValType::I32,
            Some(IrType::String) => ValType::EXTERNREF,
            Some(IrType::Color) => hq_todo!("colours"), //ValType::V128 // f32x4?
            None => ValType::I64, // NaN boxed value... let's worry about colors later
            Some(_) => unreachable!(),
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
        //let mut data = DataSection::new();
        let mut globals = GlobalSection::new();

        memories.memory(MemoryType {
            minimum: 1,
            maximum: None,
            memory64: false,
            shared: false,
            page_size_log2: None,
        });

        self.registries().strings().clone().finish(&mut imports);

        // self.registries().tables().register_override::<usize>(
        //     "strings".into(),
        //     TableOptions {
        //         element_type: RefType::EXTERNREF,
        //         min: 0,
        //         // TODO: use js string imports for preknown strings
        //         max: None,
        //         init: None,
        //     },
        // )?;

        self.registries()
            .external_functions()
            .clone()
            .finish(&mut imports, self.registries().types())?;
        for step_func in self.steps().try_borrow()?.values().cloned() {
            step_func.finish(
                &mut functions,
                &mut codes,
                self.steps(),
                self.imported_func_count()?,
                self.imported_global_count()?,
            )?;
        }

        self.tick_func(&mut functions, &mut codes, &mut exports)?;

        self.finish_events(&mut functions, &mut codes, &mut exports)?;

        self.unreachable_dbg_func(&mut functions, &mut codes, &mut exports)?;

        match self.flags.scheduler {
            Scheduler::CallIndirect => {
                let step_count = self.steps().try_borrow()?.len() as u64;
                // use register_override just in case we've accidentally defined the threads table elsewhere
                let steps_table_index = self
                    .registries()
                    .tables()
                    .register_override::<StepsTable, _, _>(step_count)?;
                #[expect(
                    clippy::cast_possible_truncation,
                    reason = "step count should never get near to u32::MAX"
                )]
                let func_indices: Vec<u32> = (0..step_count)
                    .map(|i| Ok(self.imported_func_count()? + i as u32))
                    .collect::<HQResult<_>>()?;
                elements.active(
                    Some(steps_table_index),
                    &ConstExpr::i32_const(0),
                    Elements::Functions(func_indices.into()),
                );
            }
            Scheduler::TypedFuncRef => {
                self.registries()
                    .tables()
                    .register_override::<ThreadsTable, usize, _>((
                        self.registries()
                            .types()
                            .register_default((vec![ValType::I32], vec![]))?,
                        self.imported_func_count()?,
                    ))?;
                elements.declared(Elements::Functions(
                    (self.imported_func_count()?
                        ..functions.len() + self.imported_func_count()? - 2)
                        .collect(),
                ));
            }
        }

        self.registries().types().clone().finish(&mut types);

        self.registries()
            .tables()
            .clone()
            .finish(&mut tables, &mut exports);

        exports.export("memory", ExportKind::Memory, 0);
        exports.export("noop", ExportKind::Func, self.imported_func_count()?);

        self.registries().globals().clone().finish(
            &imports,
            &mut globals,
            &mut exports,
            self.imported_global_count()?,
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
            //.section(&data_count)
            .section(&codes);
        //.section(&data);

        let wasm_bytes = module.finish();

        Ok(FinishedWasm {
            wasm_bytes: wasm_bytes.into_boxed_slice(),
            target_names: self
                .target_names
                .into_iter()
                .map(core::convert::Into::into)
                .collect(),
        })
    }

    fn imported_func_count(&self) -> HQResult<u32> {
        self.registries()
            .external_functions()
            .registry()
            .try_borrow()?
            .len()
            .try_into()
            .map_err(|_| make_hq_bug!("external function map len out of bounds"))
    }

    fn imported_global_count(&self) -> HQResult<u32> {
        self.registries()
            .strings()
            .registry()
            .try_borrow()?
            .len()
            .try_into()
            .map_err(|_| make_hq_bug!("string registry len out of bounds"))
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

    fn threads_table_index<N>(&self) -> HQResult<N>
    where
        N: TryFrom<usize>,
        <N as TryFrom<usize>>::Error: fmt::Debug,
    {
        match self.flags.scheduler {
            Scheduler::TypedFuncRef => self.registries().tables().register::<ThreadsTable, _>(),
            Scheduler::CallIndirect => hq_bug!(
                "tried to access threads_table_index outside of `WasmProject::Finish` when the scheduler is not TypedFuncRef"
            ),
        }
    }

    fn steps_table_index<N>(&self) -> HQResult<N>
    where
        N: TryFrom<usize>,
        <N as TryFrom<usize>>::Error: fmt::Debug,
    {
        match self.flags.scheduler {
            Scheduler::CallIndirect => self.registries().tables().register::<StepsTable, _>(),
            Scheduler::TypedFuncRef => hq_bug!(
                "tried to access steps_table_index outside of `WasmProject::Finish` when the scheduler is not CallIndirect"
            ),
        }
    }

    fn finish_event(
        &self,
        export_name: &str,
        indices: &[u32],
        funcs: &mut FunctionSection,
        codes: &mut CodeSection,
        exports: &mut ExportSection,
    ) -> HQResult<()> {
        let mut func = Function::new(vec![]);

        let threads_count = self.registries().globals().register(
            "threads_count".into(),
            (
                ValType::I32,
                ConstExpr::i32_const(0),
                GlobalMutable(true),
                GlobalExportable(true),
            ),
        )?;

        let instrs = indices
            .iter()
            .enumerate()
            .map(|(position, &i)| {
                crate::log(
                    format!(
                        "event step idx: {}; func idx: {}",
                        i,
                        i + self.imported_func_count()?
                    )
                    .as_str(),
                );
                Ok(match self.flags.scheduler {
                    Scheduler::TypedFuncRef => wasm![
                        RefFunc(i + self.imported_func_count()?),
                        I32Const(1),
                        TableGrow(self.threads_table_index()?),
                        Drop,
                    ],
                    Scheduler::CallIndirect => wasm![
                        #LazyGlobalGet(threads_count),
                        I32Const(4),
                        I32Mul,
                        I32Const(
                            i.try_into()
                                .map_err(|_| make_hq_bug!("step index out of bounds"))?
                        ),
                        I32Store(MemArg {
                            offset: position as u64 * 4,
                            align: 2,
                            memory_index: 0,
                        }),
                    ],
                })
            })
            .flatten_ok()
            .collect::<HQResult<Vec<_>>>()?;

        for instruction in instrs {
            func.instruction(&instruction.eval(
                self.steps(),
                self.imported_func_count()?,
                self.imported_global_count()?,
            )?);
        }
        for instruction in wasm![
            #LazyGlobalGet(threads_count),
            I32Const(
                i32::try_from(indices.len())
                    .map_err(|_| make_hq_bug!("indices len out of bounds"))?
            ),
            I32Add,
            #LazyGlobalSet(threads_count)
        ] {
            func.instruction(&instruction.eval(
                self.steps(),
                self.imported_func_count()?,
                self.imported_global_count()?,
            )?);
        }
        func.instruction(&Instruction::End);

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
    ) -> HQResult<()> {
        for (event, indices) in &self.events {
            self.finish_event(
                match event {
                    Event::FlagCLicked => "flag_clicked",
                },
                indices,
                funcs,
                codes,
                exports,
            )?;
        }

        Ok(())
    }

    fn tick_func(
        &self,
        funcs: &mut FunctionSection,
        codes: &mut CodeSection,
        exports: &mut ExportSection,
    ) -> HQResult<()> {
        let mut tick_func = Function::new(vec![(2, ValType::I32)]);

        let step_func_ty = self
            .registries()
            .types()
            .register_default((vec![ValType::I32], vec![]))?;

        let threads_count = self.registries().globals().register(
            "threads_count".into(),
            (
                ValType::I32,
                ConstExpr::i32_const(0),
                GlobalMutable(true),
                GlobalExportable(true),
            ),
        )?;

        let instructions = match self.flags.scheduler {
            crate::wasm::flags::Scheduler::CallIndirect => wasm![
                // For call_indirect: read step indices from linear memory and call_indirect
                #LazyGlobalGet(threads_count),
                LocalTee(1),
                I32Eqz,
                BrIf(0),
                Loop(WasmBlockType::Empty),
                LocalGet(0),
                LocalGet(0), // thread index
                I32Const(4), // 4 bytes per index (i32)
                I32Mul,
                I32Load(MemArg {
                    offset: 0,
                    align: 2,
                    memory_index: 0,
                }), // load step index from memory
                CallIndirect {
                    type_index: step_func_ty,
                    table_index: self.steps_table_index()?,
                },
                LocalGet(0),
                I32Const(1),
                I32Add,
                LocalTee(0),
                LocalGet(1),
                I32LtS,
                BrIf(0),
                End,
            ],
            Scheduler::TypedFuncRef => wasm![
                TableSize(self.threads_table_index()?),
                LocalTee(1),
                I32Eqz,
                BrIf(0),
                Loop(WasmBlockType::Empty),
                LocalGet(0),
                LocalGet(0),
                TableGet(self.threads_table_index()?),
                CallRef(step_func_ty),
                LocalGet(0),
                I32Const(1),
                I32Add,
                LocalTee(0),
                LocalGet(1),
                I32LtS,
                BrIf(0),
                End,
            ],
        };
        for instr in instructions {
            tick_func.instruction(&instr.eval(
                self.steps(),
                self.imported_func_count()?,
                self.imported_global_count()?,
            )?);
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

    pub fn from_ir(ir_project: &Rc<IrProject>, flags: WasmFlags) -> HQResult<Self> {
        let steps: Rc<RefCell<IndexMap<Rc<Step>, StepFunc>>> =
            Rc::new(RefCell::new(IndexMap::default()));
        let registries = Rc::new(Registries::default());
        let mut events: BTreeMap<Event, Vec<u32>> = BTreeMap::default();
        StepFunc::compile_step(
            Step::new_empty(
                &Rc::downgrade(ir_project),
                true,
                Rc::new(IrTarget::new(
                    false,
                    BTreeMap::default(),
                    Weak::new(),
                    RefCell::new(BTreeMap::default()),
                    0,
                    Box::new([]),
                )),
            )?,
            &steps,
            Rc::clone(&registries),
            flags,
        )?;
        // compile every step
        for step in ir_project.steps().try_borrow()?.iter() {
            StepFunc::compile_step(Rc::clone(step), &steps, Rc::clone(&registries), flags)?;
        }
        // add thread event handlers for them
        for thread in ir_project.threads().try_borrow()?.iter() {
            events.entry(thread.event()).or_default().push(
                u32::try_from(
                    steps
                        .try_borrow()?
                        .get_index_of(thread.first_step())
                        .ok_or_else(|| {
                            make_hq_bug!("Thread's first_step wasn't found in Thread::steps()")
                        })?,
                )
                .map_err(|_| make_hq_bug!("step func index out of bounds"))?,
            );
        }
        Ok(Self {
            flags,
            steps,
            events,
            registries,
            environment: ExternalEnvironment::WebBrowser,
            target_names: ir_project.targets().try_borrow()?.keys().cloned().collect(),
        })
    }
}

#[wasm_bindgen]
#[derive(Clone)]
pub struct FinishedWasm {
    #[wasm_bindgen(getter_with_clone)]
    pub wasm_bytes: Box<[u8]>,
    #[wasm_bindgen(getter_with_clone)]
    pub target_names: Vec<String>,
}

#[cfg(test)]
mod tests {
    use super::{Registries, WasmProject};
    use crate::ir::{IrProject, Step, Target as IrTarget};
    use crate::prelude::*;
    use crate::wasm::{ExternalEnvironment, StepFunc, WasmFlags, flags::all_wasm_features};

    #[test]
    fn empty_project_is_valid_wasm() {
        let registries = Rc::new(Registries::default());
        let project = Rc::new(IrProject::new(BTreeMap::default()));
        let steps = Rc::new(RefCell::new(IndexMap::default()));
        StepFunc::compile_step(
            Step::new_empty(
                &Rc::downgrade(&project),
                true,
                Rc::new(IrTarget::new(
                    false,
                    BTreeMap::default(),
                    Weak::new(),
                    RefCell::new(BTreeMap::default()),
                    0,
                    Box::new([]),
                )),
            )
            .unwrap(),
            &steps,
            Rc::clone(&registries),
            WasmFlags::new(all_wasm_features()),
        )
        .unwrap();
        let project = WasmProject {
            flags: WasmFlags::new(all_wasm_features()),
            steps,
            events: BTreeMap::new(),
            environment: ExternalEnvironment::WebBrowser,
            registries,
            target_names: vec![],
        };
        let wasm_bytes = project.finish().unwrap().wasm_bytes;
        if let Err(err) = wasmparser::validate(&wasm_bytes) {
            panic!(
                "wasmparser error: {:?}\nwasm:\n{}",
                err,
                wasmprinter::print_bytes(wasm_bytes).unwrap()
            )
        }
    }
}
