use core::cell::Ref;
use core::hash::{Hash, Hasher};

use uuid::Uuid;

use super::Type;
use crate::ir::types::{Type as IrType, var_val_type};
use crate::prelude::*;
use crate::sb3::{Monitor as Sb3Monitor, Target as Sb3Target, VarVal};
use crate::wasm::WasmFlags;
use crate::wasm::flags::Switch;

#[derive(Debug)]
struct Variable {
    possible_types: RefCell<Type>,
    initial_value: VarVal,
    id: String,
    monitor: Option<IrMonitor>,
}

#[derive(Clone, Debug)]
pub struct RcVar(Rc<Variable>);

#[derive(Debug)]
pub struct IrMonitor {
    pub id: Box<str>,
    pub is_ever_visible: RefCell<bool>,
}

impl RcVar {
    pub fn new(
        ty: Type,
        initial_value: &VarVal,
        monitor: Option<IrMonitor>,
        flags: &WasmFlags,
    ) -> HQResult<Self> {
        let init = maybe_eagerly_parse_var_val(initial_value, flags);
        Ok(Self(Rc::new(Variable {
            possible_types: RefCell::new(ty.or(var_val_type(&init)?)),
            initial_value: init,
            id: Uuid::new_v4().to_string(),
            monitor,
        })))
    }

    /// Create empty variable for use in SSA
    #[must_use]
    pub fn new_empty() -> Self {
        Self(Rc::new(Variable {
            possible_types: RefCell::new(Type::none()),
            initial_value: VarVal::Bool(false), // arbitrary value
            id: Uuid::new_v4().to_string(),
            monitor: None,
        }))
    }

    pub fn add_type(&self, ty: Type) {
        let current = *self.0.possible_types.borrow();
        *self.0.possible_types.borrow_mut() = current.or(ty);
    }

    #[must_use]
    pub fn possible_types(&self) -> Ref<'_, Type> {
        self.0.possible_types.borrow()
    }

    #[must_use]
    pub fn initial_value(&self) -> &VarVal {
        &self.0.initial_value
    }

    #[must_use]
    pub fn id(&self) -> &str {
        &self.0.id
    }

    #[must_use]
    pub fn monitor(&self) -> &Option<IrMonitor> {
        &self.0.monitor
    }
}

impl PartialEq for RcVar {
    fn eq(&self, other: &Self) -> bool {
        self.0.id == other.0.id
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
    }
}

impl Hash for RcVar {
    fn hash<H: Hasher>(&self, state: &mut H) {
        self.0.id.hash(state);
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

#[derive(Debug)]
pub struct TargetList {
    pub list: RcList,
    /// this MUST not be modified once the `IrProject` is emitted, i.e. once optimisation has begun
    pub is_used: RefCell<bool>,
}

impl fmt::Display for TargetList {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(
            f,
            r#"{{ "list": {}, "is_used": {} }}"#,
            self.list,
            *self.is_used.borrow()
        )
    }
}

pub type TargetVars = BTreeMap<Box<str>, Rc<TargetVar>>;

pub type TargetLists = BTreeMap<Box<str>, Rc<TargetList>>;

pub fn variables_from_target(
    target: &Sb3Target,
    monitors: &[Sb3Monitor],
    flags: &WasmFlags,
) -> HQResult<TargetVars> {
    target
        .variables
        .iter()
        .map(|(id, var_info)| {
            let monitor = monitors
                .iter()
                .find(|monitor| monitor.id() == Some(id))
                .and_then(|monitor| {
                    Some(IrMonitor {
                        is_ever_visible: RefCell::new(*monitor.visible()?),
                        id: id.clone(),
                    })
                });
            Ok((
                id.clone(),
                Rc::new(TargetVar {
                    var: RcVar::new(
                        #[expect(clippy::unwrap_used, reason = "field present in all variants")]
                        var_val_type(var_info.get_1().unwrap())?,
                        #[expect(
                            clippy::unwrap_used,
                            reason = "this field exists on all variants"
                        )]
                        var_info.get_1().unwrap(),
                        monitor,
                        flags,
                    )?,
                    is_used: RefCell::new(false),
                }),
            ))
        })
        .collect()
}

