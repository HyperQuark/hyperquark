// intermediate representation
use crate::log;
use crate::sb3::{
    Block, BlockArray, BlockArrayOrId, BlockOpcode, CostumeDataFormat, Field, Input, Sb3Project,
    VarVal, VariableInfo,
};
use crate::HQError;
use alloc::boxed::Box;
use alloc::collections::BTreeMap;
use alloc::rc::Rc;
use alloc::string::{String, ToString};
use alloc::vec::Vec;
use core::cell::RefCell;
use core::hash::BuildHasherDefault;
use hashers::fnv::FNV1aHasher64;
use indexmap::IndexMap;
use uuid::Uuid;

#[derive(Debug, Clone, PartialEq)]
pub struct IrCostume {
    pub name: String,
    pub data_format: CostumeDataFormat,
    pub md5ext: String,
}

#[derive(Debug)]
pub struct IrProject {
    pub threads: Vec<Thread>,
    pub vars: Rc<RefCell<Vec<IrVar>>>,
    pub target_names: Vec<String>,
    pub costumes: Vec<Vec<IrCostume>>, // (name, assetName)
    pub steps: IndexMap<(String, String), Step, BuildHasherDefault<FNV1aHasher64>>,
    pub sb3: Sb3Project,
}

impl TryFrom<Sb3Project> for IrProject {
    type Error = HQError;

    fn try_from(sb3: Sb3Project) -> Result<IrProject, Self::Error> {
        let vars: Rc<RefCell<Vec<IrVar>>> = Rc::new(RefCell::new(
            sb3.targets
                .iter()
                .flat_map(|target| {
                    target.variables.iter().map(|(id, info)| match info {
                        VariableInfo::LocalVar(name, val) => {
                            IrVar::new(id.clone(), name.clone(), val.clone(), false)
                        }
                        VariableInfo::CloudVar(name, val, is_cloud) => {
                            IrVar::new(id.clone(), name.clone(), val.clone(), *is_cloud)
                        }
                    })
                })
                .collect(),
        ));

        let costumes: Vec<Vec<IrCostume>> = sb3
            .targets
            .iter()
            .map(|target| {
                target
                    .costumes
                    .iter()
                    .map(|costume| {
                        IrCostume {
                            name: costume.name.clone(),
                            data_format: costume.data_format,
                            md5ext: costume.md5ext.clone(),
                            //data: load_asset(costume.md5ext.as_str()),
                        }
                    })
                    .collect()
            })
            .collect();

        let mut steps: IndexMap<(String, String), Step, BuildHasherDefault<FNV1aHasher64>> =
            Default::default();
        // insert a noop step so that these step indices match up with the step function indices in the generated wasm
        // (step function 0 is a noop)
        steps.insert(
            ("".into(), "".into()),
            Step::new(
                vec![],
                Rc::new(ThreadContext {
                    target_index: u32::MAX,
                    dbg: false,
                    vars: Rc::new(RefCell::new(vec![])),
                    target_num: sb3.targets.len(),
                    costumes: vec![],
                }),
            ),
        );
        let mut threads: Vec<Thread> = vec![];
        for (target_index, target) in sb3.targets.iter().enumerate() {
            for (id, block) in
                target
                    .blocks
                    .clone()
                    .iter()
                    .filter(|(_id, b)| match b.block_info() {
                        Some(block_info) => {
                            block_info.top_level
                                && matches!(block_info.opcode, BlockOpcode::event_whenflagclicked)
                        }
                        None => false,
                    })
            {
                let context = Rc::new(ThreadContext {
                    target_index: target_index.try_into().map_err(|_| make_hq_bug!(""))?,
                    dbg: target.comments.clone().iter().any(|(_id, comment)| {
                        matches!(comment.block_id.clone(), Some(d) if &d == id)
                            && comment.text.clone() == *"hq-dbg"
                    }),
                    vars: Rc::clone(&vars),
                    target_num: sb3.targets.len(),
                    costumes: costumes.get(target_index).ok_or(make_hq_bug!(""))?.clone(),
                });
                let thread = Thread::from_hat(
                    block.clone(),
                    target.blocks.clone(),
                    context,
                    &mut steps,
                    target.name.clone(),
                )?;
                threads.push(thread);
            }
        }
        let target_names = sb3
            .targets
            .iter()
            .map(|t| t.name.clone())
            .collect::<Vec<_>>();
        Ok(Self {
            vars,
            threads,
            target_names,
            steps,
            costumes,
            sb3,
        })
    }
}

