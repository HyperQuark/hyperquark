use super::{ExternalEnvironment, ExternalFunctionMap};
use crate::ir::{IrProject, Step, Type as IrType};
use crate::prelude::*;
use crate::wasm::{StepFunc, TypeRegistry, WasmFlags};
use wasm_encoder::{CodeSection, FunctionSection, ImportSection, Module, TypeSection, ValType};

/// A respresntation of a WASM representation of a project. Cannot be created directly;
/// use `TryFrom<IrProject>`.
pub struct WasmProject {
    flags: WasmFlags,
    step_funcs: Box<[StepFunc]>,
    type_registry: Rc<TypeRegistry>,
    external_functions: Rc<ExternalFunctionMap>,
    environment: ExternalEnvironment,
}

impl WasmProject {
    pub fn new(flags: WasmFlags, environment: ExternalEnvironment) -> Self {
        WasmProject {
            flags,
            step_funcs: Box::new([]),
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

        let mut imports = ImportSection::new();
        let mut types = TypeSection::new();
        let mut functions = FunctionSection::new();
        let mut codes = CodeSection::new();

        Rc::unwrap_or_clone(self.external_functions())
            .finish(&mut imports, self.type_registry())?;
        for step_func in self.step_funcs().iter().cloned() {
            step_func.finish(&mut functions, &mut codes)?;
        }
        Rc::unwrap_or_clone(self.type_registry()).finish(&mut types);

        module.section(&types);
        module.section(&imports);
        module.section(&functions);
        module.section(&codes);

        let wasm_bytes = module.finish();

        Ok(wasm_bytes)
    }
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
            .splice(
                (type_stack.len() - 1 - opcode.acceptable_inputs().len())..,
                [],
            )
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

impl TryFrom<Rc<IrProject>> for WasmProject {
    type Error = HQError;

    fn try_from(ir_project: Rc<IrProject>) -> HQResult<WasmProject> {
        let steps: RefCell<IndexMap<Rc<Step>, StepFunc>> = Default::default();
        let type_registry = Rc::new(TypeRegistry::new());
        let external_functions = Rc::new(ExternalFunctionMap::new());
        for thread in ir_project.threads().borrow().iter() {
            let step = thread.first_step().get_rc();
            compile_step(
                step,
                &steps,
                Rc::clone(&type_registry),
                Rc::clone(&external_functions),
            )?;
        }
        Ok(WasmProject {
            flags: Default::default(),
            step_funcs: steps.take().values().cloned().collect(),
            type_registry,
            external_functions,
            environment: ExternalEnvironment::WebBrowser,
        })
    }
}
