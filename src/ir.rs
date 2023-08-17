// intermediate representation
use crate::sb3::{
    Block, BlockArray, BlockArrayOrId, BlockOpcode, Field, Input, Sb3Project, VarVal,
    VariableInfo,
};
use alloc::collections::BTreeMap;
use alloc::rc::Rc;
use alloc::string::{String, ToString};
use alloc::vec::Vec;
use ordered_float::OrderedFloat;

#[derive(Debug)]
pub struct IrProject {
    pub threads: Vec<Thread>,
    pub vars: Rc<Vec<IrVar>>,
    pub targets: Vec<String>,
}

impl From<Sb3Project> for IrProject {
    fn from(sb3: Sb3Project) -> IrProject {
        let vars: Rc<Vec<IrVar>> = Rc::new(
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
                    target_index: target_index.try_into().unwrap(),
                    dbg: target
                        .comments
                        .clone()
                        .iter()
                        .any(|(_id, comment)| {
                            matches!(comment.block_id.clone(), Some(d) if &d == id)
                                && comment.text.clone() == *"hq-dbg"
                        }),
                    vars: Rc::clone(&vars),
                });
                let thread =
                    Thread::from_hat(block.clone(), target.blocks.clone(), context);
                threads.push(thread);
            }
        }
        let targets = sb3
            .targets
            .iter()
            .map(|t| t.name.clone())
            .collect::<Vec<_>>();
        Self {
            vars,
            threads,
            targets,
        }
    }
}

#[allow(non_camel_case_types)]
#[allow(non_snake_case)]
#[derive(Debug, Clone, PartialEq, Eq, Hash)]
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
    hq_drop(usize),
    hq_goto {
        step: Option<Rc<Step>>,
        does_yield: bool,
    },
    hq_goto_if {
        step: Option<Rc<Step>>,
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
    looks_costume,
    looks_backdrops,
    math_angle {
        NUM: OrderedFloat<f64>,
    },
    math_integer {
        NUM: OrderedFloat<f64>,
    },
    math_number {
        NUM: OrderedFloat<f64>,
    },
    math_positive_number {
        NUM: OrderedFloat<f64>,
    },
    math_whole_number {
        NUM: OrderedFloat<f64>,
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
    pen_menu_colorParam,
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

#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub struct IrBlock {
    pub opcode: IrOpcode,
    pub actual_output: BlockType,   // the output type the block produces
    pub expected_output: BlockType, // the output type the parent block wants
}

impl From<IrOpcode> for IrBlock {
    fn from(opcode: IrOpcode) -> Self {
        let descriptor = opcode.descriptor();
        let output = descriptor.output();
        Self {
            opcode,
            actual_output: output.clone(),
            expected_output: output.clone(),
        }
    }
}

impl IrBlock {
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
        )
    }
    pub fn is_hat(&self) -> bool {
        use IrOpcode::*;
        matches!(self.opcode, event_whenflagclicked)
    }
    pub fn opcode(&self) -> &IrOpcode {
        &self.opcode
    }
    pub fn expected_output(&self) -> &BlockType {
        &self.expected_output
    }
    pub fn actual_output(&self) -> &BlockType {
        &self.actual_output
    }
    pub fn set_expected_output(&mut self, ty: BlockType) {
        self.expected_output = ty;
    }
}

#[derive(Clone, Debug, PartialEq, Eq, Hash)]
pub enum BlockType {
    Text,
    Number,
    Boolean,
    // `Any` could be any one of Text, Boolean or Number
    // only to be used when the output type is unknown or needs to preserved,
    // or where values are being passed to js (ie strings) or the type must be preserved (ie variables)
    Any,
    // `Stack` is no output (a stack block) or the input of a branch (eg in an if/else block)
    Stack,
}

#[derive(Debug, PartialEq, Eq, Hash)]
pub struct BlockDescriptor {
    inputs: Vec<BlockType>,
    output: BlockType,
}

impl BlockDescriptor {
    pub fn new(inputs: Vec<BlockType>, output: BlockType) -> Self {
        Self { inputs, output }
    }
    pub fn inputs(&self) -> &Vec<BlockType> {
        &self.inputs
    }
    pub fn output(&self) -> &BlockType {
        &self.output
    }
}

