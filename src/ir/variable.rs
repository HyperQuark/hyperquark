use super::Type;
use crate::{
    prelude::*,
    sb3::{Target as Sb3Target, VarVal},
};
use core::cell::Ref;
use core::hash::{Hash, Hasher};

#[derive(Debug, Clone, PartialEq)]
pub struct Variable {
    possible_types: RefCell<Type>,
    initial_value: VarVal,
}

impl Variable {
    pub fn new(ty: Type, initial_value: VarVal) -> Self {
        Variable {
            possible_types: RefCell::new(ty),
            initial_value,
        }
    }

    pub fn add_type(&self, ty: Type) {
        let current = *self.possible_types.borrow();
        *self.possible_types.borrow_mut() = current.or(ty);
    }

    pub fn possible_types(&self) -> Ref<Type> {
        self.possible_types.borrow()
    }

    pub fn initial_value(&self) -> &VarVal {
        &self.initial_value
    }
}

#[derive(Clone, Debug)]
pub struct RcVar(pub Rc<Variable>);

impl PartialEq for RcVar {
    fn eq(&self, other: &Self) -> bool {
        Rc::ptr_eq(&self.0, &other.0)
    }
}

impl Eq for RcVar {}

impl Hash for RcVar {
    fn hash<H: Hasher>(&self, state: &mut H) {
        core::ptr::hash(Rc::as_ptr(&self.0), state);
    }
}

pub fn variables_from_target(target: &Sb3Target) -> BTreeMap<Box<str>, Rc<Variable>> {
    target
        .variables
        .iter()
        .map(|(id, var_info)| {
            (
                id.clone(),
                Rc::new(Variable::new(
                    Type::none(),
                    var_info.get_1().unwrap().clone(),
                )),
            )
        })
        .collect()
}