#[allow(non_camel_case_types)]
#[allow(non_snake_case)]
#[derive(Debug, Clone, PartialEq)]
pub enum IrOpcode {
    control_repeat,
    control_repeat_until,
    control_while,
    control_for_each {
        VARIABLE: String,
    },
    control_forever,
    control_wait,
    control_wait_until,
    control_stop {
        STOP_OPTION: String,
    },
    control_create_clone_of,
    control_create_clone_of_menu {
        CLONE_OPTOON: String,
    },
    control_delete_this_clone,
    control_get_counter,
    control_incr_counter,
    control_clear_counter,
    control_all_at_once,
    control_start_as_clone,
    data_variable {
        VARIABLE: String,
    },
    data_setvariableto {
        VARIABLE: String,
    },
    data_teevariable {
        VARIABLE: String,
    },
    data_hidevariable {
        VARIABLE: String,
    },
    data_showvariable {
        VARIABLE: String,
    },
    data_listcontents {
        LIST: String,
    },
    data_addtolist,
    data_deleteoflist,
    data_deletealloflist,
    data_insertatlist,
    data_replaceitemoflist,
    data_itemoflist,
    data_itemnumoflist,
    data_lengthoflist,
    data_listcontainsitem,
    data_hidelist,
    data_showlist,
    event_broadcast,
    event_broadcastandwait,
    event_whenflagclicked,
    event_whenkeypressed,
    event_whenthisspriteclicked,
    event_whentouchingobject,
    event_whenstageclicked,
    event_whenbackdropswitchesto,
    event_whengreaterthan,
    event_whenbroadcastreceived,
    hq_cast(InputType),
    hq_drop(usize),
    hq_goto {
        step: Option<(String, String)>,
        does_yield: bool,
    },
    hq_goto_if {
        step: Option<(String, String)>,
        does_yield: bool,
    },
    looks_say,
    looks_sayforsecs,
    looks_think,
    looks_thinkforsecs,
    looks_show,
    looks_hide,
    looks_hideallsprites,
    looks_switchcostumeto,
    looks_switchbackdropto,
    looks_switchbackdroptoandwait,
    looks_nextcostume,
    looks_nextbackdrop,
    looks_changeeffectby,
    looks_seteffectto,
    looks_cleargraphiceffects,
    looks_changesizeby,
    looks_setsizeto,
    looks_changestretchby,
    looks_setstretchto,
    looks_gotofrontback,
    looks_goforwardbackwardlayers,
    looks_size,
    looks_costumenumbername,
    looks_backdropnumbername,
    looks_backdrops,
    math_angle {
        NUM: f64,
    },
    math_integer {
        NUM: f64,
    },
    math_number {
        NUM: f64,
    },
    math_positive_number {
        NUM: f64,
    },
    math_whole_number {
        NUM: f64,
    },
    motion_movesteps,
    motion_gotoxy,
    motion_goto,
    motion_turnright,
    motion_turnleft,
    motion_pointindirection,
    motion_pointtowards,
    motion_glidesecstoxy,
    motion_glideto,
    motion_ifonedgebounce,
    motion_setrotationstyle,
    motion_changexby,
    motion_setx,
    motion_changeyby,
    motion_sety,
    motion_xposition,
    motion_yposition,
    motion_direction,
    motion_scroll_right,
    motion_scroll_up,
    motion_align_scene,
    motion_xscroll,
    motion_yscroll,
    motion_pointtowards_menu,
    operator_add,
    operator_subtract,
    operator_multiply,
    operator_divide,
    operator_lt,
    operator_equals,
    operator_gt,
    operator_and,
    operator_or,
    operator_not,
    operator_random,
    operator_join,
    operator_letter_of,
    operator_length,
    operator_contains,
    operator_mod,
    operator_round,
    operator_mathop {
        OPERATOR: String,
    },
    pen_clear,
    pen_stamp,
    pen_penDown,
    pen_penUp,
    pen_setPenColorToColor,
    pen_changePenColorParamBy,
    pen_setPenColorParamTo,
    pen_changePenSizeBy,
    pen_setPenSizeTo,
    pen_setPenShadeToNumber,
    pen_changePenShadeBy,
    pen_setPenHueToNumber,
    pen_changePenHueBy,
    procedures_definition,
    procedures_call,
    procedures_prototype,
    argument_reporter_string_number,
    argument_reporter_boolean,
    sensing_touchingobject,
    sensing_touchingcolor,
    sensing_coloristouchingcolor,
    sensing_distanceto,
    sensing_distancetomenu,
    sensing_timer,
    sensing_resettimer,
    sensing_of,
    sensing_mousex,
    sensing_mousey,
    sensing_setdragmode,
    sensing_mousedown,
    sensing_keypressed,
    sensing_current,
    sensing_dayssince2000,
    sensing_loudness,
    sensing_loud,
    sensing_askandwait,
    sensing_answer,
    sensing_username,
    sensing_userid,
    sensing_touchingobjectmenu,
    sensing_keyoptions,
    sensing_of_object_menu,
    sound_play,
    sound_playuntildone,
    sound_stopallsounds,
    sound_seteffectto,
    sound_changeeffectby,
    sound_cleareffects,
    sound_sounds_menu,
    sound_beats_menu,
    sound_effects_menu,
    sound_setvolumeto,
    sound_changevolumeby,
    sound_volume,
    text {
        TEXT: String,
    },
}

#[derive(Debug, Clone)]
struct TypeStack(pub Rc<RefCell<Option<TypeStack>>>, pub InputType);

impl TypeStack {
    pub fn new_some(prev: TypeStack) -> Rc<RefCell<Option<Self>>> {
        Rc::new(RefCell::new(Some(prev)))
    }
}

pub trait TypeStackImpl {
    fn get(&self, i: usize) -> Result<Rc<RefCell<Option<TypeStack>>>, HQError>;
}

impl TypeStackImpl for Rc<RefCell<Option<TypeStack>>> {
    fn get(&self, i: usize) -> Result<Rc<RefCell<Option<TypeStack>>>, HQError> {
        if i == 0 {
            Ok(Rc::clone(self))
        } else {
            self.borrow().ok_or(make_hq_bug!(""))?.0.get(i - 1)
        }
    }
}

#[derive(Debug, Clone)]
pub struct IrBlock {
    pub opcode: IrOpcode,
    pub type_stack: Rc<RefCell<Option<TypeStack>>>,
}

impl IrBlock {
    pub fn new_with_inputs<F>(
        opcode: IrOpcode,
        inputs: Vec<InputType>,
        type_stack: Rc<RefCell<Option<TypeStack>>>,
        add_cast: F,
    ) -> Result<Self, HQError>
    where
        F: FnMut(usize, &InputType),
    {
        let expected_inputs = opcode.expected_inputs()?;
        if inputs.len() != expected_inputs.len() {
            hq_bug!(
                "expected {} inputs, got {}",
                expected_inputs.len(),
                inputs.len()
            );
        }
        for i in 0..inputs.len() {
            let expected = expected_inputs.get(i).ok_or(make_hq_bug!(""))?;
            let actual = inputs.get(i).ok_or(make_hq_bug!(""))?;
            if !expected.includes(actual) {
                add_cast(i, expected);
            }
        }
        let output_stack = opcode.output(inputs, type_stack);
        Ok(IrBlock {
            opcode,
            type_stack: output_stack,
        })
    }

    pub fn does_request_redraw(&self) -> bool {
        use IrOpcode::*;
        matches!(
            self.opcode(),
            looks_say
                | looks_think
                | hq_goto {
                    does_yield: true,
                    ..
                }
                | motion_gotoxy
                | pen_penDown
                | pen_clear
                | looks_switchcostumeto
                | motion_turnleft
                | motion_turnright
                | looks_setsizeto
                | looks_changesizeby
        )
    }
    pub fn is_hat(&self) -> bool {
        use IrOpcode::*;
        matches!(self.opcode, event_whenflagclicked)
    }
    pub fn opcode(&self) -> &IrOpcode {
        &self.opcode
    }
    pub fn inputs(&self) -> &Vec<InputType> {
        &self.inputs
    }
    pub fn output(&self) -> &OutputType {
        &self.output
    }
}

#[derive(Clone, Debug, PartialEq)]
pub enum InputType {
    Any,
    String,
    Number,
    Float,
    Integer,
    Boolean,
    ConcreteInteger,
    Unknown,
    Union(Box<InputType>, Box<InputType>),
}

impl InputType {
    fn base_type(&self) -> InputType {
        use InputType::*;
        match self {
            Any => Union(Box::new(String), Box::new(Number)).base_type(),
            Number => Union(Box::new(Float), Box::new(Integer)).base_type(),
            Integer => Union(Box::new(Boolean), Box::new(ConcreteInteger)).base_type(),
            Union(a, b) => Union(Box::new(a.base_type()), Box::new(b.base_type())),
            _ => self.clone(),
        }
    }

    fn includes(&self, other: &Self) -> bool {
        if self.base_type() == other.base_type() {
            true
        } else if let InputType::Union(a, b) = self {
            a.includes(other) || b.includes(other)
        } else {
            false
        }
    }
}

type OutputType = Option<InputType>;

