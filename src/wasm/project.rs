use super::{ExternalEnvironment, GlobalExportable, GlobalMutable, Registries};
use crate::ir::{Event, IrProject, Step, Type as IrType};
use crate::prelude::*;
use crate::wasm::{StepFunc, WasmFlags};
use itertools::Itertools;
use wasm_bindgen::prelude::*;
use wasm_encoder::{
    BlockType as WasmBlockType, CodeSection, ConstExpr, ElementSection, Elements, ExportKind,
    ExportSection, Function, FunctionSection, GlobalSection, HeapType, ImportSection, Instruction,
    MemorySection, MemoryType, Module, RefType, TableSection, TypeSection, ValType,
};
use wasm_gen::wasm;

#[allow(dead_code)]
pub mod byte_offset {
    pub const REDRAW_REQUESTED: i32 = 0;
    pub const THREAD_NUM: i32 = 4;
    pub const THREADS: i32 = 8;
}

/// A respresentation of a WASM representation of a project. Cannot be created directly;
/// use `TryFrom<IrProject>`.
pub struct WasmProject {
    #[allow(dead_code)]
    flags: WasmFlags,
    steps: Rc<RefCell<IndexMap<Rc<Step>, StepFunc>>>,
    /// maps an event to a list of *step_func* indices (NOT function indices) which are
    /// triggered by that event.
    events: BTreeMap<Event, Vec<u32>>,
    registries: Rc<Registries>,
    target_names: Vec<Box<str>>,
    #[allow(dead_code)]
    environment: ExternalEnvironment,
}

impl WasmProject {
    #[allow(dead_code)]
    pub fn new(flags: WasmFlags, environment: ExternalEnvironment) -> Self {
        WasmProject {
            flags,
            steps: Default::default(),
            events: Default::default(),
            environment,
            registries: Rc::new(Registries::default()),
            target_names: vec![],
        }
    }

    pub fn registries(&self) -> Rc<Registries> {
        Rc::clone(&self.registries)
    }

    #[allow(dead_code)]
    pub fn environment(&self) -> ExternalEnvironment {
        self.environment
    }

