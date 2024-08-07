use crate::error::HQError;
use alloc::collections::BTreeMap;
use alloc::string::String;
use alloc::vec::Vec;
use enum_field_getter::EnumFieldGetter;
use serde::{Deserialize, Serialize};
use serde_json::Value;

#[derive(Serialize, Deserialize, Debug, Clone)]
pub struct Sb3Project {
    pub targets: Vec<Target>,
    pub monitors: Vec<Monitor>,
    pub extensions: Vec<String>,
    pub meta: Meta,
}

#[derive(Serialize, Deserialize, Debug, Clone)]
#[serde(rename_all = "camelCase")]
pub struct Comment {
    pub block_id: Option<String>,
    pub x: Option<f64>,
    pub y: Option<f64>,
    pub width: f64,
    pub height: f64,
    pub minimized: bool,
    pub text: String,
}

#[allow(non_camel_case_types)]
#[derive(Serialize, Deserialize, Debug, Clone, PartialEq, Eq)]
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
    // #[serde(other)]
    // other,
}

#[derive(Serialize, Deserialize, Debug, Clone, EnumFieldGetter)]
#[serde(untagged)]
pub enum Block {
    Normal {
        // potentially might not be top level
        #[serde(default)]
        x: i32,
        #[serde(default)]
        y: i32,
        #[serde(flatten)]
        block_info: BlockInfo,
    },
    Special(BlockArray),
}

#[derive(Serialize, Deserialize, Debug, Clone)]
#[serde(untagged)]
pub enum BlockArray {
    NumberOrAngle(u32, f64),
    ColorOrString(u32, String),
    Broadcast(u32, String, String), // might also be variable or list if not top level?
    VariableOrList(u32, String, String, f64, f64),
}

#[derive(Serialize, Deserialize, Debug, Clone)]
#[serde(untagged)]
pub enum BlockArrayOrId {
    Id(String),
    Array(BlockArray),
}

#[derive(Serialize, Deserialize, Debug, Clone, EnumFieldGetter)]
#[serde(untagged)]
pub enum Input {
    Shadow(u32, Option<BlockArrayOrId>, Option<BlockArrayOrId>),
    NoShadow(u32, Option<BlockArrayOrId>),
}

#[derive(Serialize, Deserialize, Debug, Clone, EnumFieldGetter)]
#[serde(untagged)]
pub enum Field {
    Value((Option<VarVal>,)),
    ValueId(Option<VarVal>, Option<String>),
}

#[derive(Serialize, Deserialize, Debug, Clone)]
#[serde(rename_all = "camelCase")]
pub struct Mutation {
    pub tag_name: String,
    #[serde(default)]
    pub children: Vec<()>,
    #[serde(flatten)]
    pub mutations: BTreeMap<String, Value>,
}

impl Default for Mutation {
    fn default() -> Self {
        Mutation {
            tag_name: String::from("mutation"),
            children: Default::default(),
            mutations: BTreeMap::new(),
        }
    }
}

#[derive(Serialize, Deserialize, Debug, Clone)]
#[serde(rename_all = "camelCase")]
pub struct BlockInfo {
    pub opcode: BlockOpcode,
    pub next: Option<String>,
    pub parent: Option<String>,
    pub inputs: BTreeMap<String, Input>,
    pub fields: BTreeMap<String, Field>,
    pub shadow: bool,
    pub top_level: bool,
    #[serde(default)]
    pub mutation: Mutation,
}

#[derive(Serialize, Deserialize, Debug, Clone, Copy, PartialEq)]
#[allow(non_camel_case_types)]
pub enum CostumeDataFormat {
    png,
    svg,
    jpeg,
    jpg,
    bmp,
    gif,
}

