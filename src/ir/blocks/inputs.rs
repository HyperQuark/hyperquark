use super::special::from_special_block;
use super::{NextBlocks, from_block};
use crate::instructions::IrOpcode;
use crate::ir::{IrProject, StepContext};
use crate::prelude::*;
use crate::sb3::{BlockArray, BlockArrayOrId, BlockInfo, BlockMap, BlockOpcode, Input};
use crate::wasm::WasmFlags;

fn input_names(block_info: &BlockInfo, context: &StepContext) -> HQResult<Vec<String>> {
    let opcode = &block_info.opcode;
    // target and procs need to be declared outside of the match block
    // to prevent lifetime issues
    let target = context.target();
    let procs = target.procedures()?;
    Ok(
        #[expect(
            clippy::wildcard_enum_match_arm,
            reason = "too many opcodes to match individually"
        )]
        match opcode {
            BlockOpcode::looks_say | BlockOpcode::looks_think => vec!["MESSAGE"],
            BlockOpcode::operator_add
            | BlockOpcode::operator_divide
            | BlockOpcode::operator_subtract
            | BlockOpcode::operator_multiply
            | BlockOpcode::operator_mod => vec!["NUM1", "NUM2"],
            BlockOpcode::operator_mathop | BlockOpcode::operator_round => vec!["NUM"],
            BlockOpcode::operator_lt
            | BlockOpcode::operator_gt
            | BlockOpcode::operator_equals
            | BlockOpcode::operator_and
            | BlockOpcode::operator_or => vec!["OPERAND1", "OPERAND2"],
            BlockOpcode::operator_join | BlockOpcode::operator_contains => {
                vec!["STRING1", "STRING2"]
            }
            BlockOpcode::operator_letter_of => vec!["LETTER", "STRING"],
            BlockOpcode::motion_gotoxy => vec!["X", "Y"],
            BlockOpcode::motion_movesteps => vec!["STEPS"],
            BlockOpcode::motion_pointindirection => vec!["DIRECTION"],
            BlockOpcode::motion_turnleft | BlockOpcode::motion_turnright => vec!["DEGREES"],
            BlockOpcode::sensing_keypressed => vec!["KEY_OPTION"],
            BlockOpcode::sensing_dayssince2000
            | BlockOpcode::data_variable
            | BlockOpcode::argument_reporter_boolean
            | BlockOpcode::argument_reporter_string_number
            | BlockOpcode::looks_costume
            | BlockOpcode::looks_size
            | BlockOpcode::looks_nextcostume
            | BlockOpcode::looks_costumenumbername
            | BlockOpcode::looks_backdrops
            | BlockOpcode::looks_hide
            | BlockOpcode::looks_show
            | BlockOpcode::pen_penDown
            | BlockOpcode::pen_penUp
            | BlockOpcode::pen_clear
            | BlockOpcode::control_forever
            | BlockOpcode::pen_menu_colorParam
            | BlockOpcode::motion_direction
            | BlockOpcode::data_deletealloflist
            | BlockOpcode::data_lengthoflist
            | BlockOpcode::data_listcontents
            | BlockOpcode::control_stop
            | BlockOpcode::event_broadcast_menu
            | BlockOpcode::sensing_timer
            | BlockOpcode::sensing_resettimer
            | BlockOpcode::sensing_answer
            | BlockOpcode::looks_backdropnumbername
            | BlockOpcode::looks_nextbackdrop
            | BlockOpcode::data_showvariable
            | BlockOpcode::data_hidevariable
            | BlockOpcode::sensing_mousex
            | BlockOpcode::sensing_mousey
            | BlockOpcode::sensing_mousedown
            | BlockOpcode::motion_xposition
            | BlockOpcode::motion_yposition
            | BlockOpcode::sensing_keyoptions => vec![],
            BlockOpcode::sensing_askandwait => vec!["QUESTION"],
            BlockOpcode::event_broadcast | BlockOpcode::event_broadcastandwait => {
                vec!["BROADCAST_INPUT"]
            }
            BlockOpcode::control_wait => vec!["DURATION"],
            BlockOpcode::data_setvariableto
            | BlockOpcode::data_changevariableby
            | BlockOpcode::control_for_each => vec!["VALUE"],
            BlockOpcode::operator_random => vec!["FROM", "TO"],
            BlockOpcode::pen_setPenColorParamTo | BlockOpcode::pen_changePenColorParamBy => {
                vec!["COLOR_PARAM", "VALUE"]
            }
            BlockOpcode::control_if
            | BlockOpcode::control_if_else
            | BlockOpcode::control_repeat_until
            | BlockOpcode::control_while
            | BlockOpcode::control_wait_until => vec!["CONDITION"],
            BlockOpcode::operator_not => vec!["OPERAND"],
            BlockOpcode::control_repeat => vec!["TIMES"],
            BlockOpcode::operator_length => vec!["STRING"],
            BlockOpcode::looks_switchcostumeto => vec!["COSTUME"],
            BlockOpcode::looks_switchbackdropto => vec!["BACKDROP"],
            BlockOpcode::looks_setsizeto | BlockOpcode::pen_setPenSizeTo => vec!["SIZE"],
            BlockOpcode::looks_changesizeby => vec!["CHANGE"],
            BlockOpcode::pen_setPenColorToColor => vec!["COLOR"],
            BlockOpcode::data_addtolist
            | BlockOpcode::data_itemnumoflist
            | BlockOpcode::data_listcontainsitem => vec!["ITEM"],
            BlockOpcode::data_itemoflist | BlockOpcode::data_deleteoflist => vec!["INDEX"],
            BlockOpcode::data_replaceitemoflist | BlockOpcode::data_insertatlist => {
                vec!["INDEX", "ITEM"]
            }
            BlockOpcode::motion_changexby => vec!["DX"],
            BlockOpcode::motion_changeyby => vec!["DY"],
            BlockOpcode::motion_setx => vec!["X"],
            BlockOpcode::motion_sety => vec!["Y"],
            BlockOpcode::procedures_call => 'proc_block: {
                let serde_json::Value::String(proccode) = block_info
                    .mutation
                    .mutations
                    .get("proccode")
                    .ok_or_else(|| make_hq_bad_proj!("missing proccode on procedures_call"))?
                else {
                    hq_bad_proj!("non-string proccode on procedures_call");
                };
                let Some(proc) = procs.get(proccode.as_str()) else {
                    break 'proc_block vec![];
                };
                proc.arg_ids().iter().map(|b| &**b).collect()
            }
            other => hq_todo!("unimplemented input_names for {:?}", other),
        }
        .into_iter()
        .map(String::from)
        .collect(),
    )
}