impl IrOpcode {
    pub fn does_request_redraw(&self) -> bool {
        use IrOpcode::*;
        matches!(self, looks_say | looks_think)
    }

    pub fn expected_inputs(&self) -> Result<Vec<InputType>, HQError> {
        use InputType::*;
        use IrOpcode::*;
        Ok(match self {
            operator_add | operator_subtract | operator_multiply | operator_divide
            | operator_mod | operator_random => vec![Number, Number],
            operator_round | operator_mathop { .. } => vec![Number],
            looks_say | looks_think | data_setvariableto { .. } => vec![Any],
            math_integer { .. }
            | math_angle { .. }
            | math_whole_number { .. }
            | math_positive_number { .. }
            | math_number { .. }
            | sensing_timer
            | looks_size
            | data_variable { .. }
            | text { .. } => vec![],
            operator_lt | operator_gt => vec![Number, Number],
            operator_equals => vec![Any, Any],
            operator_and | operator_or => vec![Boolean, Boolean],
            operator_not => vec![Boolean],
            operator_join | operator_contains => vec![String, String],
            operator_letter_of => vec![Number, String],
            operator_length => vec![String],
            hq_goto { .. }
            | sensing_resettimer
            | pen_clear
            | pen_stamp
            | pen_penDown
            | pen_penUp => vec![],
            hq_goto_if { .. } => vec![Boolean],
            hq_drop(n) => vec![Any; *n],
            hq_cast(ty) => vec![*ty],
            data_teevariable { .. } => vec![Any],
            pen_setPenColorToColor
            | pen_changePenSizeBy
            | pen_setPenSizeTo
            | pen_setPenShadeToNumber
            | pen_changePenShadeBy
            | pen_setPenHueToNumber
            | pen_changePenHueBy
            | looks_setsizeto
            | looks_changesizeby
            | motion_turnleft
            | motion_turnright
            // todo: looks_switchcostumeto waiting on generic monomorphisation to work properly
            | looks_switchcostumeto => vec![Any],
            pen_changePenColorParamBy | pen_setPenColorParamTo => vec![String, Number],
            motion_gotoxy => vec![Number, Number],
            _ => hq_todo!("{:?}", &self),
        })
    }

    pub fn output(
        &self,
        inputs: Vec<InputType>,
        type_stack: Rc<RefCell<Option<TypeStack>>>,
    ) -> Result<Rc<RefCell<Option<TypeStack>>>, HQError> {
        use InputType::*;
        use IrOpcode::*;
        let expected_inputs = self.expected_inputs()?;
        if inputs.len() != expected_inputs.len() {
            hq_bug!(
                "expected {} inputs, got {}",
                expected_inputs.len(),
                inputs.len()
            );
        }
        let get_input = |i| inputs.get(i).ok_or(make_hq_bug!(""));
        let output = match self {
            data_teevariable { .. } => Ok(TypeStack::new_some(TypeStack(
                Rc::clone(&type_stack),
                get_input(0)?.clone(),
            ))),
            hq_cast(ty) => Ok(TypeStack::new_some(TypeStack(
                Rc::clone(&type_stack.borrow().unwrap().0),
                ty.clone(),
            ))),
            operator_add | operator_subtract | operator_multiply | operator_random
            | operator_mod => Ok(TypeStack::new_some(TypeStack(
                Rc::clone(&type_stack),
                if Integer.includes(get_input(0)?) && Integer.includes(get_input(1)?) {
                    ConcreteInteger
                } else {
                    Float
                },
            ))),
            operator_divide | looks_size | sensing_timer | math_number { .. } => Ok(
                TypeStack::new_some(TypeStack(Rc::clone(&type_stack), Float)),
            ),
            data_variable => Ok(TypeStack::new_some(TypeStack(
                Rc::clone(&type_stack),
                Unknown,
            ))),
            operator_round
            | operator_length
            | math_integer { .. }
            | math_angle { .. }
            | math_whole_number { .. }
            | math_positive_number { .. } => Ok(TypeStack::new_some(TypeStack(
                Rc::clone(&type_stack),
                ConcreteInteger,
            ))),
            operator_mathop { OPERATOR } => Ok(TypeStack::new_some(TypeStack(
                Rc::clone(&type_stack),
                match OPERATOR.as_str() {
                    "CEILING" | "FLOOR" => ConcreteInteger,
                    _ => Float,
                },
            ))),
            text { .. } | operator_join | operator_letter_of => Ok(TypeStack::new_some(TypeStack(
                Rc::clone(&type_stack),
                String,
            ))),
            operator_contains | operator_and | operator_or | operator_gt | operator_lt
            | operator_equals | operator_not => Ok(TypeStack::new_some(TypeStack(
                Rc::clone(&type_stack),
                Boolean,
            ))),
            data_setvariableto { .. }
            | motion_gotoxy
            | motion_turnleft
            | motion_turnright
            | looks_switchcostumeto
            | looks_changesizeby
            | looks_setsizeto
            | looks_say
            | looks_size
            | pen_setPenColorToColor
            | pen_changePenSizeBy
            | pen_setPenSizeTo
            | pen_setPenShadeToNumber
            | pen_changePenShadeBy
            | pen_setPenHueToNumber
            | pen_changePenHueBy
            | pen_clear
            | pen_penUp
            | pen_penDown
            | pen_stamp
            | sensing_resettimer
            | hq_goto { .. }
            | hq_goto_if { .. } => Ok(Rc::clone(&type_stack)),
            hq_drop(n) => type_stack.get(*n),
            _ => hq_todo!("{:?}", &self),
        };
        output
    }
}

#[derive(Debug, Clone, PartialEq)]
pub struct IrVar {
    id: String,
    name: String,
    initial_value: VarVal,
    is_cloud: bool,
}

impl IrVar {
    pub fn new(id: String, name: String, initial_value: VarVal, is_cloud: bool) -> Self {
        Self {
            id,
            name,
            initial_value,
            is_cloud,
        }
    }
    pub fn id(&self) -> &String {
        &self.id
    }
    pub fn name(&self) -> &String {
        &self.name
    }
    pub fn initial_value(&self) -> &VarVal {
        &self.initial_value
    }
    pub fn is_cloud(&self) -> &bool {
        &self.is_cloud
    }
}

#[derive(Debug, Clone, PartialEq, PartialOrd, Eq, Ord)]
pub enum ThreadStart {
    GreenFlag,
}

#[derive(Debug, Clone)]
pub struct Step {
    opcodes: Vec<IrBlock>,
    context: Rc<ThreadContext>,
}

impl Step {
    pub fn new(opcodes: Vec<IrBlock>, context: Rc<ThreadContext>) -> Step {
        Step { opcodes, context }
    }
    pub fn opcodes(&self) -> &Vec<IrBlock> {
        &self.opcodes
    }
    pub fn opcodes_mut(&mut self) -> &mut Vec<IrBlock> {
        &mut self.opcodes
    }
    pub fn context(&self) -> Rc<ThreadContext> {
        Rc::clone(&self.context)
    }
}