#[derive(Serialize, Deserialize, Debug, Clone)]
#[serde(rename_all = "camelCase")]
pub struct Costume {
    pub asset_id: String,
    pub name: String,
    pub md5ext: String,
    pub data_format: CostumeDataFormat,
    #[serde(default)]
    pub bitmap_resolution: f64,
    pub rotation_center_x: f64,
    pub rotation_center_y: f64,
}

#[derive(Serialize, Deserialize, Debug, Clone)]
#[serde(rename_all = "camelCase")]
pub struct Sound {
    pub asset_id: String,
    pub name: String,
    pub md5ext: String,
    pub data_format: String,
    pub rate: f64,
    pub sample_count: f64,
}

#[derive(Serialize, Deserialize, Debug, Clone, PartialEq)]
#[serde(untagged)]
pub enum VarVal {
    Float(f64),
    Bool(bool),
    String(String),
}

#[derive(Serialize, Deserialize, Debug, Clone, PartialEq)]
#[serde(untagged)]
pub enum VariableInfo {
    CloudVar(String, VarVal, bool),
    LocalVar(String, VarVal),
}

#[derive(Serialize, Deserialize, Debug, Clone)]
#[serde(rename_all = "camelCase")]
pub struct Target {
    pub is_stage: bool,
    pub name: String,
    pub variables: BTreeMap<String, VariableInfo>,
    pub lists: BTreeMap<String, (String, Vec<VarVal>)>,
    #[serde(default)]
    pub broadcasts: BTreeMap<String, String>,
    pub blocks: BTreeMap<String, Block>,
    pub comments: BTreeMap<String, Comment>,
    pub current_costume: u32,
    pub costumes: Vec<Costume>,
    pub sounds: Vec<Sound>,
    pub layer_order: i32,
    pub volume: f64,
    #[serde(default)]
    pub tempo: f64,
    #[serde(default)]
    pub video_state: Option<String>,
    #[serde(default)]
    pub video_transparency: f64,
    #[serde(default)]
    pub text_to_speech_language: Option<String>,
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
    pub rotation_style: String,
    #[serde(flatten)]
    pub unknown: BTreeMap<String, Value>,
}

#[derive(Serialize, Deserialize, Debug, Clone)]
#[serde(untagged)]
pub enum ListMonitorValue {
    List(Vec<VarVal>),
    String(String),
}

#[derive(Serialize, Deserialize, Debug, Clone, EnumFieldGetter)]
#[serde(untagged)]
pub enum Monitor {
    #[serde(rename_all = "camelCase")]
    ListMonitor {
        id: String,
        mode: String, // The name of the monitor's mode: "default", "large", "slider", or "list" - should be "list"
        opcode: String,
        params: BTreeMap<String, String>,
        sprite_name: Option<String>,
        width: f64,
        height: f64,
        x: f64,
        y: f64,
        visible: bool,
        value: ListMonitorValue,
    },
    #[serde(rename_all = "camelCase")]
    VarMonitor {
        id: String,
        mode: String, // The name of the monitor's mode: "default", "large", "slider", or "list".
        opcode: String,
        params: BTreeMap<String, String>,
        sprite_name: Option<String>,
        value: VarVal,
        width: f64,
        height: f64,
        x: f64,
        y: f64,
        visible: bool,
        slider_min: f64,
        slider_max: f64,
        is_discrete: bool,
    },
}

#[derive(Serialize, Deserialize, Debug, Clone)]
pub struct Meta {
    pub semver: String,
    pub vm: String,
    pub agent: String,
}

impl TryFrom<String> for Sb3Project {
    type Error = HQError;

    fn try_from(string: String) -> Result<Self, Self::Error> {
        (&string[..]).try_into()
    }
}

impl TryFrom<&str> for Sb3Project {
    type Error = HQError;

