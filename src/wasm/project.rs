use super::{ExternalEnvironment, GlobalExportable, GlobalMutable, Registries};
use crate::ir::{Event, IrProject, Step, Target as IrTarget, Type as IrType};
use crate::prelude::*;
use crate::wasm::{StepFunc, StringsTable, ThreadsTable, WasmFlags};
use itertools::Itertools;
use wasm_bindgen::prelude::*;
use wasm_encoder::{
    AbstractHeapType, BlockType as WasmBlockType, CodeSection, ConstExpr, DataCountSection,
    DataSection, ElementSection, Elements, ExportKind, ExportSection, Function, FunctionSection,
    GlobalSection, HeapType, ImportSection, Instruction, MemorySection, MemoryType, Module,
    RefType, StartSection, TableSection, TypeSection, ValType,
};
use wasm_gen::wasm;

/// A respresentation of a WASM representation of a project. Cannot be created directly;
/// use `TryFrom<IrProject>`.
pub struct WasmProject {
    #[expect(dead_code, reason = "doesn't need to be used... yet")]
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
            Some(IrType::QuasiInt | IrType::ColorARGB | IrType::ColorRGB) => ValType::I32,
            Some(IrType::String) => ValType::EXTERNREF,
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
        let mut data = DataSection::new();
        let mut globals = GlobalSection::new();

        memories.memory(MemoryType {
            minimum: 1,
            maximum: None,
            memory64: false,
            shared: false,
            page_size_log2: None,
        });

        let mut start_func = Function::new([]);

        Rc::unwrap_or_clone(self.registries().tabled_strings().clone()).finish(
            self.registries().strings(),
            self.registries().tables().register::<StringsTable, _>()?,
            &mut start_func,
        )?;

        Rc::unwrap_or_clone(self.registries().strings().clone()).finish(&mut imports);

        self.registries().lists().clone().finish(
            &mut data,
            &mut elements,
            &mut start_func,
            self.imported_global_count()?,
        )?;

        start_func.instruction(&Instruction::End);

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

        self.registries().static_functions().clone().finish(
            &mut functions,
            &mut codes,
            self.registries.types(),
        )?;

        for step_func in self.steps().try_borrow()?.values().cloned() {
            step_func.finish(
                &mut functions,
                &mut codes,
                self.steps(),
                self.imported_func_count()?,
                self.static_func_count()?,
                self.imported_global_count()?,
            )?;
        }

        self.tick_func(&mut functions, &mut codes, &mut exports)?;

        self.finish_events(&mut functions, &mut codes, &mut exports)?;

        self.unreachable_dbg_func(&mut functions, &mut codes, &mut exports)?;

        codes.function(&start_func);
        functions.function(self.registries().types().function(vec![], vec![])?);

        let start_section = StartSection {
            function_index: self.imported_func_count()? + functions.len() - 1,
        };

        self.registries()
            .tables()
            .register_override::<ThreadsTable, usize, _>(
                self.registries().types().thread_struct_type()?,
            )?;

        elements.declared(Elements::Functions(
            (self.imported_func_count()? + self.static_func_count()?
                ..functions.len() + self.imported_func_count()? + self.static_func_count()? - 2)
                .collect(),
        ));

        Rc::unwrap_or_clone(self.registries().types().clone()).finish(&mut types);

        self.registries()
            .tables()
            .clone()
            .finish(&mut tables, &mut exports);

        // crate::log!(
        //     "imported func count: {}, static func count: {}",
        //     self.imported_func_count()?,
        //     self.static_func_count()?
        // );

        exports.export("memory", ExportKind::Memory, 0);
        exports.export(
            "noop",
            ExportKind::Func,
            self.imported_func_count()? + self.static_func_count()?,
        );

        self.registries().globals().clone().finish(
            &mut globals,
            &mut exports,
            self.imported_global_count()?,
            self.imported_func_count()?,
            self.static_func_count()?,
        );

        module
            .section(&types)
            .section(&imports)
            .section(&functions)
            .section(&tables)
            .section(&memories)
            .section(&globals)
            .section(&exports)
            .section(&start_section)
            .section(&elements)
            .section(&DataCountSection { count: data.len() })
            .section(&codes)
            .section(&data);

        let wasm_bytes = module.finish();

