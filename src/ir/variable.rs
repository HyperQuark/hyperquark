use uuid::Uuid;

use super::Type;
use crate::{
    prelude::*,
    sb3::{Target as Sb3Target, VarVal},
};
use core::cell::Ref;
use core::hash::{Hash, Hasher};

#[derive(Debug)]
struct Variable {
    possible_types: RefCell<Type>,
    #[expect(unused, reason = "will be used in the future")]
    initial_value: VarVal,
    id: String,
}

#[derive(Clone, Debug)]
pub struct RcVar(Rc<Variable>);

impl RcVar {
    pub fn new(ty: Type, initial_value: VarVal) -> Self {
        Self(Rc::new(Variable {
            possible_types: RefCell::new(ty),
            initial_value,
            id: Uuid::new_v4().to_string(),
        }))
    }

    pub fn add_type(&self, ty: Type) {
        let current = *self.0.possible_types.borrow();
        *self.0.possible_types.borrow_mut() = current.or(ty);
    }

    pub fn possible_types(&self) -> Ref<'_, Type> {
        self.0.possible_types.borrow()
    }

    pub fn _initial_value(&self) -> &VarVal {
        &self.0.initial_value
    }

    pub fn id(&self) -> &str {
        &self.0.id
    }
}

impl PartialEq for RcVar {
    fn eq(&self, other: &Self) -> bool {
        self.0.id == other.0.id
        // Rc::ptr_eq(self.0.get_ref(), other.0.get_ref())
    }
}

impl Eq for RcVar {}

impl PartialOrd for RcVar {
    fn partial_cmp(&self, other: &Self) -> Option<core::cmp::Ordering> {
        Some(self.cmp(other))
    }
}

impl Ord for RcVar {
    fn cmp(&self, other: &Self) -> core::cmp::Ordering {
        self.0.id.cmp(&other.0.id)
        //Rc::as_ptr(&self.0).cmp(&Rc::as_ptr(&other.0))
    }
}

impl Hash for RcVar {
    fn hash<H: Hasher>(&self, state: &mut H) {
        self.0.id.hash(state); //core::ptr::hash(Rc::as_ptr(&self.0), state);
    }
}

#[derive(Debug)]
pub struct TargetVar {
    pub var: RcVar,
    /// this MUST not be modified once the `IrProject` is emitted, i.e. once optimisation has begun
    pub is_used: RefCell<bool>,
}

impl fmt::Display for TargetVar {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(
            f,
            r#"{{ "var": {}, "is_used": {} }}"#,
            self.var,
            *self.is_used.borrow()
        )
    }
}

pub type TargetVars = BTreeMap<Box<str>, Rc<TargetVar>>;

pub fn variables_from_target(target: &Sb3Target) -> TargetVars {
    target
        .variables
        .iter()
        .map(|(id, var_info)| {
            (
                id.clone(),
                Rc::new(TargetVar {
                    var: RcVar::new(
                        Type::none(),
                        #[expect(
                            clippy::unwrap_used,
                            reason = "this field exists on all variants"
                        )]
                        var_info.get_1().unwrap().clone(),
                    ),
                    is_used: RefCell::new(false),
                }),
            )
        })
        .collect()
}

pub fn used_vars(vars: &TargetVars) -> Box<[RcVar]> {
    vars.values()
        .filter_map(|var| {
            if *var.is_used.borrow() {
                Some(var.var.clone())
            } else {
                None
            }
        })
        .collect()
}

impl fmt::Display for RcVar {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        let possible_types = self.0.possible_types.borrow();
        //let id = Rc::as_ptr(&self.0) as usize;
        let id = self.id();
        write!(
            f,
            r#"{{
            "possible_types": "{possible_types}",
            "id": {id:?}
        }}"#
        )
    }
}
