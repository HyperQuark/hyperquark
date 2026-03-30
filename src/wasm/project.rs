use itertools::Itertools;
use wasm_bindgen::prelude::*;
use wasm_encoder::{
    AbstractHeapType, BlockType as WasmBlockType, CodeSection, ConstExpr, DataCountSection,
    DataSection, ElementSection, Elements, ExportKind, ExportSection, FieldType, Function,
    FunctionSection, GlobalSection, HeapType, ImportSection, Instruction, MemorySection,
    MemoryType, Module, RefType, StartSection, StorageType, TableSection, TypeSection, ValType,
};
use wasm_gen::wasm;

use super::{ExternalEnvironment, GlobalExportable, GlobalMutable, Registries};
use crate::ir::{Event, IrProject, StepIndex, Type as IrType};
use crate::prelude::*;
use crate::wasm::registries::functions::static_functions::{
    MarkWaitingFlag, SpawnNewThread, SpawnThreadInStack,
};
use crate::wasm::{StepFunc, StringsTable, ThreadsTable, WasmFlags};

/// A respresentation of a WASM representation of a project. Cannot be created directly;
/// use `TryFrom<IrProject>`.
pub struct WasmProject {
    #[expect(dead_code, reason = "doesn't need to be used... yet")]
    flags: WasmFlags,
    /// step funcs corresponding to the non-inlined steps, in the same order (hopefully)
    steps: Rc<RefCell<Vec<StepFunc>>>,
    /// maps an event to a list of *`step_func`* indices (NOT function indices) which are
    /// triggered by that event.
    events: BTreeMap<Event, Vec<u32>>,
    registries: Rc<Registries>,
    target_names: Vec<Box<str>>,
    costume_names: Rc<Vec<Vec<Box<str>>>>,
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
            steps: Rc::new(RefCell::new(Vec::new())),
            events: BTreeMap::default(),
            environment,
            registries: Rc::new(Registries::default()),
            target_names: vec![],
            costume_names: Rc::new(vec![]),
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
    pub const fn steps(&self) -> &Rc<RefCell<Vec<StepFunc>>> {
        &self.steps
    }

    /// maps a broad IR type to a WASM type
    pub fn ir_type_to_wasm(ir_type: IrType) -> HQResult<ValType> {
        let base = ir_type.base_type();
        Ok(match base {
            Some(IrType::Float) => ValType::F64,
            Some(IrType::Int | IrType::Boolean | IrType::ColorARGB | IrType::ColorRGB) => {
                ValType::I32
            }
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

        self.registries()
            .external_functions()
            .clone()
            .finish(&mut imports, self.registries().types())?;

        self.registries()
            .static_functions()
            .register_override::<SpawnNewThread, usize, _>((
                self.registries().types().step_func_type()?,
                self.registries().types().stack_struct_type()?,
                self.registries().types().stack_array_type()?,
                self.registries().types().thread_struct_type()?,
                self.threads_table_index()?,
            ))?;

        self.registries()
            .static_functions()
            .register_override::<SpawnThreadInStack, usize, _>((
                self.registries().types().step_func_type()?,
                self.registries().types().stack_struct_type()?,
                self.registries().types().stack_array_type()?,
                self.registries().types().thread_struct_type()?,
                self.threads_table_index()?,
            ))?;

        self.registries()
            .static_functions()
            .register_override::<MarkWaitingFlag, usize, _>(self.registries().types().struct_(
                vec![FieldType {
                    element_type: StorageType::I8,
                    mutable: true,
                }],
            )?)?;

        self.registries().static_functions().clone().finish(
            &mut functions,
            &mut exports,
            &mut codes,
            self.registries.types(),
            self.imported_func_count()?,
        )?;

        for step_func in self.steps().try_borrow()?.iter().cloned() {
            step_func.finish(
                &mut functions,
                &mut codes,
                &self.events,
                self.registries().types(),
                self.threads_count_global()?,
                self.spawn_new_thread_func()?,
                self.spawn_thread_in_stack_func()?,
                self.threads_table_index()?,
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
                ..self.imported_func_count()?
                    + self.static_func_count()?
                    + u32::try_from(self.steps().try_borrow()?.len())
                        .map_err(|_| make_hq_bug!("steps len out of bounds"))?)
                .collect(),
        ));

        Rc::unwrap_or_clone(self.registries().types().clone()).finish(&mut types);

        // TODO: make an elements registry to deal with this at the site of the block
        for (table_index, table_name) in self
            .registries()
            .tables()
            .registry()
            .try_borrow()?
            .keys()
            .enumerate()
        {
            if table_name.starts_with("costume_names_") {
                #[expect(
                    clippy::unwrap_used,
                    reason = "checked immediately before that this is a prefix of the table name"
                )]
                let target_index: u32 = table_name
                    .strip_prefix("costume_names_")
                    .unwrap()
                    .parse()
                    .map_err(|_| make_hq_bug!("couldn't parse target index from table name"))?;
                let costume_names = self
                    .costume_names
                    .get(target_index as usize)
                    .ok_or_else(|| make_hq_bug!("target index out of bounds for costume names"))?;
                let name_globals = costume_names
                    .iter()
                    .map(|costume_name| {
                        Ok(ConstExpr::global_get(
                            u32::try_from(
                                self.registries()
                                    .strings()
                                    .registry()
                                    .try_borrow()?
                                    .get_index_of(costume_name)
                                    .ok_or_else(|| {
                                        make_hq_bug!(
                                            "couldn't find costume name string in strings registry"
                                        )
                                    })?,
                            )
                            .map_err(|_| make_hq_bug!("string index out of bounds"))?,
                        ))
                    })
                    .collect::<HQResult<Box<[_]>>>()?;
                elements.active(
                    Some(
                        table_index
                            .try_into()
                            .map_err(|_| make_hq_bug!("table index out of bounds"))?,
                    ),
                    &ConstExpr::i32_const(0),
                    Elements::Expressions(RefType::EXTERNREF, Cow::Borrowed(&*name_globals)),
                );
            }
        }