pub fn lists_from_target(target: &Sb3Target, flags: &WasmFlags) -> HQResult<TargetLists> {
    target
        .lists
        .iter()
        .map(|(id, list_info)| {
            Ok((
                id.clone(),
                Rc::new(TargetList {
                    list: RcList::new(
                        list_info
                            .1
                            .iter()
                            .map(var_val_type)
                            .try_fold(IrType::none(), |a, b| -> HQResult<_> { Ok(a.or(b?)) })?,
                        list_info.1.clone(),
                        flags,
                    )?,
                    is_used: RefCell::new(false),
                }),
            ))
        })
        .collect()
}

/// A list of variables in a target that are used somewhere (whether read or written to)
#[must_use]
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

impl fmt::Display for RcList {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        let possible_types = self.0.possible_types.borrow();
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

fn maybe_eagerly_parse_var_val(var_val: &VarVal, flags: &WasmFlags) -> VarVal {
    match var_val {
        VarVal::Float(f) => {
            if flags.integers == Switch::On && f % 1.0 == 0.0 {
                #[expect(
                    clippy::cast_possible_truncation,
                    reason = "integer-ness already confirmed; `as` is saturating."
                )]
                VarVal::Int(*f as i32)
            } else {
                VarVal::Float(*f)
            }
        }
        VarVal::Int(i) => VarVal::Int(*i),
        VarVal::Bool(b) => VarVal::Bool(*b),
        VarVal::String(s) => {
            if flags.eager_number_parsing == Switch::On
                && let Ok(f) = s.parse::<f64>()
                && *f.to_string() == **s
            {
                if flags.integers == Switch::On && f % 1.0 == 0.0 {
                    #[expect(
                        clippy::cast_possible_truncation,
                        reason = "integer-ness already confirmed; `as` is saturating."
                    )]
                    VarVal::Int(f as i32)
                } else {
                    VarVal::Float(f)
                }
            } else {
                VarVal::String(s.clone())
            }
        }
    }
}

#[derive(Debug)]
struct List {
    possible_types: RefCell<Type>,
    length_mutable: RefCell<bool>,
    initial_value: Vec<VarVal>,
    id: String,
}

#[derive(Clone, Debug)]
pub struct RcList(Rc<List>);

impl RcList {
    pub fn new(ty: Type, initial_value: Vec<VarVal>, flags: &WasmFlags) -> HQResult<Self> {
        let init: Vec<_> = initial_value
            .into_iter()
            .map(|val| {
                let parsed_val = maybe_eagerly_parse_var_val(&val, flags);
                if flags.integers == Switch::Off
                    && let VarVal::Int(i) = parsed_val
                {
                    VarVal::String(i.to_string().into_boxed_str())
                } else {
                    parsed_val
                }
            })
            .collect();
        Ok(Self(Rc::new(List {
            possible_types: RefCell::new(
                ty.or(init
                    .iter()
                    .try_fold(Type::none(), |t: Type, v| -> HQResult<_> {
                        Ok(t.or(var_val_type(v)?))
                    })?),
            ),
            length_mutable: RefCell::new(false),
            initial_value: init,
            id: Uuid::new_v4().to_string(),
        })))
    }

    pub fn add_type(&self, ty: Type) {
        let current = *self.0.possible_types.borrow();
        *self.0.possible_types.borrow_mut() = current.or(ty);
    }

    #[must_use]
    pub fn possible_types(&self) -> Ref<'_, Type> {
        self.0.possible_types.borrow()
    }

    #[must_use]
    pub fn initial_value(&self) -> &Vec<VarVal> {
        &self.0.initial_value
    }

    #[must_use]
    pub fn id(&self) -> &str {
        &self.0.id
    }

    #[must_use]
    pub fn length_mutable(&self) -> &RefCell<bool> {
        &self.0.length_mutable
    }
}

impl PartialEq for RcList {
    fn eq(&self, other: &Self) -> bool {
        self.0.id == other.0.id
    }
}

impl Eq for RcList {}

impl PartialOrd for RcList {
    fn partial_cmp(&self, other: &Self) -> Option<core::cmp::Ordering> {
        Some(self.cmp(other))
    }
}

impl Ord for RcList {
    fn cmp(&self, other: &Self) -> core::cmp::Ordering {
        self.0.id.cmp(&other.0.id)
    }
}

impl Hash for RcList {
    fn hash<H: Hasher>(&self, state: &mut H) {
        self.0.id.hash(state);
    }
}