impl IrOpcode {
    pub fn does_request_redraw(&self) -> bool {
        use IrOpcode::*;
        matches!(self, looks_say | looks_think)
    }
    pub fn descriptor(&self) -> BlockDescriptor {
        use BlockType::*;
        use IrOpcode::*;
        match self {
            operator_add | operator_subtract | operator_multiply | operator_divide
            | operator_mod | operator_random => BlockDescriptor::new(vec![Number, Number], Number),
            operator_round | operator_mathop { .. } => BlockDescriptor::new(vec![Number], Number),
            looks_say | looks_think => BlockDescriptor::new(vec![Any], Stack),
            math_number { .. }
            | math_integer { .. }
            | math_angle { .. }
            | math_whole_number { .. }
            | math_positive_number { .. } => BlockDescriptor::new(vec![], Number),
            data_variable { .. } => BlockDescriptor::new(vec![], Any),
            data_setvariableto { .. } => BlockDescriptor::new(vec![Any], Stack),
            //data_changevariableby { .. } => BlockDescriptor::new(vec![Number], Stack),
            text { .. } => BlockDescriptor::new(vec![], Text),
            operator_lt | operator_gt => BlockDescriptor::new(vec![Number, Number], Boolean),
            operator_equals | operator_contains => BlockDescriptor::new(vec![Any, Any], Boolean),
            operator_and | operator_or => BlockDescriptor::new(vec![Boolean, Boolean], Boolean),
            operator_not => BlockDescriptor::new(vec![Boolean], Boolean),
            operator_join => BlockDescriptor::new(vec![Any, Any], Text),
            operator_letter_of => BlockDescriptor::new(vec![Number, Any], Text),
            operator_length => BlockDescriptor::new(vec![Any], Number),
            hq_goto { .. } => BlockDescriptor::new(vec![], Stack),
            hq_goto_if { .. } => BlockDescriptor::new(vec![Boolean], Stack),
            hq_drop(n) => BlockDescriptor::new(vec![Any; *n], Stack),
            _ => todo!("{:?}", &self),
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Hash)]
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

#[derive(Debug, Clone, PartialEq, Eq, Hash)]
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

#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub struct ThreadContext {
    pub target_index: u32,
    pub dbg: bool,
    pub vars: Rc<Vec<IrVar>>, // hopefully there can't be two variables with the same id in differwnt sprites, otherwise this will break horrendously
}

#[derive(Debug, Clone, PartialEq)]
pub struct Thread {
    start: ThreadStart,
    first_step: Rc<Step>,
}

trait IrBlockVec {
    fn add_block(
        &mut self,
        block_id: String,
        blocks: &BTreeMap<String, Block>,
        context: Rc<ThreadContext>,
        last_nexts: Vec<String>,
    );
    fn add_block_arr(&mut self, block_arr: &BlockArray);
}

