use super::{ExternalFunctionMap, TypeRegistry};
use crate::ir::Type as IrType;
use crate::prelude::*;
use wasm_encoder::{Function, Instruction, ValType};

/// representation of a step's function
pub struct StepFunc {
    locals: RefCell<Vec<ValType>>,
    instructions: RefCell<Vec<Instruction<'static>>>,
    param_count: u32,
    type_registry: Rc<TypeRegistry>,
    external_functions: Rc<ExternalFunctionMap>,
}

impl StepFunc {
    pub fn type_registry(&self) -> Rc<TypeRegistry> {
        self.type_registry.clone()
    }

    pub fn external_functions(&self) -> Rc<ExternalFunctionMap> {
        self.external_functions.clone()
    }

    /// creates a new step function, with one paramter
    pub fn new(
        type_registry: Rc<TypeRegistry>,
        external_functions: Rc<ExternalFunctionMap>,
    ) -> Self {
        StepFunc {
            locals: RefCell::new(vec![]),
            instructions: RefCell::new(vec![]),
            param_count: 1,
            type_registry,
            external_functions,
        }
    }

    /// creates a new step function with the specified amount of paramters.
    /// currently only used in testing to validate types
    pub fn new_with_param_count(
        count: usize,
        type_registry: Rc<TypeRegistry>,
        external_functions: Rc<ExternalFunctionMap>,
    ) -> HQResult<Self> {
        Ok(StepFunc {
            locals: RefCell::new(vec![]),
            instructions: RefCell::new(vec![]),
            param_count: u32::try_from(count)
                .map_err(|_| make_hq_bug!("param count out of bounds"))?,
            type_registry,
            external_functions,
        })
    }

    /// Returns the index of the `n`th local of the specified type in this function,
    /// adding some if necessary
    pub fn get_local(&self, val_type: ValType, n: u32) -> HQResult<u32> {
        let existing_count = self
            .locals
            .borrow()
            .iter()
            .filter(|ty| **ty == val_type)
            .count();
        Ok(u32::try_from(if existing_count < (n as usize) {
            {
                self.locals
                    .borrow_mut()
                    .extend([val_type].repeat(n as usize - existing_count));
            }
            self.locals.borrow().len() - 1
        } else {
            // TODO: return nth rather than last
            self.locals
                .borrow()
                .iter()
                .rposition(|ty| *ty == val_type)
                .unwrap()
        })
        .map_err(|_| make_hq_bug!("local index was out of bounds"))?
            + self.param_count)
    }

    pub fn add_instructions(&self, instructions: impl IntoIterator<Item = Instruction<'static>>) {
        self.instructions.borrow_mut().extend(instructions);
    }

    /// Takes ownership of the function and returns the backing `wasm_encoder` `Function`
    pub fn finish(self) -> Function {
        let mut func = Function::new_with_locals_types(self.locals.take());
        for instruction in self.instructions.take() {
            func.instruction(&instruction);
        }
        func.instruction(&Instruction::End);
        func
    }
}
