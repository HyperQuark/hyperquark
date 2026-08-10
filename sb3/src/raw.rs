use std::collections::BTreeMap;

use arbitrary::Arbitrary;
use enum_field_getter::EnumFieldGetter;
use serde::{Deserialize, Serialize};
use serde_json::Value;

use crate::{ParseSb3Error, ParseSb3ErrorKind};

/// A scratch project.
#[derive(Serialize, Deserialize, Debug, Clone, PartialEq)]
pub struct Sb3Project {
    pub targets: Vec<Target>,
    #[serde(default)]
    pub monitors: Vec<Monitor>,
    pub extensions: Vec<Box<str>>,
    pub meta: Meta,
}

/// A comment, possibly attached to a block.
#[derive(Serialize, Deserialize, Debug, Clone, PartialEq, Arbitrary)]
#[serde(rename_all = "camelCase")]
pub struct Comment {
    pub block_id: Option<Box<str>>,
    pub x: Option<f64>,
    pub y: Option<f64>,
    pub width: f64,
    pub height: f64,
    pub minimized: bool,
    pub text: Box<str>,
}

/// A possible block opcode.
#[expect(non_camel_case_types, reason = "opcodes are snake_case")]
#[derive(Serialize, Deserialize, Debug, Clone, PartialEq, Eq, Arbitrary)]
pub enum BlockOpcode {
    control_repeat,
    control_repeat_until,
    control_while,
    control_for_each,
    control_forever,
    control_wait,
    control_wait_until,
    control_if,
    control_if_else,
    control_stop,
    control_create_clone_of,
    control_delete_this_clone,
    control_get_counter,
    control_incr_counter,
    control_clear_counter,
    control_all_at_once,
    control_start_as_clone,
    control_create_clone_of_menu,
    data_variable,
    data_setvariableto,
    data_changevariableby,
    data_hidevariable,
    data_showvariable,
    data_listcontents,
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
    event_broadcast_menu,
    event_broadcastandwait,
    event_whenflagclicked,
    event_whenkeypressed,
    event_whenthisspriteclicked,
    event_whentouchingobject,
    event_whenstageclicked,
    event_whenbackdropswitchesto,
    event_whengreaterthan,
    event_whenbroadcastreceived,
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
    motion_movesteps,
    motion_gotoxy,
    motion_goto,
    motion_goto_menu,
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
    operator_mathop,
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
}

/// A scratch block, either in normal object form or the compact array form.
#[derive(Serialize, Deserialize, Debug, Clone, EnumFieldGetter, PartialEq)]
#[serde(untagged)]
pub enum Block {
    Normal {
        #[serde(default)]
        x: i32,
        #[serde(default)]
        y: i32,
        #[serde(flatten)]
        block_info: BlockInfo,
    },
    Special(BlockArray),
}

/// A special representation of a scratch block.
#[derive(Serialize, Deserialize, Debug, Clone, PartialEq, Arbitrary)]
#[serde(untagged)]
pub enum BlockArray {
    NumberOrAngle(u32, f64),
    ColorOrString(u32, Box<str>),
    Broadcast(u32, Box<str>, Box<str>),
    VariableOrList(u32, Box<str>, Box<str>, f64, f64),
}

/// Either a block array or a block id.
#[derive(Serialize, Deserialize, Debug, Clone, PartialEq, Arbitrary)]
#[serde(untagged)]
pub enum BlockArrayOrId {
    Id(Box<str>),
    Array(BlockArray),
}

/// Possible inputs (round or predicate) in a block.
#[derive(Serialize, Deserialize, Debug, Clone, EnumFieldGetter, PartialEq, Arbitrary)]
#[serde(untagged)]
pub enum Input {
    Shadow(u32, Option<BlockArrayOrId>, Option<BlockArrayOrId>),
    NoShadow(u32, Option<BlockArrayOrId>),
}

/// Possible fields (rectangular) in a block.
#[derive(Serialize, Deserialize, Debug, Clone, EnumFieldGetter, PartialEq, Arbitrary)]
#[serde(untagged)]
pub enum Field {
    Value((Option<VarVal>,)),
    ValueId(Option<VarVal>, Option<Box<str>>),
}

impl Field {
    #[must_use]
    pub const fn get_0(&self) -> Option<&VarVal> {
        match self {
            Self::ValueId(val, _) | Self::Value((val,)) => val.as_ref(),
        }
    }
}

/// Represents a mutation on a block.
#[derive(Serialize, Deserialize, Debug, Clone, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub struct Mutation {
    pub tag_name: Box<str>,
    #[serde(default)]
    pub children: Vec<()>,
    #[serde(flatten)]
    pub mutations: BTreeMap<Box<str>, Value>,
}

impl Default for Mutation {
    fn default() -> Self {
        Self {
            tag_name: "mutation".into(),
            children: vec![],
            mutations: BTreeMap::new(),
        }
    }
}

/// Represents a non-special block.
#[derive(Serialize, Deserialize, Debug, Clone, PartialEq)]
#[serde(rename_all = "camelCase")]
pub struct BlockInfo {
    pub opcode: BlockOpcode,
    pub next: Option<Box<str>>,
    pub parent: Option<Box<str>>,
    pub inputs: BTreeMap<Box<str>, Input>,
    pub fields: BTreeMap<Box<str>, Field>,
    pub shadow: bool,
    pub top_level: bool,
    #[serde(default)]
    pub mutation: Mutation,
}

#[derive(Serialize, Deserialize, Debug, Clone, Copy, PartialEq, Eq, Arbitrary)]
#[expect(non_camel_case_types, reason = "lowercase in project.json")]
pub enum CostumeDataFormat {
    png,
    svg,
    jpeg,
    jpg,
    bmp,
    gif,
}