#[derive(Debug, Clone, PartialEq)]
pub struct ThreadContext {
    pub target_index: u32,
    pub dbg: bool,
    pub vars: Rc<RefCell<Vec<IrVar>>>, // todo: fix variable id collisions between targets
    pub target_num: usize,
    pub costumes: Vec<IrCostume>,
}

#[derive(Debug, Clone, PartialEq)]
pub struct Thread {
    start: ThreadStart,
    first_step: String,
    target_id: String,
}

trait IrBlockVec {
    fn add_block(
        &mut self,
        block_id: String,
        blocks: &BTreeMap<String, Block>,
        context: Rc<ThreadContext>,
        last_nexts: Vec<String>,
        steps: &mut IndexMap<(String, String), Step, BuildHasherDefault<FNV1aHasher64>>,
        target_id: String,
    ) -> Result<(), HQError>;
    fn add_block_arr(&mut self, block_arr: &BlockArray) -> Result<(), HQError>;
    fn add_inputs(
        &mut self,
        inputs: &BTreeMap<String, Input>,
        blocks: &BTreeMap<String, Block>,
        context: Rc<ThreadContext>,
        steps: &mut IndexMap<(String, String), Step, BuildHasherDefault<FNV1aHasher64>>,
        target_id: String,
    ) -> Result<(), HQError>;
    fn fixup_types(&mut self) -> Result<(), HQError>;
}