    pub fn steps(&self) -> Rc<RefCell<IndexMap<Rc<Step>, StepFunc>>> {
        Rc::clone(&self.steps)
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

        let strings = self.registries().strings().clone().finish();

        self.registries().tables().register::<usize>(
            "strings".into(),
            (
                RefType::EXTERNREF,
                strings
                    .len()
                    .try_into()
                    .map_err(|_| make_hq_bug!("strings length out of bounds"))?,
                // TODO: use js string imports for preknown strings
                None,
            ),
        )?;

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
            )?;
        }

        self.tick_func(&mut functions, &mut codes, &mut exports)?;

        self.finish_events(&mut functions, &mut codes, &mut exports)?;

        self.unreachable_dbg_func(&mut functions, &mut codes, &mut exports)?;

        self.registries().types().clone().finish(&mut types);

        self.registries()
            .tables()
            .clone()
            .finish(&imports, &mut tables, &mut exports);

        elements.declared(Elements::Functions(
            (self.imported_func_count()?..functions.len() + self.imported_func_count()? - 2)
                .collect(),
        ));

        exports.export("memory", ExportKind::Memory, 0);
        exports.export("noop", ExportKind::Func, self.imported_func_count()?);

        self.registries()
            .globals()
            .clone()
            .finish(&imports, &mut globals, &mut exports);

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
            strings,
            target_names: self
                .target_names
                .into_iter()
                .map(|bstr| bstr.into())
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
        <N as TryFrom<usize>>::Error: alloc::fmt::Debug,
    {
        let step_func_ty = self
            .registries()
            .types()
            .register_default((vec![ValType::I32], vec![]))?;

        self.registries().tables().register(
            "threads".into(),
            (
                RefType {
                    nullable: false,
                    heap_type: HeapType::Concrete(step_func_ty),
                },
                0,
                // default to noop, just so the module validates.
                Some(ConstExpr::ref_func(self.imported_func_count()?)),
            ),
        )
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

        let threads_table_index = self.threads_table_index()?;

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
            .map(|i| {
                crate::log(
                    format!(
                        "event step idx: {}; func idx: {}",
                        i,
                        i + self.imported_func_count()?
                    )
                    .as_str(),
                );
                Ok(wasm![
                    RefFunc(i + self.imported_func_count()?),
                    I32Const(1),
                    TableGrow(threads_table_index),
                    Drop,
                ])
            })
            .flatten_ok()
            .collect::<HQResult<Vec<_>>>()?;

        for instruction in instrs {
            func.instruction(&instruction.eval(self.steps(), self.imported_func_count()?)?);
        }
        for instruction in wasm![
            GlobalGet(threads_count),
            I32Const(
                i32::try_from(indices.len())
                    .map_err(|_| make_hq_bug!("indices len out of bounds"))?
            ),
            I32Add,
            GlobalSet(threads_count)
        ] {
            func.instruction(&instruction.eval(self.steps(), self.imported_func_count()?)?);
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
        for (event, indices) in self.events.iter() {
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

        let threads_table_index = self.threads_table_index()?;

        let instructions = wasm![
            TableSize(threads_table_index),
            LocalTee(1),
            I32Eqz,
            BrIf(0),
            Loop(WasmBlockType::Empty),
            LocalGet(0),
            LocalGet(0),
            TableGet(threads_table_index),
            CallRef(step_func_ty),
            LocalGet(0),
            I32Const(1),
            I32Add,
            LocalTee(0),
            LocalGet(1),
            I32LtS,
            BrIf(0),
            End,
        ];
        for instr in instructions {
            tick_func.instruction(&instr.eval(self.steps(), self.imported_func_count()?)?);
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
        let steps: Rc<RefCell<IndexMap<Rc<Step>, StepFunc>>> = Default::default();
        let registries = Rc::new(Registries::default());
        let mut events: BTreeMap<Event, Vec<u32>> = Default::default();
        StepFunc::compile_step(
            Rc::new(Step::new_empty()),
            Rc::clone(&steps),
            Rc::clone(&registries),
            flags,
        )?;
        // compile every step
        for step in ir_project.steps().try_borrow()?.iter() {
            StepFunc::compile_step(
                Rc::clone(step),
                Rc::clone(&steps),
                Rc::clone(&registries),
                flags,
            )?;
        }
        // mark first steps as used in a non-inline context
        for thread in ir_project.threads().try_borrow()?.iter() {
            thread.first_step().make_used_non_inline()?;
        }
        // get rid of steps which aren't used in a non-inlined context
        for step in ir_project.steps().try_borrow()?.iter() {
            if *step.inline().try_borrow()? && !*step.used_non_inline().try_borrow()? {
                steps.try_borrow_mut()?.swap_remove(step);
            }
        }
        // add thread event handlers for them
        for thread in ir_project.threads().try_borrow()?.iter() {
            events.entry(thread.event()).or_default().push(
                u32::try_from(
                    steps
                        .try_borrow()?
                        .get_index_of(thread.first_step())
                        .ok_or(make_hq_bug!(
                            "Thread's first_step wasn't found in Thread::steps()"
                        ))?,
                )
                .map_err(|_| make_hq_bug!("step func index out of bounds"))?,
            );
        }
        Ok(WasmProject {
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
    pub strings: Vec<String>,
    #[wasm_bindgen(getter_with_clone)]
    pub target_names: Vec<String>,
}

#[cfg(test)]
mod tests {
    use super::{Registries, WasmProject};
    use crate::ir::Step;
    use crate::prelude::*;
    use crate::wasm::{ExternalEnvironment, StepFunc};

    #[test]
    fn empty_project_is_valid_wasm() {
        let registries = Rc::new(Registries::default());
        let steps = Default::default();
        StepFunc::compile_step(
            Rc::new(Step::new_empty()),
            Rc::clone(&steps),
            Rc::clone(&registries),
            Default::default(),
        )
        .unwrap();
        let project = WasmProject {
            flags: Default::default(),
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