impl IrBlockVec for Vec<IrBlock> {
    fn add_block_arr(&mut self, block_arr: &BlockArray) {
        self.push(
            match block_arr {
                BlockArray::NumberOrAngle(ty, value) => match ty {
                    4 => IrOpcode::math_number { NUM: *value },
                    5 => IrOpcode::math_positive_number { NUM: *value },
                    6 => IrOpcode::math_whole_number { NUM: *value },
                    7 => IrOpcode::math_integer { NUM: *value },
                    8 => IrOpcode::math_angle { NUM: *value },
                    _ => panic!("bad project json (block array of type ({}, u32))", ty),
                },
                BlockArray::ColorOrString(ty, value) => match ty {
                    4 => IrOpcode::math_number {
                        NUM: value.parse().unwrap(),
                    },
                    5 => IrOpcode::math_positive_number {
                        NUM: value.parse().unwrap(),
                    },
                    6 => IrOpcode::math_whole_number {
                        NUM: value.parse().unwrap(),
                    },
                    7 => IrOpcode::math_integer {
                        NUM: value.parse().unwrap(),
                    },
                    8 => IrOpcode::math_angle {
                        NUM: value.parse().unwrap(),
                    },
                    9 => todo!(),
                    10 => IrOpcode::text {
                        TEXT: value.to_string(),
                    },
                    _ => panic!("bad project json (block array of type ({}, string))", ty),
                },
                BlockArray::Broadcast(ty, _name, id) => match ty {
                    12 => IrOpcode::data_variable {
                        VARIABLE: id.to_string(),
                    },
                    _ => todo!(),
                },
                BlockArray::VariableOrList(ty, _name, id, _pos_x, _pos_y) => match ty {
                    12 => IrOpcode::data_variable {
                        VARIABLE: id.to_string(),
                    },
                    _ => todo!(),
                },
            }
            .into(),
        )
    }
    fn add_block(
        &mut self,
        block_id: String,
        blocks: &BTreeMap<String, Block>,
        context: Rc<ThreadContext>,
        last_nexts: Vec<String>,
    ) {
        let block = blocks.get(&block_id).unwrap();
        match block {
            Block::Normal { block_info, .. } => {
                //println!("{}: {:?}", &block_id, &block_info.opcode);
                for (name, input) in &block_info.inputs {
                    if matches!(name.as_str(), "SUBSTACK" | "SUBSTACK2") {
                        continue;
                    }
                    match input {
                        Input::Shadow(_, maybe_block, _) | Input::NoShadow(_, maybe_block) => {
                            let Some(block) = maybe_block else { panic!("block doest exist"); };
                            match block {
                                BlockArrayOrId::Id(id) => {
                                    self.add_block(
                                        id.clone(),
                                        blocks,
                                        Rc::clone(&context),
                                        last_nexts.clone(), // this probably isn't needed bc inputs donpt have a next? passing an empty vec would save some memory and overhead
                                    );
                                }
                                BlockArrayOrId::Array(arr) => {
                                    self.add_block_arr(arr);
                                }
                            }
                        }
                    }
                }

                self.append(&mut (match block_info.opcode {
                    BlockOpcode::looks_say => vec![IrOpcode::looks_say],
                    BlockOpcode::looks_think => vec![IrOpcode::looks_think],
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
                    BlockOpcode::operator_mathop => {
                        let maybe_val = match block_info.fields.get("OPERATOR")
                            .expect("invalid project.json - missing field OPERATOR (E038)") {
                            Field::Value((v,)) | Field::ValueId(v, _) => v,
                        };
                        let val_varval = maybe_val.clone().expect("invalid project.json - null value for OPERATOR field (E039)");
                        let VarVal::String(val) = val_varval else {
                            panic!("invalid project.json - expected OPERATOR field to be string (E040)");
                        };
                        vec![IrOpcode::operator_mathop {
                            OPERATOR: val,
                        }]
                    },
                    BlockOpcode::data_variable => {
                        let Field::ValueId(_val, maybe_id) = block_info.fields.get("VARIABLE")
                            .expect("invalid project.json - missing field VARIABLE (E023)") else {
                                panic!("invalid project.json - missing variable id for VARIABLE field (E024)");
                            };
                        let id = maybe_id.clone().expect("invalid project.json - null variable id for VARIABLE field (E025)");
                        vec![IrOpcode::data_variable {
                          VARIABLE: id,
                        }]
                    },
                    BlockOpcode::data_setvariableto => {
                        let Field::ValueId(_val, maybe_id) = block_info.fields.get("VARIABLE")
                            .expect("invalid project.json - missing field VARIABLE (E026)") else {
                                panic!("invalid project.json - missing variable id for VARIABLE field (E027)");
                            };
                        let id = maybe_id.clone().expect("invalid project.json - null variable id for VARIABLE field (E028)");
                        vec![IrOpcode::data_setvariableto {
                          VARIABLE: id,
                        }]
                    },
                    BlockOpcode::data_changevariableby => {
                        let Field::ValueId(_val, maybe_id) = block_info.fields.get("VARIABLE")
                            .expect("invalid project.json - missing field VARIABLE (E029)") else {
                                panic!("invalid project.json - missing variable id for VARIABLE field (E030)");
                            };
                        let id = maybe_id.clone().expect("invalid project.json - null id for VARIABLE field (E031)");
                        vec![
                          IrOpcode::data_variable {
                            VARIABLE: id.to_string(),
                          },
                          IrOpcode::operator_add,
                          IrOpcode::data_setvariableto {
                            VARIABLE: id,
                          }
                        ]
                    },
                    BlockOpcode::control_if => {
                        let substack_id = if let BlockArrayOrId::Id(id) = block_info.inputs.get("SUBSTACK").expect("missing SUBSTACK input for control_if").get_1().unwrap().clone().unwrap() { id } else { panic!("malformed SUBSTACK input") };
                        let mut new_nexts = last_nexts.clone();
                        if let Some(ref next) = block_info.next {
                            new_nexts.push(next.clone());
                        }
                        vec![IrOpcode::hq_goto_if { step: Some(step_from_top_block(substack_id, new_nexts, blocks, Rc::clone(&context))), does_yield: false, }, IrOpcode::hq_goto { step: if block_info.next.is_some() { Some(step_from_top_block(block_info.next.clone().unwrap(), last_nexts, blocks, Rc::clone(&context))) } else { None }, does_yield: false, }]
                    }
                    BlockOpcode::control_if_else => {
                        let substack_id = if let BlockArrayOrId::Id(id) = block_info.inputs.get("SUBSTACK").expect("missing SUBSTACK input for control_if").get_1().unwrap().clone().unwrap() { id } else { panic!("malformed SUBSTACK input") };
                        let substack2_id = if let BlockArrayOrId::Id(id) = block_info.inputs.get("SUBSTACK2").expect("missing SUBSTACK input for control_if").get_1().unwrap().clone().unwrap() { id } else { panic!("malformed SUBSTACK2 input") };
                        let mut new_nexts = last_nexts.clone();
                        if let Some(ref next) = block_info.next {
                            new_nexts.push(next.clone());
                        }
                        vec![IrOpcode::hq_goto_if { step: Some(step_from_top_block(substack_id, new_nexts.clone(), blocks, Rc::clone(&context))), does_yield: false, }, IrOpcode::hq_goto { step: Some(step_from_top_block(substack2_id, new_nexts.clone(), blocks, Rc::clone(&context))), does_yield: false, }]
                    }
                    _ => todo!(),
                }).into_iter().map(IrBlock::from).collect());
            }
            Block::Special(a) => self.add_block_arr(a),
        };
    }
}