impl IrBlockVec for Vec<IrBlock> {
    fn fixup_types(&mut self) -> Result<(), HQError> {
        let mut type_stack: Vec<(usize, BlockType)> = vec![];
        let mut expected_outputs: Vec<(usize, BlockType)> = vec![];
        for (index, op) in self.iter().enumerate() {
            if type_stack.len() < op.opcode().descriptor()?.inputs().len() {
                hq_bug!(
                    "type stack not big enough (expected >={} items, got {})",
                    op.opcode().descriptor()?.inputs().len(),
                    type_stack.len()
                )
            };
            for block_type in op.opcode().descriptor()?.inputs().iter().rev() {
                let top_type = type_stack
                    .pop()
                    .ok_or(make_hq_bug!("couldn't pop from type stack"))?;
                expected_outputs.push((top_type.0, *block_type))
            }
            if !matches!(op.opcode().descriptor()?.output(), BlockType::Stack) {
                type_stack.push((index, (*op.opcode().descriptor()?.output())));
            }
        }
        if !type_stack.is_empty() {
            hq_bug!(
                "type stack too big (expected 0 items at end of step, got {} ({:?}))",
                type_stack.len(),
                &type_stack,
            )
        };
        for (index, ty) in expected_outputs {
            self.get_mut(index)
                .ok_or(make_hq_bug!("ir block doesn't exist"))?
                .set_expected_output(ty);
        }
        Ok(())
    }
    fn add_inputs(
        &mut self,
        inputs: &BTreeMap<String, Input>,
        blocks: &BTreeMap<String, Block>,
        context: Rc<ThreadContext>,
        steps: &mut IndexMap<(String, String), Step, BuildHasherDefault<FNV1aHasher64>>,
        target_id: String,
    ) -> Result<(), HQError> {
        for (name, input) in inputs {
            if name.starts_with("SUBSTACK") {
                continue;
            }
            match input {
                Input::Shadow(_, maybe_block, _) | Input::NoShadow(_, maybe_block) => {
                    let Some(block) = maybe_block else {
                        hq_bad_proj!("block doesn't exist"); // is this a problem with the project, or is it a bug?
                    };
                    match block {
                        BlockArrayOrId::Id(id) => {
                            self.add_block(
                                id.clone(),
                                blocks,
                                Rc::clone(&context),
                                vec![],
                                steps,
                                target_id.clone(),
                            )?;
                        }
                        BlockArrayOrId::Array(arr) => {
                            self.add_block_arr(arr)?;
                        }
                    }
                }
            }
        }
        Ok(())
    }
    fn add_block_arr(&mut self, block_arr: &BlockArray) -> Result<(), HQError> {
        self.push(
            match block_arr {
                BlockArray::NumberOrAngle(ty, value) => match ty {
                    4 => IrOpcode::math_number { NUM: *value },
                    5 => IrOpcode::math_positive_number { NUM: *value },
                    6 => IrOpcode::math_whole_number { NUM: *value },
                    7 => IrOpcode::math_integer { NUM: *value },
                    8 => IrOpcode::math_angle { NUM: *value },
                    _ => hq_bad_proj!("bad project json (block array of type ({}, u32))", ty),
                },
                BlockArray::ColorOrString(ty, value) => match ty {
                    4 => IrOpcode::math_number {
                        NUM: value.parse().map_err(|_| make_hq_bug!(""))?,
                    },
                    5 => IrOpcode::math_positive_number {
                        NUM: value.parse().map_err(|_| make_hq_bug!(""))?,
                    },
                    6 => IrOpcode::math_whole_number {
                        NUM: value.parse().map_err(|_| make_hq_bug!(""))?,
                    },
                    7 => IrOpcode::math_integer {
                        NUM: value.parse().map_err(|_| make_hq_bug!(""))?,
                    },
                    8 => IrOpcode::math_angle {
                        NUM: value.parse().map_err(|_| make_hq_bug!(""))?,
                    },
                    9 => hq_todo!(""),
                    10 => IrOpcode::text {
                        TEXT: value.to_string(),
                    },
                    _ => hq_bad_proj!("bad project json (block array of type ({}, string))", ty),
                },
                BlockArray::Broadcast(ty, _name, id) => match ty {
                    12 => IrOpcode::data_variable {
                        VARIABLE: id.to_string(),
                    },
                    _ => hq_todo!(""),
                },
                BlockArray::VariableOrList(ty, _name, id, _pos_x, _pos_y) => match ty {
                    12 => IrOpcode::data_variable {
                        VARIABLE: id.to_string(),
                    },
                    _ => hq_todo!(""),
                },
            }
            .try_into()?,
        );
        Ok(())
    }
    fn add_block(
        &mut self,
        block_id: String,
        blocks: &BTreeMap<String, Block>,
        context: Rc<ThreadContext>,
        last_nexts: Vec<String>,
        steps: &mut IndexMap<(String, String), Step, BuildHasherDefault<FNV1aHasher64>>,
        target_id: String,
    ) -> Result<(), HQError> {
        let block = blocks.get(&block_id).ok_or(make_hq_bug!(""))?;
        match block {
            Block::Normal { block_info, .. } => {
                self.add_inputs(
                    &block_info.inputs,
                    blocks,
                    Rc::clone(&context),
                    steps,
                    target_id.clone(),
                )?;

                self.append(
                    &mut (match block_info.opcode {
                        BlockOpcode::motion_gotoxy => vec![IrOpcode::motion_gotoxy],
                        BlockOpcode::sensing_timer => vec![IrOpcode::sensing_timer],
                        BlockOpcode::sensing_resettimer => vec![IrOpcode::sensing_resettimer],
                        BlockOpcode::looks_say => vec![IrOpcode::looks_say],
                        BlockOpcode::looks_think => vec![IrOpcode::looks_think],
                        BlockOpcode::looks_show => vec![IrOpcode::looks_show],
                        BlockOpcode::looks_hide => vec![IrOpcode::looks_hide],
                        BlockOpcode::looks_hideallsprites => vec![IrOpcode::looks_hideallsprites],
                        BlockOpcode::looks_switchcostumeto => vec![IrOpcode::looks_switchcostumeto],
                        BlockOpcode::looks_costume => {
                            let val = match block_info.fields.get("COSTUME").ok_or(
                                make_hq_bad_proj!("invalid project.json - missing field COSTUME"),
                            )? {
                                Field::Value((val,)) => val,
                                Field::ValueId(val, _) => val,
                            }; /* else {
                                   hq_bad_proj!("invalid project.json - missing costume for COSTUME field");
                               };*/
                            let VarVal::String(name) = val.clone().ok_or(make_hq_bad_proj!(
                                "invalid project.json - null costume name for COSTUME field"
                            ))?
                            else {
                                hq_bad_proj!(
                                    "invalid project.json - COSTUME field is not of type String"
                                );
                            };
                            log(name.as_str());
                            let index = context
                                .costumes
                                .iter()
                                .position(|costume| costume.name == name)
                                .ok_or(make_hq_bad_proj!("missing costume with name {}", name))?;
                            log(format!("{}", index).as_str());
                            vec![IrOpcode::math_whole_number { NUM: index as f64 }]
                        }
                        BlockOpcode::looks_switchbackdropto => {
                            vec![IrOpcode::looks_switchbackdropto]
                        }
                        BlockOpcode::looks_switchbackdroptoandwait => {
                            vec![IrOpcode::looks_switchbackdroptoandwait]
                        }
                        BlockOpcode::looks_nextcostume => vec![IrOpcode::looks_nextcostume],
                        BlockOpcode::looks_nextbackdrop => vec![IrOpcode::looks_nextbackdrop],
                        BlockOpcode::looks_changeeffectby => vec![IrOpcode::looks_changeeffectby],
                        BlockOpcode::looks_seteffectto => vec![IrOpcode::looks_seteffectto],
                        BlockOpcode::looks_cleargraphiceffects => {
                            vec![IrOpcode::looks_cleargraphiceffects]
                        }
                        BlockOpcode::looks_changesizeby => vec![IrOpcode::looks_changesizeby],
                        BlockOpcode::looks_setsizeto => vec![IrOpcode::looks_setsizeto],
                        BlockOpcode::motion_turnleft => vec![IrOpcode::motion_turnleft],
                        BlockOpcode::looks_size => vec![IrOpcode::looks_size],
                        BlockOpcode::operator_add => vec![IrOpcode::operator_add],
                        BlockOpcode::operator_subtract => vec![IrOpcode::operator_subtract],
                        BlockOpcode::operator_multiply => vec![IrOpcode::operator_multiply],
                        BlockOpcode::operator_divide => vec![IrOpcode::operator_divide],
                        BlockOpcode::operator_mod => vec![IrOpcode::operator_mod],
                        BlockOpcode::operator_round => vec![IrOpcode::operator_round],
                        BlockOpcode::operator_lt => vec![IrOpcode::operator_lt],
                        BlockOpcode::operator_equals => vec![IrOpcode::operator_equals],
                        BlockOpcode::operator_gt => vec![IrOpcode::operator_gt],
                        BlockOpcode::operator_and => vec![IrOpcode::operator_and],
                        BlockOpcode::operator_or => vec![IrOpcode::operator_or],
                        BlockOpcode::operator_not => vec![IrOpcode::operator_not],
                        BlockOpcode::operator_random => vec![IrOpcode::operator_random],
                        BlockOpcode::operator_join => vec![IrOpcode::operator_join],
                        BlockOpcode::operator_letter_of => vec![IrOpcode::operator_letter_of],
                        BlockOpcode::operator_length => vec![IrOpcode::operator_length],
                        BlockOpcode::operator_contains => vec![IrOpcode::operator_contains],
                        BlockOpcode::pen_clear => vec![IrOpcode::pen_clear],
                        BlockOpcode::pen_stamp => vec![IrOpcode::pen_stamp],
                        BlockOpcode::pen_penDown => vec![IrOpcode::pen_penDown],
                        BlockOpcode::pen_penUp => vec![IrOpcode::pen_penUp],
                        BlockOpcode::pen_setPenColorToColor => {
                            vec![IrOpcode::pen_setPenColorToColor]
                        }
                        BlockOpcode::pen_changePenColorParamBy => {
                            vec![IrOpcode::pen_changePenColorParamBy]
                        }
                        BlockOpcode::pen_setPenColorParamTo => {
                            vec![IrOpcode::pen_setPenColorParamTo]
                        }
                        BlockOpcode::pen_changePenSizeBy => vec![IrOpcode::pen_changePenSizeBy],
                        BlockOpcode::pen_setPenSizeTo => vec![IrOpcode::pen_setPenSizeTo],
                        BlockOpcode::pen_setPenShadeToNumber => {
                            vec![IrOpcode::pen_setPenShadeToNumber]
                        }
                        BlockOpcode::pen_changePenShadeBy => vec![IrOpcode::pen_changePenShadeBy],
                        BlockOpcode::pen_setPenHueToNumber => vec![IrOpcode::pen_setPenHueToNumber],
                        BlockOpcode::pen_changePenHueBy => vec![IrOpcode::pen_changePenHueBy],
                        BlockOpcode::pen_menu_colorParam => {
                            let maybe_val = match block_info.fields.get("colorParam").ok_or(
                                make_hq_bad_proj!(
                                    "invalid project.json - missing field colorParam"
                                ),
                            )? {
                                Field::Value((v,)) | Field::ValueId(v, _) => v,
                            };
                            let val_varval = maybe_val.clone().ok_or(make_hq_bad_proj!(
                                "invalid project.json - null value for OPERATOR field"
                            ))?;
                            let VarVal::String(val) = val_varval else {
                                hq_bad_proj!(
                                    "invalid project.json - expected colorParam field to be string"
                                );
                            };
                            vec![IrOpcode::text { TEXT: val }]
                        }
                        BlockOpcode::operator_mathop => {
                            let maybe_val = match block_info.fields.get("OPERATOR").ok_or(
                                make_hq_bad_proj!("invalid project.json - missing field OPERATOR"),
                            )? {
                                Field::Value((v,)) | Field::ValueId(v, _) => v,
                            };
                            let val_varval = maybe_val.clone().ok_or(make_hq_bad_proj!(
                                "invalid project.json - null value for OPERATOR field"
                            ))?;
                            let VarVal::String(val) = val_varval else {
                                hq_bad_proj!(
                                    "invalid project.json - expected OPERATOR field to be string"
                                );
                            };
                            vec![IrOpcode::operator_mathop { OPERATOR: val }]
                        }
                        BlockOpcode::data_variable => {
                            let Field::ValueId(_val, maybe_id) =
                                block_info.fields.get("VARIABLE").ok_or(make_hq_bad_proj!(
                                    "invalid project.json - missing field VARIABLE"
                                ))?
                            else {
                                hq_bad_proj!(
                                    "invalid project.json - missing variable id for VARIABLE field"
                                );
                            };
                            let id = maybe_id.clone().ok_or(make_hq_bad_proj!(
                                "invalid project.json - null variable id for VARIABLE field"
                            ))?;
                            vec![IrOpcode::data_variable { VARIABLE: id }]
                        }
                        BlockOpcode::data_setvariableto => {
                            let Field::ValueId(_val, maybe_id) =
                                block_info.fields.get("VARIABLE").ok_or(make_hq_bad_proj!(
                                    "invalid project.json - missing field VARIABLE"
                                ))?
                            else {
                                hq_bad_proj!(
                                    "invalid project.json - missing variable id for VARIABLE field"
                                );
                            };
                            let id = maybe_id.clone().ok_or(make_hq_bad_proj!(
                                "invalid project.json - null variable id for VARIABLE field"
                            ))?;
                            vec![IrOpcode::data_setvariableto { VARIABLE: id }]
                        }
                        BlockOpcode::data_changevariableby => {
                            let Field::ValueId(_val, maybe_id) =
                                block_info.fields.get("VARIABLE").ok_or(make_hq_bad_proj!(
                                    "invalid project.json - missing field VARIABLE"
                                ))?
                            else {
                                hq_bad_proj!(
                                    "invalid project.json - missing variable id for VARIABLE field"
                                );
                            };
                            let id = maybe_id.clone().ok_or(make_hq_bad_proj!(
                                "invalid project.json - null id for VARIABLE field"
                            ))?;
                            vec![
                                IrOpcode::data_variable {
                                    VARIABLE: id.to_string(),
                                },
                                IrOpcode::operator_add,
                                IrOpcode::data_setvariableto { VARIABLE: id },
                            ]
                        }
                        BlockOpcode::control_if => {
                            let substack_id = if let BlockArrayOrId::Id(id) = block_info
                                .inputs
                                .get("SUBSTACK")
                                .ok_or(make_hq_bad_proj!("missing SUBSTACK input for control_if"))?
                                .get_1()
                                .ok_or(make_hq_bug!(""))?
                                .clone()
                                .ok_or(make_hq_bug!(""))?
                            {
                                id
                            } else {
                                hq_bad_proj!("malformed SUBSTACK input")
                            };
                            let mut new_nexts = last_nexts.clone();
                            if let Some(ref next) = block_info.next {
                                new_nexts.push(next.clone());
                            }
                            step_from_top_block(
                                substack_id.clone(),
                                new_nexts,
                                blocks,
                                Rc::clone(&context),
                                steps,
                                target_id.clone(),
                            )?;
                            step_from_top_block(
                                block_info.next.clone().ok_or(make_hq_bug!(""))?,
                                last_nexts,
                                blocks,
                                Rc::clone(&context),
                                steps,
                                target_id.clone(),
                            )?;
                            vec![
                                IrOpcode::hq_goto_if {
                                    step: Some((target_id.clone(), substack_id)),
                                    does_yield: false,
                                },
                                IrOpcode::hq_goto {
                                    step: if block_info.next.is_some() {
                                        Some((
                                            target_id,
                                            block_info.next.clone().ok_or(make_hq_bug!(""))?,
                                        ))
                                    } else {
                                        None
                                    },
                                    does_yield: false,
                                },
                            ]
                        }
                        BlockOpcode::control_if_else => {
                            let substack_id = if let BlockArrayOrId::Id(id) = block_info
                                .inputs
                                .get("SUBSTACK")
                                .ok_or(make_hq_bad_proj!("missing SUBSTACK input for control_if"))?
                                .get_1()
                                .ok_or(make_hq_bug!(""))?
                                .clone()
                                .ok_or(make_hq_bug!(""))?
                            {
                                id
                            } else {
                                hq_bad_proj!("malformed SUBSTACK input")
                            };
                            let substack2_id = if let BlockArrayOrId::Id(id) = block_info
                                .inputs
                                .get("SUBSTACK2")
                                .ok_or(make_hq_bad_proj!("missing SUBSTACK input for control_if"))?
                                .get_1()
                                .ok_or(make_hq_bug!(""))?
                                .clone()
                                .ok_or(make_hq_bug!(""))?
                            {
                                id
                            } else {
                                hq_bad_proj!("malformed SUBSTACK2 input")
                            };
                            let mut new_nexts = last_nexts;
                            if let Some(ref next) = block_info.next {
                                new_nexts.push(next.clone());
                            }
                            step_from_top_block(
                                substack_id.clone(),
                                new_nexts.clone(),
                                blocks,
                                Rc::clone(&context),
                                steps,
                                target_id.clone(),
                            )?;
                            step_from_top_block(
                                substack2_id.clone(),
                                new_nexts.clone(),
                                blocks,
                                Rc::clone(&context),
                                steps,
                                target_id.clone(),
                            )?;
                            vec![
                                IrOpcode::hq_goto_if {
                                    step: Some((target_id.clone(), substack_id)),
                                    does_yield: false,
                                },
                                IrOpcode::hq_goto {
                                    step: Some((target_id, substack2_id)),
                                    does_yield: false,
                                },
                            ]
                        }
                        BlockOpcode::control_repeat => {
                            let substack_id = if let BlockArrayOrId::Id(id) = block_info
                                .inputs
                                .get("SUBSTACK")
                                .ok_or(make_hq_bad_proj!("missing SUBSTACK input for control_if"))?
                                .get_1()
                                .ok_or(make_hq_bug!(""))?
                                .clone()
                                .ok_or(make_hq_bug!(""))?
                            {
                                id
                            } else {
                                hq_bad_proj!("malformed SUBSTACK input")
                            };
                            let mut condition_opcodes = vec![
                                IrOpcode::hq_goto_if {
                                    step: Some((
                                        target_id.clone(),
                                        block_info.next.clone().ok_or(make_hq_bug!(""))?,
                                    )),
                                    does_yield: true,
                                }
                                .try_into()?,
                                IrOpcode::hq_goto {
                                    step: Some((target_id.clone(), substack_id.clone())),
                                    does_yield: true,
                                }
                                .try_into()?,
                            ];
                            let looper_id = Uuid::new_v4().to_string();
                            context.vars.borrow_mut().push(IrVar::new(
                                looper_id.clone(),
                                looper_id.clone(),
                                VarVal::Float(0.0),
                                false,
                            ));
                            if !steps.contains_key(&(target_id.clone(), looper_id.clone())) {
                                let mut looper_opcodes = vec![
                                    IrOpcode::data_variable {
                                        VARIABLE: looper_id.clone(),
                                    }
                                    .try_into()?,
                                    IrOpcode::math_number { NUM: 1.0 }.try_into()?,
                                    IrOpcode::operator_subtract.try_into()?,
                                    IrOpcode::data_teevariable {
                                        VARIABLE: looper_id.clone(),
                                    }
                                    .try_into()?,
                                    IrOpcode::math_number { NUM: 1.0 }.try_into()?,
                                    IrOpcode::operator_lt.try_into()?,
                                ];
                                //looper_opcodes.add_inputs(&block_info.inputs, blocks, Rc::clone(&context), steps, target_id.clone());
                                looper_opcodes.append(&mut condition_opcodes.clone());
                                looper_opcodes.fixup_types()?;
                                steps.insert(
                                    (target_id.clone(), looper_id.clone()),
                                    Step::new(looper_opcodes, Rc::clone(&context)),
                                );
                            }
                            step_from_top_block(
                                block_info.next.clone().ok_or(make_hq_bug!(""))?,
                                last_nexts,
                                blocks,
                                Rc::clone(&context),
                                steps,
                                target_id.clone(),
                            )?;
                            step_from_top_block(
                                substack_id.clone(),
                                vec![looper_id.clone()],
                                blocks,
                                Rc::clone(&context),
                                steps,
                                target_id.clone(),
                            )?;
                            let mut opcodes = vec![];
                            opcodes.add_inputs(
                                &block_info.inputs,
                                blocks,
                                Rc::clone(&context),
                                steps,
                                target_id.clone(),
                            )?;
                            opcodes.append(&mut condition_opcodes);
                            opcodes.fixup_types()?;
                            steps.insert(
                                (target_id.clone(), block_id.clone()),
                                Step::new(opcodes.clone(), Rc::clone(&context)),
                            );
                            vec![
                                IrOpcode::operator_round,
                                IrOpcode::data_teevariable {
                                    VARIABLE: looper_id,
                                },
                                IrOpcode::math_number { NUM: 1.0 },
                                IrOpcode::operator_lt,
                                IrOpcode::hq_goto_if {
                                    step: Some((
                                        target_id.clone(),
                                        block_info.next.clone().ok_or(make_hq_bug!(""))?,
                                    )),
                                    does_yield: true,
                                },
                                IrOpcode::hq_goto {
                                    step: Some((target_id, substack_id)),
                                    does_yield: false,
                                },
                            ]
                        }
                        BlockOpcode::control_repeat_until => {
                            let substack_id = if let BlockArrayOrId::Id(id) = block_info
                                .inputs
                                .get("SUBSTACK")
                                .ok_or(make_hq_bad_proj!("missing SUBSTACK input for control_if"))?
                                .get_1()
                                .ok_or(make_hq_bug!(""))?
                                .clone()
                                .ok_or(make_hq_bug!(""))?
                            {
                                id
                            } else {
                                hq_bad_proj!("malformed SUBSTACK input")
                            };
                            let mut condition_opcodes = vec![
                                IrOpcode::hq_goto_if {
                                    step: Some((
                                        target_id.clone(),
                                        block_info.next.clone().ok_or(make_hq_bug!(""))?,
                                    )),
                                    does_yield: true,
                                }
                                .try_into()?,
                                IrOpcode::hq_goto {
                                    step: Some((target_id.clone(), substack_id.clone())),
                                    does_yield: true,
                                }
                                .try_into()?,
                            ];
                            let looper_id = Uuid::new_v4().to_string();
                            if !steps.contains_key(&(target_id.clone(), looper_id.clone())) {
                                let mut looper_opcodes = vec![];
                                looper_opcodes.add_inputs(
                                    &block_info.inputs,
                                    blocks,
                                    Rc::clone(&context),
                                    steps,
                                    target_id.clone(),
                                )?;
                                looper_opcodes.append(&mut condition_opcodes.clone());
                                looper_opcodes.fixup_types()?;
                                steps.insert(
                                    (target_id.clone(), looper_id.clone()),
                                    Step::new(looper_opcodes, Rc::clone(&context)),
                                );
                            }
                            step_from_top_block(
                                block_info.next.clone().ok_or(make_hq_bug!(""))?,
                                last_nexts,
                                blocks,
                                Rc::clone(&context),
                                steps,
                                target_id.clone(),
                            )?;
                            step_from_top_block(
                                substack_id.clone(),
                                vec![looper_id],
                                blocks,
                                Rc::clone(&context),
                                steps,
                                target_id.clone(),
                            )?;
                            let mut opcodes = vec![];
                            opcodes.add_inputs(
                                &block_info.inputs,
                                blocks,
                                Rc::clone(&context),
                                steps,
                                target_id.clone(),
                            )?;
                            opcodes.append(&mut condition_opcodes);
                            opcodes.fixup_types()?;
                            steps.insert(
                                (target_id.clone(), block_id.clone()),
                                Step::new(opcodes.clone(), Rc::clone(&context)),
                            );
                            vec![
                                IrOpcode::hq_goto_if {
                                    step: Some((
                                        target_id.clone(),
                                        block_info.next.clone().ok_or(make_hq_bug!(""))?,
                                    )),
                                    does_yield: true,
                                },
                                IrOpcode::hq_goto {
                                    step: Some((target_id, substack_id)),
                                    does_yield: false,
                                },
                            ]
                        }
                        BlockOpcode::control_forever => {
                            let substack_id = if let BlockArrayOrId::Id(id) = block_info
                                .inputs
                                .get("SUBSTACK")
                                .ok_or(make_hq_bad_proj!("missing SUBSTACK input for control_if"))?
                                .get_1()
                                .ok_or(make_hq_bug!(""))?
                                .clone()
                                .ok_or(make_hq_bug!(""))?
                            {
                                id
                            } else {
                                hq_bad_proj!("malformed SUBSTACK input")
                            };
                            let mut condition_opcodes = vec![
                                //IrOpcode::hq_goto_if { step: Some((target_id.clone(), block_info.next.clone().map_err(|_| make_hq_bug!(""))?)), does_yield: true }.into(),
                                IrOpcode::hq_goto {
                                    step: Some((target_id.clone(), substack_id.clone())),
                                    does_yield: true,
                                }
                                .try_into()?,
                            ];
                            let looper_id = Uuid::new_v4().to_string();
                            if !steps.contains_key(&(target_id.clone(), looper_id.clone())) {
                                let mut looper_opcodes = vec![];
                                //looper_opcodes.add_inputs(&block_info.inputs, blocks, Rc::clone(&context), steps, target_id.clone());
                                looper_opcodes.append(&mut condition_opcodes.clone());
                                looper_opcodes.fixup_types()?;
                                steps.insert(
                                    (target_id.clone(), looper_id.clone()),
                                    Step::new(looper_opcodes, Rc::clone(&context)),
                                );
                            }
                            if let Some(next) = block_info.next.clone() {
                                step_from_top_block(
                                    next,
                                    last_nexts,
                                    blocks,
                                    Rc::clone(&context),
                                    steps,
                                    target_id.clone(),
                                )?;
                            }
                            step_from_top_block(
                                substack_id.clone(),
                                vec![looper_id],
                                blocks,
                                Rc::clone(&context),
                                steps,
                                target_id.clone(),
                            )?;
                            let mut opcodes = vec![];
                            //opcodes.add_inputs(&block_info.inputs, blocks, Rc::clone(&context), steps, target_id.clone());
                            opcodes.append(&mut condition_opcodes);
                            opcodes.fixup_types()?;
                            steps.insert(
                                (target_id.clone(), block_id.clone()),
                                Step::new(opcodes.clone(), Rc::clone(&context)),
                            );
                            vec![
                                //IrOpcode::hq_goto_if { step: Some((target_id.clone(), block_info.next.clone().map_err(|_| make_hq_bug!(""))?)), does_yield: true },
                                IrOpcode::hq_goto {
                                    step: Some((target_id, substack_id)),
                                    does_yield: false,
                                },
                            ]
                        }
                        ref other => hq_todo!("unknown block {:?}", other),
                    })
                    .into_iter()
                    .map(IrBlock::try_from)
                    .collect::<Result<_, _>>()?,
                );
            }
            Block::Special(a) => self.add_block_arr(a)?,
        };
        Ok(())
    }
}

