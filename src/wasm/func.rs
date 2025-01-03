use super::{ExternalFunctionMap, TypeRegistry};
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
    /// adding some if necessary. `n` should be greater than 0.
    pub fn get_local(&self, val_type: ValType, n: u32) -> HQResult<u32> {
        if n == 0 {
            hq_bug!("can't have a 0 amount of locals; n should be >0 for get_local")
        }
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
            self.locals
                .borrow()
                .iter()
                .enumerate()
                .filter(|(_, ty)| **ty == val_type)
                .map(|(i, _)| i)
                .nth(n as usize - 1)
                .ok_or(make_hq_bug!(
                    "couldn't find nth local of type {:?}",
                    val_type
                ))?
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

#[cfg(test)]
mod tests {
    use super::StepFunc;
    use wasm_encoder::ValType;

    #[test]
    fn get_local_works_with_valid_inputs_with_1_param() {
        let func = StepFunc::new(Default::default(), Default::default());
        assert_eq!(func.get_local(ValType::I32, 1).unwrap(), 1);

        assert_eq!(func.get_local(ValType::I32, 1).unwrap(), 1);

        assert_eq!(func.get_local(ValType::F64, 1).unwrap(), 2);
        assert_eq!(func.get_local(ValType::I32, 1).unwrap(), 1);

        assert_eq!(func.get_local(ValType::I32, 2).unwrap(), 3);
        assert_eq!(func.get_local(ValType::F64, 1).unwrap(), 2);
        assert_eq!(func.get_local(ValType::I32, 1).unwrap(), 1);

        assert_eq!(func.get_local(ValType::F64, 4).unwrap(), 6);
    }

    #[test]
    fn get_local_fails_when_n_is_0() {
        let func = StepFunc::new(Default::default(), Default::default());
        assert!(func.get_local(ValType::I32, 0).is_err());
    }

    #[test]
    fn get_local_works_with_valid_inputs_with_3_params() {
        let func =
            StepFunc::new_with_param_count(3, Default::default(), Default::default()).unwrap();
        assert_eq!(func.get_local(ValType::I32, 1).unwrap(), 3);

        assert_eq!(func.get_local(ValType::I32, 1).unwrap(), 3);

        assert_eq!(func.get_local(ValType::F64, 1).unwrap(), 4);
        assert_eq!(func.get_local(ValType::I32, 1).unwrap(), 3);

        assert_eq!(func.get_local(ValType::I32, 2).unwrap(), 5);
        assert_eq!(func.get_local(ValType::F64, 1).unwrap(), 4);
        assert_eq!(func.get_local(ValType::I32, 1).unwrap(), 3);

        assert_eq!(func.get_local(ValType::F64, 4).unwrap(), 8);
    }
}