pub fn step_from_top_block(
    top_id: String,
    mut last_nexts: Vec<String>,
    blocks: &BTreeMap<String, Block>,
    context: Rc<ThreadContext>,
) -> Rc<Step> {
    let mut ops: Vec<IrBlock> = vec![];
    let mut next_block = blocks.get(&top_id).unwrap();
    let mut next_id = Some(top_id);
    loop {
        ops.add_block(next_id.clone().unwrap(), blocks, Rc::clone(&context), last_nexts.clone());
        if next_block.block_info().unwrap().next.is_none() {
            next_id = last_nexts.pop();
        } else {
            next_id = next_block.block_info().unwrap().next.clone();
        }
        assert!(!ops.is_empty());
        if matches!(ops.last().unwrap().opcode(), IrOpcode::hq_goto { .. }) {
          next_id = None;
        }
        if next_id.is_none() {
            break;
        } else {
            next_block = blocks.get(&next_id.clone().unwrap()).unwrap();
        }
        let Some(last_block) = ops.last() else { unreachable!() };
        if last_block.does_request_redraw()
            && !(*last_block.opcode() == IrOpcode::looks_say && context.dbg)
        {
            break;
        }
    }
    let mut type_stack: Vec<(usize, BlockType)> = vec![];
    let mut expected_outputs: Vec<(usize, BlockType)> = vec![];
    for (index, op) in ops.iter().enumerate() {
        assert!(
            type_stack.len() >= op.opcode().descriptor().inputs().len(),
            "type stack not big enough (expected >={} items, got {}) (E019)",
            op.opcode().descriptor().inputs().len(),
            type_stack.len()
        );
        for block_type in op.opcode().descriptor().inputs().iter().rev() {
            let top_type = type_stack
                .pop()
                .expect("couldn't pop from type stack (E020)");
            expected_outputs.push((top_type.0, block_type.clone()))
        }
        if !matches!(op.opcode().descriptor().output(), BlockType::Stack) {
            type_stack.push((index, (*op.opcode().descriptor().output()).clone()));
        }
    }
    assert!(
        type_stack.is_empty(),
        "type stack too big (expected 0 items at end of step, got {})",
        type_stack.len()
    );
    for (index, ty) in expected_outputs {
        ops.get_mut(index)
            .expect("ir block doesn't exist (E043)")
            .set_expected_output(ty.clone());
    }
    let mut step = Step::new(ops.clone(), Rc::clone(&context));
    step.opcodes_mut().push(if let Some(ref id) = next_id.clone() {
        IrBlock::from(IrOpcode::hq_goto {
            step: Some(Rc::clone(&step_from_top_block(
                id.clone(),
                last_nexts,
                blocks,
                Rc::clone(&context),
            ))),
            does_yield: true,
        })
    } else {
        IrBlock::from(IrOpcode::hq_goto {
            step: None,
            does_yield: false,
        })
    });
    Rc::from(step)
}

impl Thread {
    pub fn new(start: ThreadStart, first_step: Rc<Step>) -> Thread {
        Thread {
            start,
            first_step: Rc::clone(&first_step),
        }
    }
    pub fn start(&self) -> &ThreadStart {
        &self.start
    }
    pub fn first_step(&self) -> Rc<Step> {
        Rc::clone(&self.first_step)
    }
    pub fn from_hat(
        hat: Block,
        blocks: BTreeMap<String, Block>,
        context: Rc<ThreadContext>,
    ) -> Thread {
        let first_step = if let Block::Normal { block_info, .. } = &hat {
            if let Some(next_id) = &block_info.next {
                step_from_top_block(next_id.clone(), vec![], &blocks, Rc::clone(&context))
            } else {
                unreachable!();
            }
        } else {
            unreachable!();
        };
        let start_type = if let Block::Normal { block_info, .. } = &hat {
            match block_info.opcode {
                BlockOpcode::event_whenflagclicked => ThreadStart::GreenFlag,
                _ => todo!(),
            }
        } else {
            unreachable!()
        };
        Self::new(start_type, first_step)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn create_ir() {
        use crate::sb3::Sb3Project;
        use std::fs;
        let proj: Sb3Project = fs::read_to_string("./hq-test.project.json")
            .expect("couldn't read hq-test.project.json")
            .try_into()
            .expect("invalid project.json");
        let ir: IrProject = proj.into();
        println!("{:?}", ir);
    }
}