pub fn inputs(
    block_info: &BlockInfo,
    blocks: &BlockMap,
    context: &StepContext,
    project: &Weak<IrProject>,
    flags: &WasmFlags,
) -> HQResult<Vec<IrOpcode>> {
    Ok(input_names(block_info, context)?
        .into_iter()
        .map(|name| -> HQResult<Vec<IrOpcode>> {
            let input = match block_info.inputs.get((*name).into()) {
                Some(noshadow @ Input::NoShadow(_, Some(_))) => noshadow,
                Some(shadow @ Input::Shadow(_, Some(_), _)) => shadow,
                None | Some(Input::NoShadow(_, None) | Input::Shadow(_, None, _)) => {
                    // revert to a sensible default
                    &Input::NoShadow(
                        0,
                        Some(BlockArrayOrId::Array(BlockArray::NumberOrAngle(6, 0.0))),
                    )
                }
            };
            #[expect(
                clippy::wildcard_enum_match_arm,
                reason = "all variants covered in previous match guards"
            )]
            match input {
                Input::NoShadow(_, Some(block)) | Input::Shadow(_, Some(block), _) => match block {
                    BlockArrayOrId::Array(arr) => {
                        Ok(vec![from_special_block(arr, context, flags)?])
                    }
                    BlockArrayOrId::Id(id) => from_block(
                        blocks.get(id).ok_or_else(|| {
                            make_hq_bad_proj!("block for input {} doesn't exist", name)
                        })?,
                        blocks,
                        context,
                        project,
                        NextBlocks::new(false),
                        flags,
                    ),
                },
                _ => hq_bad_proj!("missing input block for {}", name),
            }
        })
        .collect::<HQResult<Vec<_>>>()?
        .iter()
        .flatten()
        .cloned()
        .collect())
}