pub fn step_from_top_block<'a>(
    top_id: String,
    mut last_nexts: Vec<String>,
    blocks: &BTreeMap<String, Block>,
    context: Rc<ThreadContext>,
    steps: &'a mut IndexMap<(String, String), Step, BuildHasherDefault<FNV1aHasher64>>,
    target_id: String,
) -> Result<&'a Step, HQError> {
    if steps.contains_key(&(target_id.clone(), top_id.clone())) {
        return steps.get(&(target_id, top_id)).ok_or(make_hq_bug!(""));
    }
    let mut ops: Vec<IrBlock> = vec![];
    let mut next_block = blocks.get(&top_id).ok_or(make_hq_bug!(""))?;
    let mut next_id = Some(top_id.clone());
    loop {
        ops.add_block(
            next_id.clone().ok_or(make_hq_bug!(""))?,
            blocks,
            Rc::clone(&context),
            last_nexts.clone(),
            steps,
            target_id.clone(),
        )?;
        if next_block
            .block_info()
            .ok_or(make_hq_bug!(""))?
            .next
            .is_none()
        {
            next_id = last_nexts.pop();
        } else {
            next_id = next_block
                .block_info()
                .ok_or(make_hq_bug!(""))?
                .next
                .clone();
        }
        if ops.is_empty() {
            hq_bug!("assertion failed: !ops.is_empty()")
        };
        if matches!(
            ops.last().ok_or(make_hq_bug!(""))?.opcode(),
            IrOpcode::hq_goto { .. }
        ) {
            next_id = None;
        }
        if next_id.is_none() {
            break;
        } else if let Some(block) = blocks.get(&next_id.clone().ok_or(make_hq_bug!(""))?) {
            next_block = block;
        } else if steps.contains_key(&(target_id.clone(), next_id.clone().ok_or(make_hq_bug!(""))?))
        {
            ops.push(
                IrOpcode::hq_goto {
                    step: Some((target_id.clone(), next_id.clone().ok_or(make_hq_bug!(""))?)),
                    does_yield: false,
                }
                .try_into()?,
            );
            next_id = None;
            break;
        } else {
            hq_bad_proj!("invalid next_id");
        }
        let Some(last_block) = ops.last() else {
            unreachable!()
        };
        if last_block.does_request_redraw()
            && !(*last_block.opcode() == IrOpcode::looks_say && context.dbg)
        {
            break;
        }
    }
    ops.fixup_types()?;
    let mut step = Step::new(ops.clone(), Rc::clone(&context));
    step.opcodes_mut().push(if let Some(ref id) = next_id {
        step_from_top_block(
            id.clone(),
            last_nexts,
            blocks,
            Rc::clone(&context),
            steps,
            target_id.clone(),
        )?;
        IrBlock::try_from(IrOpcode::hq_goto {
            step: Some((target_id.clone(), id.clone())),
            does_yield: true,
        })?
    } else {
        IrBlock::try_from(IrOpcode::hq_goto {
            step: None,
            does_yield: false,
        })?
    });
    steps.insert((target_id.clone(), top_id.clone()), step);
    steps.get(&(target_id, top_id)).ok_or(make_hq_bug!(""))
}