        self.registries()
            .tables()
            .clone()
            .finish(&mut tables, &mut exports);

        exports.export("memory", ExportKind::Memory, 0);

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

    fn spawn_new_thread_func<N>(&self) -> HQResult<N>
    where
        N: TryFrom<usize>,
        <N as TryFrom<usize>>::Error: fmt::Debug,
    {
        self.registries()
            .static_functions()
            .register::<SpawnNewThread, _>()
    }

    fn spawn_thread_in_stack_func<N>(&self) -> HQResult<N>
    where
        N: TryFrom<usize>,
        <N as TryFrom<usize>>::Error: fmt::Debug,
    {
        self.registries()
            .static_functions()
            .register::<SpawnThreadInStack, _>()
    }

    fn threads_count_global<N>(&self) -> HQResult<N>
    where
        N: TryFrom<usize>,
        <N as TryFrom<usize>>::Error: fmt::Debug,
    {
        self.registries().globals().register(
            "threads_count".into(),
            (
                ValType::I32,
                ConstExpr::i32_const(0),
                GlobalMutable(true),
                GlobalExportable(true),
            ),
        )
    }

    #[expect(clippy::needless_pass_by_value, reason = "annoying to borrow a box")]
    fn finish_event(
        &self,
        export_name: Box<str>,
        indices: &[u32],
        funcs: &mut FunctionSection,
        codes: &mut CodeSection,
        exports: &mut ExportSection,
    ) -> HQResult<u32> {
        let mut func = Function::new(vec![]);

        let threads_count = self.threads_count_global()?;

        let spawn_new_thread = self.spawn_new_thread_func()?;

        let instrs = indices
            .iter()
            .map(|&i| {
                Ok(wasm![
                    RefFunc(i + self.imported_func_count()? + self.static_func_count()?),
                    RefNull(HeapType::Abstract {
                        shared: false,
                        ty: AbstractHeapType::Struct
                    }),
                    #StaticFunctionCall(spawn_new_thread),
                ])
            })
            .flatten_ok()
            .collect::<HQResult<Vec<_>>>()?;

        for instruction in instrs {
            for real_instruction in instruction.eval(
                &self.events,
                self.registries().types(),
                self.threads_count_global()?,
                self.spawn_new_thread_func()?,
                self.spawn_thread_in_stack_func()?,
                self.threads_table_index()?,
                self.imported_func_count()?,
                self.static_func_count()?,
                self.imported_global_count()?,
            )? {
                func.instruction(&real_instruction);
            }
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
            for real_instruction in instruction.eval(
                &self.events,
                self.registries().types(),
                self.threads_count_global()?,
                self.spawn_new_thread_func()?,
                self.spawn_thread_in_stack_func()?,
                self.threads_table_index()?,
                self.imported_func_count()?,
                self.static_func_count()?,
                self.imported_global_count()?,
            )? {
                func.instruction(&real_instruction);
            }
        }
        func.instruction(&Instruction::End);

        funcs.function(self.registries().types().function(vec![], vec![])?);
        codes.function(&func);
        exports.export(
            &export_name,
            ExportKind::Func,
            self.imported_func_count()? + funcs.len() - 1,
        );

        Ok(self.imported_func_count()? + funcs.len() - 1)
    }