    fn try_from(string: &str) -> Result<Self, Self::Error> {
        use serde_json::error::Category;
        let sb3: Result<Self, serde_json::Error> = serde_json::from_str(string);
        match sb3 {
            Ok(proj) => Ok(proj),
            Err(err) => match err.classify() {
                Category::Syntax => hq_bad_proj!(
                    "Invalid JSON syntax at project.json:{}:{}",
                    err.line(),
                    err.column()
                ),
                Category::Data => hq_bad_proj!(
                    "Invalid project.json at project.json:{}:{}",
                    err.line(),
                    err.column()
                ),
                Category::Eof => hq_bad_proj!(
                    "Unexpected end of file at project.json:{}:{}",
                    err.line(),
                    err.column()
                ),
                _ => hq_bad_proj!("Failed to deserialize json"),
            },
        }
    }
}
/*
#[cfg(test)]
pub mod tests {
    use super::*;

    pub fn test_project_id(id: &str) -> String {
        use std::time::{SystemTime, UNIX_EPOCH};
        println!("https://api.scratch.mit.edu/projects/{:}/", id);
        let token_val = serde_json::from_str::<Value>(
            &reqwest::blocking::get(format!("https://api.scratch.mit.edu/projects/{:}/", id))
                .unwrap()
                .text()
                .unwrap(),
        )
        .unwrap()["project_token"]
            .clone();
        let token = token_val.as_str().unwrap();
        println!("{:}", token);
        println!(
            "https://projects.scratch.mit.edu/{:}/?token={:}&nocache={:}",
            id,
            token,
            SystemTime::now()
                .duration_since(UNIX_EPOCH)
                .unwrap_or_default()
                .as_millis()
        );

        //dbg!(&resp);
        reqwest::blocking::get(format!(
            "https://projects.scratch.mit.edu/{:}/?token={:}",
            id, token
        ))
        .unwrap()
        .text()
        .unwrap()
        //let j: Sb3Project = serde_json::from_str(&resp[..]).unwrap();
        //j
    }

    #[test]
    fn paper_minecraft() {
        let resp = self::test_project_id("10128407");
        let j: Sb3Project = resp.try_into().unwrap();
        dbg!(j);
        /*let k: Value = serde_json::from_str(&resp).unwrap();
        for (it, t) in j.targets.iter().enumerate() {
            for (i, b) in &t.blocks {
                if let Some(bi) = &b.block_info() {
                    if bi.opcode == super::BlockOpcode::other {
                        if let Value::Object(o) = &k {
                          if let Value::Array(a) = &o["targets"] {
                            if let Value::Object(o2) = &a[it] {
                              if let Value::Object(o3) = &o2["blocks"] {
                                if let Value::Object(o4) = &o3[i] {
                                  println!("{}", o4["opcode"]);
                                }
                              }
                            }
                          }
                        }
                    }
                }
            }
        }*/
    }

    #[test]
    fn level_eaten() {
        let resp = self::test_project_id("704676520");
        let j: Sb3Project = resp.try_into().unwrap();
        dbg!(j);
        /*let k: Value = serde_json::from_str(&resp).unwrap();
        for (it, t) in j.targets.iter().enumerate() {
            for (i, b) in &t.blocks {
                if let Some(bi) = &b.block_info() {
                    if bi.opcode == super::BlockOpcode::other {
                        if let Value::Object(o) = &k {
                          if let Value::Array(a) = &o["targets"] {
                            if let Value::Object(o2) = &a[it] {
                              if let Value::Object(o3) = &o2["blocks"] {
                                if let Value::Object(o4) = &o3[i] {
                                  println!("{}", o4["opcode"]);
                                }
                              }
                            }
                          }
                        }
                    }
                }
            }
        }*/
    }

    #[test]
    fn hq_test_project() {
        let resp = self::test_project_id("771449498");
        dbg!(&resp);
        let j: Sb3Project = resp.try_into().unwrap();
        dbg!(j);
    }

    #[test]
    fn default_project() {
        let resp = self::test_project_id("510186917");
        let j: Sb3Project = resp.try_into().unwrap();
        dbg!(j);
    }
}
*/