impl Thread {
    pub fn new(start: ThreadStart, first_step: String, target_id: String) -> Thread {
        Thread {
            start,
            first_step,
            target_id,
        }
    }
    pub fn start(&self) -> &ThreadStart {
        &self.start
    }
    pub fn first_step(&self) -> &String {
        &self.first_step
    }
    pub fn target_id(&self) -> &String {
        &self.target_id
    }
    pub fn from_hat(
        hat: Block,
        blocks: BTreeMap<String, Block>,
        context: Rc<ThreadContext>,
        steps: &mut IndexMap<(String, String), Step, BuildHasherDefault<FNV1aHasher64>>,
        target_id: String,
    ) -> Result<Thread, HQError> {
        let (first_step_id, _first_step) = if let Block::Normal { block_info, .. } = &hat {
            if let Some(next_id) = &block_info.next {
                (
                    next_id.clone(),
                    step_from_top_block(
                        next_id.clone(),
                        vec![],
                        &blocks,
                        Rc::clone(&context),
                        steps,
                        target_id.clone(),
                    )?,
                )
            } else {
                unreachable!();
            }
        } else {
            unreachable!();
        };
        let start_type = if let Block::Normal { block_info, .. } = &hat {
            match block_info.opcode {
                BlockOpcode::event_whenflagclicked => ThreadStart::GreenFlag,
                _ => hq_todo!(""),
            }
        } else {
            unreachable!()
        };
        Ok(Self::new(start_type, first_step_id, target_id))
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn create_ir() -> Result<(), HQError> {
        use crate::sb3::Sb3Project;
        use std::fs;
        let proj: Sb3Project = fs::read_to_string("./hq-test.project.json")
            .expect("couldn't read hq-test.project.json")
            .try_into()?;
        let ir: IrProject = proj.try_into()?;
        println!("{:?}", ir);
        Ok(())
    }
}