        Ok(FinishedWasm {
            wasm_bytes: wasm_bytes.into_boxed_slice(),
            strings: self
                .registries()
                .strings()
                .registry()
                .borrow()
                .keys()
                .cloned()
                .map(str::into_string)
                .collect(),
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

    fn static_func_count(&self) -> HQResult<u32> {
        self.registries()
            .static_functions()
            .registry()
            .try_borrow()?
            .len()
            .try_into()
            .map_err(|_| make_hq_bug!("static function map len out of bounds"))
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
        functions.function(self.registries().types().function(vec![], vec![])?);
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
        self.registries().tables().register::<ThreadsTable, _>()
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

        let stack_struct_ty = self.registries().types().stack_struct_type()?;
        let stack_array_ty = self.registries().types().stack_array_type()?;
        let thread_struct_ty = self.registries().types().thread_struct_type()?;

        let instrs = indices
            .iter()
            .map(|&i| {
                Ok(wasm![
                    I32Const(1),
                    RefFunc(i + self.imported_func_count()? + self.static_func_count()?),
                    RefNull(HeapType::Abstract {
                        shared: false,
                        ty: AbstractHeapType::Struct
                    }),
                    StructNew(stack_struct_ty),
                    // todo: play around with initial size of stack array
                    RefNull(HeapType::Concrete(stack_struct_ty)),
                    RefNull(HeapType::Concrete(stack_struct_ty)),
                    RefNull(HeapType::Concrete(stack_struct_ty)),
                    RefNull(HeapType::Concrete(stack_struct_ty)),
                    RefNull(HeapType::Concrete(stack_struct_ty)),
                    RefNull(HeapType::Concrete(stack_struct_ty)),
                    RefNull(HeapType::Concrete(stack_struct_ty)),
                    ArrayNewFixed {
                        array_size: 8,
                        array_type_index: stack_array_ty,
                    },
                    StructNew(thread_struct_ty),
                    I32Const(1),
                    TableGrow(self.threads_table_index()?),
                    Drop,
                ])
            })
            .flatten_ok()
            .collect::<HQResult<Vec<_>>>()?;

        for instruction in instrs {
            func.instruction(&instruction.eval(
                self.steps(),
                self.imported_func_count()?,
                self.static_func_count()?,
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
                self.static_func_count()?,
                self.imported_global_count()?,
            )?);
        }
        func.instruction(&Instruction::End);

        funcs.function(self.registries().types().function(vec![], vec![])?);
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
        let thread_struct_type = self.registries().types().thread_struct_type()?;
        let stack_struct_ty = self.registries().types().stack_struct_type()?;

        let mut tick_func = Function::new(vec![
            (2, ValType::I32),
            (
                1,
                ValType::Ref(RefType {
                    nullable: false,
                    heap_type: HeapType::Concrete(thread_struct_type),
                }),
            ),
            (
                1,
                ValType::Ref(RefType {
                    nullable: false,
                    heap_type: HeapType::Concrete(stack_struct_ty),
                }),
            ),
        ]);

        let step_func_ty = self.registries().types().step_func_type()?;
        let stack_array_ty = self.registries().types().stack_array_type()?;

        let instructions = wasm![
            TableSize(self.threads_table_index()?),
            LocalTee(1),
            I32Eqz,
            BrIf(0),
            Loop(WasmBlockType::Empty),
            LocalGet(0),
            LocalGet(0),
            TableGet(self.threads_table_index()?),
            BrOnNull(0),
            LocalTee(2),
            StructGet {
                struct_type_index: thread_struct_type,
                field_index: 1
            },
            LocalGet(2),
            StructGet {
                struct_type_index: thread_struct_type,
                field_index: 0
            },
            I32Const(1),
            I32Sub,
            ArrayGet(stack_array_ty),
            RefAsNonNull,
            LocalTee(3),
            StructGet {
                struct_type_index: stack_struct_ty,
                field_index: 1
            },
            LocalGet(3),
            StructGet {
                struct_type_index: stack_struct_ty,
                field_index: 0
            },
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
            tick_func.instruction(&instr.eval(
                self.steps(),
                self.imported_func_count()?,
                self.static_func_count()?,
                self.imported_global_count()?,
            )?);
        }
        tick_func.instruction(&Instruction::End);
        funcs.function(self.registries().types().function(vec![], vec![])?);
        codes.function(&tick_func);
        exports.export(
            "tick",
            ExportKind::Func,
            funcs.len() + self.imported_func_count()? - 1,
        );
        Ok(())
    }

    pub fn from_ir(
        ir_project: &Rc<IrProject>,
        _ssa_token: crate::optimisation::SSAToken,
        flags: WasmFlags,
    ) -> HQResult<Self> {
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
    #[wasm_bindgen(getter_with_clone)]
    pub strings: Vec<String>,
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
        let project = Rc::new(IrProject::new(BTreeMap::default(), BTreeMap::default()));
        let steps = Rc::new(RefCell::new(IndexMap::default()));
        StepFunc::compile_step(
            Step::new_empty(
                &Rc::downgrade(&project),
                true,
                Rc::new(IrTarget::new(
                    false,
                    BTreeMap::default(),
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