    fn finish_events(
        &self,
        funcs: &mut FunctionSection,
        codes: &mut CodeSection,
        exports: &mut ExportSection,
    ) -> HQResult<()> {
        let event_funcs = self
            .events
            .iter()
            .map(|(event, indices)| {
                Ok(Some((
                    event,
                    self.finish_event(
                        match event {
                            Event::FlagClicked => "flag_clicked".into(),
                            Event::Broadcast(_) => return Ok(None), // broadcasts handled in the sender blocks
                            Event::SpriteClicked(index) => {
                                format!("spriteClicked{index}").into_boxed_str()
                            }
                        },
                        indices,
                        funcs,
                        codes,
                        exports,
                    )?,
                )))
            })
            .collect::<HQResult<Box<[_]>>>()?
            .into_iter()
            .flatten()
            .collect::<BTreeMap<_, _>>();
        let sprite_clicked_indices: Box<[i32]> = self
            .events
            .keys()
            .filter_map(|e| {
                #[expect(clippy::redundant_else, reason = "false positive")]
                if let Event::SpriteClicked(index) = e {
                    Some(
                        i32::try_from(*index)
                            .map_err(|_| make_hq_bug!("target index out of bounds")),
                    )
                } else {
                    None
                }
            })
            .collect::<HQResult<_>>()?;
        if !sprite_clicked_indices.is_empty() {
            let mut sprite_clicked_func = Function::new([]);
            let sprite_clicked_instrs: Vec<_> = sprite_clicked_indices
                .iter()
                .flat_map(|index| {
                    wasm![
                        LocalGet(0),
                        I32Const(*index),
                        I32Eq,
                        If(WasmBlockType::Empty),
                        Call(
                            event_funcs[&Event::SpriteClicked(
                                #[expect(
                                    clippy::unwrap_used,
                                    reason = "guaranteed to succeed because i32 was originally a \
                                              u32"
                                )]
                                (*index).try_into().unwrap()
                            )]
                        ),
                        Return,
                        End,
                    ]
                })
                .chain(wasm![End])
                .collect();
            for instruction in sprite_clicked_instrs {
                for real_instruction in instruction.eval(
                    &self.events,
                    self.registries().types(),
                    self.threads_count_global()?,
                    self.spawn_new_thread_func()?,
                    self.spawn_thread_in_stack_func()?,
                    self.threads_table_index()?,
                    self.imported_func_count()?,
                    self.static_func_count()?,
                    self.imported_global_count()?,
                )? {
                    sprite_clicked_func.instruction(&real_instruction);
                }
            }
            funcs.function(
                self.registries()
                    .types()
                    .function(vec![ValType::I32], vec![])?,
            );
            codes.function(&sprite_clicked_func);
            exports.export(
                "trigger_sprite_clicked",
                ExportKind::Func,
                self.imported_func_count()? + funcs.len() - 1,
            );
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
                    nullable: true,
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
            LocalTee(2),
            RefIsNull,
            If(WasmBlockType::Empty),
            LocalGet(0),
            I32Const(1),
            I32Add,
            LocalTee(0),
            LocalGet(1),
            I32LtS,
            If(WasmBlockType::Empty),
            Br(2),
            Else,
            Return,
            End,
            End,
            LocalGet(2),
            RefAsNonNull,
            StructGet {
                struct_type_index: thread_struct_type,
                field_index: 1
            },
            LocalGet(2),
            RefAsNonNull,
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
            for real_instruction in instr.eval(
                &self.events,
                self.registries().types(),
                self.threads_count_global()?,
                self.spawn_new_thread_func()?,
                self.spawn_thread_in_stack_func()?,
                self.threads_table_index()?,
                self.imported_func_count()?,
                self.static_func_count()?,
                self.imported_global_count()?,
            )? {
                tick_func.instruction(&real_instruction);
            }
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
        let steps = Rc::new(RefCell::new(Vec::new()));
        let registries = Rc::new(Registries::default());
        let mut events: BTreeMap<Event, Vec<u32>> = BTreeMap::default();
        let costume_names = Rc::new(
            ir_project
                .targets()
                .try_borrow()?
                .values()
                .map(|target| {
                    target
                        .costumes()
                        .iter()
                        .map(|costume| costume.name.clone())
                        .collect()
                })
                .collect(),
        );
        for (i, step) in ir_project.steps().try_borrow()?.iter().enumerate() {
            StepFunc::compile_step(
                step,
                StepIndex(i),
                &steps,
                Rc::clone(&registries),
                flags,
                Rc::clone(&costume_names),
            )?;
        }
        // add thread event handlers for them
        for thread in ir_project.threads().try_borrow()?.iter() {
            events.entry(thread.event().clone()).or_default().push(
                u32::try_from(thread.first_step().0)
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
            costume_names,
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
    use crate::prelude::*;
    use crate::wasm::flags::all_wasm_features;
    use crate::wasm::{ExternalEnvironment, WasmFlags};

    #[test]
    fn empty_project_is_valid_wasm() {
        let registries = Rc::new(Registries::default());
        let steps = Rc::new(RefCell::new(Vec::new()));
        let project = WasmProject {
            flags: WasmFlags::new(all_wasm_features()),
            steps,
            events: BTreeMap::new(),
            environment: ExternalEnvironment::WebBrowser,
            registries,
            target_names: vec![],
            costume_names: Rc::new(vec![]),
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