#[derive(Serialize, Deserialize, Debug, Clone, PartialEq, Arbitrary)]
#[serde(rename_all = "camelCase")]
pub struct Costume {
    pub asset_id: Box<str>,
    pub name: Box<str>,
    pub md5ext: Box<str>,
    pub data_format: CostumeDataFormat,
    #[serde(default)]
    pub bitmap_resolution: f64,
    pub rotation_center_x: f64,
    pub rotation_center_y: f64,
}

#[derive(Serialize, Deserialize, Debug, Clone, PartialEq, Arbitrary)]
#[serde(rename_all = "camelCase")]
pub struct Sound {
    pub asset_id: Box<str>,
    pub name: Box<str>,
    pub md5ext: Box<str>,
    pub data_format: Box<str>,
    pub rate: f64,
    pub sample_count: f64,
    pub format: Option<Box<str>>,
}

#[derive(Serialize, Deserialize, Debug, Clone, PartialEq, Arbitrary)]
#[serde(untagged)]
pub enum VarVal {
    Int(i32),
    Float(f64),
    Bool(bool),
    String(Box<str>),
}

#[derive(Serialize, Deserialize, Debug, Clone, PartialEq, EnumFieldGetter, Arbitrary)]
#[serde(untagged)]
pub enum VariableInfo {
    CloudVar(Box<str>, VarVal, bool),
    LocalVar(Box<str>, VarVal),
}

pub type BlockMap = BTreeMap<Box<str>, Block>;

#[derive(Serialize, Deserialize, Debug, Clone, PartialEq)]
#[serde(rename_all = "camelCase")]
pub struct Target {
    pub is_stage: bool,
    pub name: Box<str>,
    pub variables: BTreeMap<Box<str>, VariableInfo>,
    pub lists: BTreeMap<Box<str>, (Box<str>, Vec<VarVal>)>,
    #[serde(default)]
    pub broadcasts: BTreeMap<Box<str>, Box<str>>,
    pub blocks: BlockMap,
    pub comments: BTreeMap<Box<str>, Comment>,
    pub current_costume: u32,
    pub costumes: Vec<Costume>,
    pub sounds: Vec<Sound>,
    #[serde(default)]
    pub layer_order: i32,
    #[serde(default)]
    pub volume: f64,
    #[serde(default)]
    pub tempo: f64,
    #[serde(default)]
    pub video_state: Option<Box<str>>,
    #[serde(default)]
    pub video_transparency: f64,
    #[serde(default)]
    pub text_to_speech_language: Option<Box<str>>,
    #[serde(default)]
    pub visible: bool,
    #[serde(default)]
    pub x: f64,
    #[serde(default)]
    pub y: f64,
    #[serde(default)]
    pub size: f64,
    #[serde(default)]
    pub direction: f64,
    #[serde(default)]
    pub draggable: bool,
    #[serde(default)]
    pub rotation_style: Box<str>,
}

#[derive(Serialize, Deserialize, Debug, Clone, PartialEq, Arbitrary)]
#[serde(untagged)]
pub enum ListMonitorValue {
    List(Vec<VarVal>),
    String(Box<str>),
    Number(f64),
}

#[derive(Serialize, Deserialize, Debug, Clone, EnumFieldGetter, PartialEq, Arbitrary)]
#[serde(untagged)]
pub enum Monitor {
    #[serde(rename_all = "camelCase")]
    ListMonitor {
        id: Box<str>,
        mode: Box<str>,
        opcode: Box<str>,
        params: BTreeMap<Box<str>, Box<str>>,
        sprite_name: Option<Box<str>>,
        width: Option<f64>,
        height: Option<f64>,
        x: f64,
        y: f64,
        visible: bool,
        value: ListMonitorValue,
    },
    #[serde(rename_all = "camelCase")]
    VarMonitor {
        id: Box<str>,
        mode: Box<str>,
        opcode: Box<str>,
        params: BTreeMap<Box<str>, Box<str>>,
        sprite_name: Option<Box<str>>,
        value: VarVal,
        width: Option<f64>,
        height: Option<f64>,
        x: f64,
        y: f64,
        visible: bool,
        slider_min: f64,
        slider_max: f64,
        is_discrete: bool,
    },
}

#[derive(Serialize, Deserialize, Debug, Clone, PartialEq, Arbitrary)]
pub struct Meta {
    pub semver: Box<str>,
    pub vm: Box<str>,
    pub agent: Box<str>,
}

impl TryFrom<String> for Sb3Project {
    type Error = ParseSb3Error;

    fn try_from(string: String) -> Result<Self, Self::Error> {
        Self::try_from(string.as_str())
    }
}

impl TryFrom<&str> for Sb3Project {
    type Error = ParseSb3Error;

    fn try_from(string: &str) -> Result<Self, Self::Error> {
        use serde_json::error::Category;

        let sb3: Result<Self, serde_json::Error> = serde_json::from_str(string);
        match sb3 {
            Ok(project) => Ok(project),
            Err(err) => {
                let kind = match err.classify() {
                    Category::Syntax => ParseSb3ErrorKind::Syntax,
                    Category::Data => ParseSb3ErrorKind::Data,
                    Category::Eof => ParseSb3ErrorKind::Eof,
                    Category::Io => ParseSb3ErrorKind::Io,
                };
                Err(ParseSb3Error {
                    kind,
                    line: err.line(),
                    column: err.column(),
                    message: err.to_string(),
                })
            }
        }
    }
}
