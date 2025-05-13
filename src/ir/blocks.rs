use super::context::StepContext;
use super::target::Target;
use super::{IrProject, RcVar, Step, Type as IrType};
use crate::instructions::{
    fields::{
        ControlIfElseFields, ControlLoopFields, DataSetvariabletoFields, DataTeevariableFields,
        DataVariableFields, HqCastFields, HqFloatFields, HqIntegerFields, HqTextFields,
        HqYieldFields, LooksSayFields, LooksThinkFields, ProceduresArgumentFields,
        ProceduresCallWarpFields,
    },
    IrOpcode, YieldMode,
};
use crate::ir::Variable;
use crate::prelude::*;
use crate::sb3;
use sb3::{Block, BlockArray, BlockArrayOrId, BlockInfo, BlockMap, BlockOpcode, Input};

fn insert_casts(mut blocks: Vec<IrOpcode>) -> HQResult<Vec<IrOpcode>> {
    let mut type_stack: Vec<(IrType, usize)> = vec![]; // a vector of types, and where they came from
    let mut casts: Vec<(usize, IrType)> = vec![]; // a vector of cast targets, and where they're needed
    for (i, block) in blocks.iter().enumerate() {
        let expected_inputs = block.acceptable_inputs();
        if type_stack.len() < expected_inputs.len() {
            hq_bug!("didn't have enough inputs on the type stack")
        }
        let actual_inputs: Vec<_> = type_stack
            .splice((type_stack.len() - expected_inputs.len()).., [])
            .collect();
        for (&expected, actual) in core::iter::zip(expected_inputs.iter(), actual_inputs) {
            if !expected
                .base_types()
                .any(|ty1| actual.0.base_types().any(|ty2| ty2 == ty1))
            {
                casts.push((actual.1, expected));
            }
        }
        // TODO: make this more specific by using the actual input types post-cast
        if let Some(output) = block.output_type(expected_inputs)? {
            type_stack.push((output, i));
        }
    }
    for (pos, ty) in casts.into_iter().rev() {
        blocks.insert(pos + 1, IrOpcode::hq_cast(HqCastFields(ty)));
    }
    Ok(blocks)
}

#[derive(Clone, Debug)]
pub enum NextBlock {
    ID(Box<str>),
    Step(Weak<Step>),
}

#[derive(Clone, Debug)]
pub struct NextBlockInfo {
    pub yield_first: bool,
    pub block: NextBlock,
}

/// contains a vector of next blocks, as well as information on how to proceed when
/// there are no next blocks: true => terminate the thread, false => do nothing
/// (useful for e.g. loop bodies, or for non-stack blocks)
#[derive(Clone, Debug)]
pub struct NextBlocks(Vec<NextBlockInfo>, bool);

impl NextBlocks {
    pub const fn new(terminating: bool) -> Self {
        Self(vec![], terminating)
    }

    pub const fn terminating(&self) -> bool {
        self.1
    }

    pub fn extend_with_inner(&self, new: NextBlockInfo) -> Self {
        let mut cloned = self.0.clone();
        cloned.push(new);
        Self(cloned, self.terminating())
    }

    pub fn pop_inner(self) -> (Option<NextBlockInfo>, Self) {
        let terminating = self.terminating();
        let mut vec = self.0;
        let popped = vec.pop();
        (popped, Self(vec, terminating))
    }
}

pub fn from_block(
    block: &Block,
    blocks: &BlockMap,
    context: &StepContext,
    project: &Weak<IrProject>,
    final_next_blocks: NextBlocks,
) -> HQResult<Vec<IrOpcode>> {
    insert_casts(match block {
        Block::Normal { block_info, .. } => {
            from_normal_block(block_info, blocks, context, project, final_next_blocks)?.to_vec()
        }
        Block::Special(block_array) => vec![from_special_block(block_array, context)?],
    })
}

pub fn input_names(block_info: &BlockInfo, context: &StepContext) -> HQResult<Vec<String>> {
    let opcode = &block_info.opcode;
    // target and procs need to be declared outside of the match block
    // to prevent lifetime issues
    let target = context
        .target
        .upgrade()
        .ok_or_else(|| make_hq_bug!("couldn't upgrade Weak<Target>"))?;
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
            | BlockOpcode::operator_multiply => vec!["NUM1", "NUM2"],
            BlockOpcode::operator_lt
            | BlockOpcode::operator_gt
            | BlockOpcode::operator_equals
            | BlockOpcode::operator_and
            | BlockOpcode::operator_or => vec!["OPERAND1", "OPERAND2"],
            BlockOpcode::operator_join | BlockOpcode::operator_contains => {
                vec!["STRING1", "STRING2"]
            }
            BlockOpcode::operator_letter_of => vec!["LETTER", "STRING"],
            BlockOpcode::sensing_dayssince2000
            | BlockOpcode::data_variable
            | BlockOpcode::argument_reporter_boolean
            | BlockOpcode::argument_reporter_string_number => vec![],
            BlockOpcode::data_setvariableto | BlockOpcode::data_changevariableby => vec!["VALUE"],
            BlockOpcode::control_if
            | BlockOpcode::control_if_else
            | BlockOpcode::control_repeat_until => vec!["CONDITION"],
            BlockOpcode::operator_not => vec!["OPERAND"],
            BlockOpcode::control_repeat => vec!["TIMES"],
            BlockOpcode::operator_length => vec!["STRING"],
            BlockOpcode::procedures_call => {
                let serde_json::Value::String(proccode) = block_info
                    .mutation
                    .mutations
                    .get("proccode")
                    .ok_or_else(|| make_hq_bad_proj!("missing proccode on procedures_call"))?
                else {
                    hq_bad_proj!("non-string proccode on procedures_call");
                };
                let Some(proc) = procs.get(proccode.as_str()) else {
                    hq_bad_proj!("procedures_call proccode doesn't exist")
                };
                proc.context().arg_ids().iter().map(|b| &**b).collect()
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
) -> HQResult<Vec<IrOpcode>> {
    Ok(input_names(block_info, context)?
        .into_iter()
        .map(|name| -> HQResult<Vec<IrOpcode>> {
            let input = match block_info.inputs.get((*name).into()) {
                Some(input) => input,
                None => {
                    if name.starts_with("CONDITION") {
                        &Input::NoShadow(
                            0,
                            Some(BlockArrayOrId::Array(BlockArray::NumberOrAngle(6, 0.0))),
                        )
                    } else {
                        hq_bad_proj!("missing input {}", name)
                    }
                }
            };
            #[expect(
                clippy::wildcard_enum_match_arm,
                reason = "all variants covered in previous match guards"
            )]
            match input {
                Input::NoShadow(_, Some(block)) | Input::Shadow(_, Some(block), _) => match block {
                    BlockArrayOrId::Array(arr) => Ok(vec![from_special_block(arr, context)?]),
                    BlockArrayOrId::Id(id) => from_block(
                        blocks.get(id).ok_or_else(|| {
                            make_hq_bad_proj!("block for input {} doesn't exist", name)
                        })?,
                        blocks,
                        context,
                        project,
                        NextBlocks::new(false),
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

#[derive(Copy, Clone, Debug, PartialEq, Eq)]
pub enum ProcArgType {
    Boolean,
    StringNumber,
}

impl ProcArgType {
    fn default_block(self) -> Vec<IrOpcode> {
        vec![match self {
            Self::Boolean => IrOpcode::hq_integer(HqIntegerFields(0)),
            Self::StringNumber => IrOpcode::hq_text(HqTextFields("".into())),
        }]
    }
}

fn procedure_argument(
    arg_type: ProcArgType,
    block_info: &BlockInfo,
    context: &StepContext,
) -> HQResult<Vec<IrOpcode>> {
    let Some(proc_context) = context.proc_context.clone() else {
        return Ok(arg_type.default_block());
    };
    let sb3::VarVal::String(arg_name) = block_info
        .fields
        .get("VALUE")
        .ok_or_else(|| make_hq_bad_proj!("missing VALUE field for proc argument"))?
        .get_0()
        .ok_or_else(|| make_hq_bad_proj!("missing value of VALUE field"))?
    else {
        hq_bad_proj!("non-string proc argument name")
    };
    let Some(index) = proc_context
        .arg_names()
        .iter()
        .position(|name| name == arg_name)
    else {
        return Ok(arg_type.default_block());
    };
    let expected_type = proc_context
        .arg_types()
        .get(index)
        .ok_or_else(|| make_hq_bad_proj!("argument index not in range of argumenttypes"))?;
    hq_assert!(
        (arg_type == ProcArgType::Boolean && *expected_type == IrType::Boolean)
            || (arg_type == ProcArgType::StringNumber
                && (*expected_type == IrType::String || *expected_type == IrType::Number)),
        "argument block doesn't match actual argument type"
    );
    Ok(vec![IrOpcode::procedures_argument(
        ProceduresArgumentFields(index, *expected_type),
    )])
}

#[expect(clippy::too_many_arguments, reason = "too many arguments!")]
// TODO: put these arguments into a struct?
fn generate_loop(
    warp: bool,
    should_break: &mut bool,
    block_info: &BlockInfo,
    blocks: &BTreeMap<Box<str>, Block>,
    context: &StepContext,
    final_next_blocks: NextBlocks,
    first_condition_instructions: Option<Vec<IrOpcode>>,
    condition_instructions: Vec<IrOpcode>,
    flip_if: bool,
    setup_instructions: Vec<IrOpcode>,
) -> HQResult<Vec<IrOpcode>> {
    let BlockArrayOrId::Id(substack_id) = match block_info.inputs.get("SUBSTACK") {
        Some(input) => input,
        None => return Ok(vec![IrOpcode::hq_drop]), // TODO: consider loops without input (i.e. forever)
    }
    .get_1()
    .ok_or_else(|| make_hq_bug!(""))?
    .clone()
    .ok_or_else(|| make_hq_bug!(""))?
    else {
        hq_bad_proj!("malformed SUBSTACK input")
    };
    let Some(substack_block) = blocks.get(&substack_id) else {
        hq_bad_proj!("SUBSTACK block doesn't seem to exist")
    };
    if warp {
        // TODO: can this be expressed in the same way as non-warping loops,
        // just with yield_first: false?
        let substack_blocks = from_block(
            substack_block,
            blocks,
            context,
            &context.target()?.project(),
            NextBlocks::new(false),
        )?;
        let substack_step = Step::new_rc(
            None,
            context.clone(),
            substack_blocks,
            &context.target()?.project(),
        )?;
        let condition_step = Step::new_rc(
            None,
            context.clone(),
            condition_instructions,
            &context.target()?.project(),
        )?;
        let first_condition_step = if let Some(instrs) = first_condition_instructions {
            Some(Step::new_rc(
                None,
                context.clone(),
                instrs,
                &context.target()?.project(),
            )?)
        } else {
            None
        };
        Ok(setup_instructions
            .into_iter()
            .chain(vec![IrOpcode::control_loop(ControlLoopFields {
                first_condition: first_condition_step,
                condition: condition_step,
                body: substack_step,
                flip_if,
            })])
            .collect())
    } else {
        *should_break = true;
        let (next_block, outer_next_blocks) = if let Some(ref next_block) = block_info.next {
            (Some(NextBlock::ID(next_block.clone())), final_next_blocks)
        } else if let (Some(next_block_info), popped_next_blocks) =
            final_next_blocks.clone().pop_inner()
        {
            (Some(next_block_info.block), popped_next_blocks)
        } else {
            (None, final_next_blocks)
        };
        let next_step = match next_block {
            Some(NextBlock::ID(id)) => {
                let Some(next_block_block) = blocks.get(&id) else {
                    hq_bad_proj!("next block doesn't exist")
                };
                Step::from_block(
                    next_block_block,
                    id,
                    blocks,
                    context,
                    &context.target()?.project(),
                    outer_next_blocks,
                )?
            }
            Some(NextBlock::Step(ref step)) => step
                .upgrade()
                .ok_or_else(|| make_hq_bug!("couldn't upgrade Weak<Step>"))?,
            None => Step::new_terminating(context.clone(), &context.target()?.project())?,
        };
        let condition_step = Step::new_rc(
            None,
            context.clone(),
            condition_instructions.clone(),
            &context.target()?.project(),
        )?;
        let substack_blocks = from_block(
            substack_block,
            blocks,
            context,
            &context.target()?.project(),
            NextBlocks::new(false).extend_with_inner(NextBlockInfo {
                yield_first: true,
                block: NextBlock::Step(Rc::downgrade(&condition_step)),
            }),
        )?;
        let substack_step = Step::new_rc(
            None,
            context.clone(),
            substack_blocks,
            &context.target()?.project(),
        )?;
        condition_step
            .opcodes_mut()?
            .push(IrOpcode::control_if_else(ControlIfElseFields {
                branch_if: Rc::clone(if flip_if { &next_step } else { &substack_step }),
                branch_else: Rc::clone(if flip_if { &substack_step } else { &next_step }),
            }));
        Ok(setup_instructions
            .into_iter()
            .chain(first_condition_instructions.map_or(condition_instructions, |instrs| instrs))
            .chain(vec![IrOpcode::control_if_else(ControlIfElseFields {
                branch_if: if flip_if {
                    Rc::clone(&next_step)
                } else {
                    Rc::clone(&substack_step)
                },
                branch_else: if flip_if { substack_step } else { next_step },
            })])
            .collect())
    }
}

#[expect(
    clippy::too_many_lines,
    reason = "difficult to split up; will probably be condensed in future"
)]
fn generate_if_else(
    if_block: (&Block, Box<str>),
    maybe_else_block: Option<(&Block, Box<str>)>,
    block_info: &BlockInfo,
    final_next_blocks: &NextBlocks,
    blocks: &BTreeMap<Box<str>, Block>,
    context: &StepContext,
    should_break: &mut bool,
) -> HQResult<Vec<IrOpcode>> {
    crate::log(
        format!(
            "generate_if_else (if block: {}) (else block?: {})",
            if_block.1,
            maybe_else_block.is_some()
        )
        .as_str(),
    );
    let this_project = context.project()?;
    let dummy_project = Rc::new(IrProject::new(this_project.global_variables().clone()));
    let dummy_target = Rc::new(Target::new(
        false,
        context.target()?.variables().clone(),
        Rc::downgrade(&dummy_project),
        RefCell::new(context.target()?.procedures()?.clone()),
        0,
    ));
    dummy_project
        .targets()
        .try_borrow_mut()
        .map_err(|_| make_hq_bug!("couldn't mutably borrow cell"))?
        .insert("".into(), Rc::clone(&dummy_target));
    let dummy_context = StepContext {
        target: Rc::downgrade(&dummy_target),
        ..context.clone()
    };
    let dummy_if_step = Step::from_block(
        if_block.0,
        if_block.1.clone(),
        blocks,
        &dummy_context,
        &Rc::downgrade(&dummy_project),
        NextBlocks::new(false),
    )?;
    let dummy_else_step = if let Some((else_block, else_block_id)) = maybe_else_block.clone() {
        Step::from_block(
            else_block,
            else_block_id,
            blocks,
            &dummy_context,
            &Rc::downgrade(&dummy_project),
            NextBlocks::new(false),
        )?
    } else {
        Rc::new(Step::new_empty())
    };
    let if_step_yields = dummy_if_step.does_yield()?;
    let else_step_yields = dummy_else_step.does_yield()?;
    crate::log(format!("if yields: {if_step_yields}, else yields: {else_step_yields}").as_str());
    if !context.warp && (dummy_if_step.does_yield()? || dummy_else_step.does_yield()?) {
        // TODO: ideally if only one branch yields then we'd duplicate the next step and put one
        // version inline after the branch, and the other tagges on in the substep's NextBlocks
        // as usual, to allow for extra variable type optimisations.
        #[expect(
            clippy::option_if_let_else,
            reason = "map_or_else alternative is too complex"
        )]
        let (next_block, next_blocks) = if let Some(ref next_block) = block_info.next {
            (
                Some(NextBlock::ID(next_block.clone())),
                final_next_blocks.extend_with_inner(NextBlockInfo {
                    yield_first: false,
                    block: NextBlock::ID(next_block.clone()),
                }),
            )
        } else if let (Some(next_block_info), _) = final_next_blocks.clone().pop_inner() {
            (Some(next_block_info.block), final_next_blocks.clone())
        } else {
            (
                None,
                final_next_blocks.clone(), // preserve termination behaviour
            )
        };
        let final_if_step = {
            let s = Step::from_block(
                if_block.0,
                if_block.1,
                blocks,
                context,
                &context.target()?.project(),
                next_blocks.clone(),
            )?;
            crate::log("recompiled if_step with correct next blocks");
            s
        };
        let final_else_step = if let Some((else_block, else_block_id)) = maybe_else_block {
            let s = Step::from_block(
                else_block,
                else_block_id,
                blocks,
                context,
                &context.target()?.project(),
                next_blocks,
            )?;
            crate::log("recompiled else step with correct next blocks");
            s
        } else {
            let opcode = match next_block {
                Some(NextBlock::ID(id)) => {
                    let next_block = blocks
                        .get(&id)
                        .ok_or_else(|| make_hq_bad_proj!("missing next block"))?;
                    crate::log(format!("got NextBlock::Id({id:?})").as_str());
                    vec![IrOpcode::hq_yield(HqYieldFields {
                        mode: YieldMode::Inline(Step::from_block(
                            next_block,
                            id.clone(),
                            blocks,
                            context,
                            &context.target()?.project(),
                            next_blocks,
                        )?),
                    })]
                }
                Some(NextBlock::Step(step)) => {
                    let rcstep = step
                        .upgrade()
                        .ok_or_else(|| make_hq_bug!("couldn't upgrade Weak<Step>"))?;
                    crate::log(format!("got NextBlock::Step({:?})", rcstep.id()).as_str());
                    vec![IrOpcode::hq_yield(HqYieldFields {
                        mode: YieldMode::Inline(rcstep),
                    })]
                }
                None => {
                    crate::log("no next block after if!");
                    if next_blocks.terminating() {
                        crate::log("terminating after if/else");
                        vec![IrOpcode::hq_yield(HqYieldFields {
                            mode: YieldMode::None,
                        })]
                    } else {
                        crate::log("not terminating, at end of if/else");
                        vec![]
                    }
                }
            };
            Rc::new(Step::new(
                None,
                context.clone(),
                opcode,
                context.target()?.project(),
            ))
        };
        *should_break = true;
        Ok(vec![IrOpcode::control_if_else(ControlIfElseFields {
            branch_if: final_if_step,
            branch_else: final_else_step,
        })])
    } else {
        let final_if_step = Step::from_block(
            if_block.0,
            if_block.1,
            blocks,
            context,
            &context.target()?.project(),
            NextBlocks::new(false),
        )?;
        let final_else_step = if let Some((else_block, else_block_id)) = maybe_else_block {
            Step::from_block(
                else_block,
                else_block_id,
                blocks,
                context,
                &context.target()?.project(),
                NextBlocks::new(false),
            )?
        } else {
            Step::new_rc(None, context.clone(), vec![], &context.target()?.project())?
        };
        Ok(vec![IrOpcode::control_if_else(ControlIfElseFields {
            branch_if: final_if_step,
            branch_else: final_else_step,
        })])
    }
}

#[expect(
    clippy::too_many_lines,
    reason = "a big monolithic function is somewhat unavoidable here"
)]
fn from_normal_block(
    block_info: &BlockInfo,
    blocks: &BlockMap,
    context: &StepContext,
    project: &Weak<IrProject>,
    final_next_blocks: NextBlocks,
) -> HQResult<Box<[IrOpcode]>> {
    let mut curr_block = Some(block_info);
    let mut final_next_blocks = final_next_blocks;
    let mut opcodes = vec![];
    let mut should_break = false;
    while let Some(block_info) = curr_block {
        opcodes.append(
            &mut inputs(block_info, blocks, context, project)?
                .into_iter()
                .chain(
                    #[expect(
                        clippy::wildcard_enum_match_arm,
                        reason = "too many opcodes to match individually"
                    )]
                    match &block_info.opcode {
                        BlockOpcode::operator_add => vec![IrOpcode::operator_add],
                        BlockOpcode::operator_subtract => vec![IrOpcode::operator_subtract],
                        BlockOpcode::operator_multiply => vec![IrOpcode::operator_multiply],
                        BlockOpcode::operator_divide => vec![IrOpcode::operator_divide],
                        BlockOpcode::looks_say => vec![IrOpcode::looks_say(LooksSayFields {
                            debug: context.debug,
                            target_idx: context
                                .target
                                .upgrade()
                                .ok_or_else(|| make_hq_bug!("couldn't upgrade Weak<Target>"))?
                                .index(),
                        })],
                        BlockOpcode::looks_think => vec![IrOpcode::looks_think(LooksThinkFields {
                            debug: context.debug,
                            target_idx: context
                                .target
                                .upgrade()
                                .ok_or_else(|| make_hq_bug!("couldn't upgrade Weak<Target>"))?
                                .index(),
                        })],
                        BlockOpcode::operator_join => vec![IrOpcode::operator_join],
                        BlockOpcode::operator_length => vec![IrOpcode::operator_length],
                        BlockOpcode::operator_contains => vec![IrOpcode::operator_contains],
                        BlockOpcode::operator_letter_of => vec![IrOpcode::operator_letter_of],
                        BlockOpcode::sensing_dayssince2000 => vec![IrOpcode::sensing_dayssince2000],
                        BlockOpcode::operator_lt => vec![IrOpcode::operator_lt],
                        BlockOpcode::operator_gt => vec![IrOpcode::operator_gt],
                        BlockOpcode::operator_equals => vec![IrOpcode::operator_equals],
                        BlockOpcode::operator_not => vec![IrOpcode::operator_not],
                        BlockOpcode::data_setvariableto => {
                            let sb3::Field::ValueId(_val, maybe_id) =
                                block_info.fields.get("VARIABLE").ok_or_else(|| {
                                    make_hq_bad_proj!(
                                        "invalid project.json - missing field VARIABLE"
                                    )
                                })?
                            else {
                                hq_bad_proj!(
                                    "invalid project.json - missing variable id for VARIABLE field"
                                );
                            };
                            let id = maybe_id.clone().ok_or_else(|| {
                                make_hq_bad_proj!(
                                    "invalid project.json - null variable id for VARIABLE field"
                                )
                            })?;
                            let target = context
                                .target
                                .upgrade()
                                .ok_or_else(|| make_hq_bug!("couldn't upgrade Weak"))?;
                            let variable = if let Some(var) = target.variables().get(&id) {
                                var
                            } else if let Some(var) = context
                                .target()?
                                .project()
                                .upgrade()
                                .ok_or_else(|| make_hq_bug!("couldn't upgrade Weak<Project>"))?
                                .global_variables()
                                .get(&id)
                            {
                                &Rc::clone(var)
                            } else {
                                hq_bad_proj!("variable not found")
                            };
                            vec![IrOpcode::data_setvariableto(DataSetvariabletoFields(
                                RcVar(Rc::clone(variable)),
                            ))]
                        }
                        BlockOpcode::data_changevariableby => {
                            let sb3::Field::ValueId(_val, maybe_id) =
                                block_info.fields.get("VARIABLE").ok_or_else(|| {
                                    make_hq_bad_proj!(
                                        "invalid project.json - missing field VARIABLE"
                                    )
                                })?
                            else {
                                hq_bad_proj!(
                                    "invalid project.json - missing variable id for VARIABLE field"
                                );
                            };
                            let id = maybe_id.clone().ok_or_else(|| {
                                make_hq_bad_proj!(
                                    "invalid project.json - null variable id for VARIABLE field"
                                )
                            })?;
                            let target = context
                                .target
                                .upgrade()
                                .ok_or_else(|| make_hq_bug!("couldn't upgrade Weak"))?;
                            let variable = if let Some(var) = target.variables().get(&id) {
                                var
                            } else if let Some(var) = context
                                .target()?
                                .project()
                                .upgrade()
                                .ok_or_else(|| make_hq_bug!("couldn't upgrade Weak<Project>"))?
                                .global_variables()
                                .get(&id)
                            {
                                &Rc::clone(var)
                            } else {
                                hq_bad_proj!("variable not found")
                            };
                            vec![
                                IrOpcode::data_variable(DataVariableFields(RcVar(Rc::clone(
                                    variable,
                                )))),
                                IrOpcode::operator_add,
                                IrOpcode::data_setvariableto(DataSetvariabletoFields(RcVar(
                                    Rc::clone(variable),
                                ))),
                            ]
                        }
                        BlockOpcode::data_variable => {
                            let sb3::Field::ValueId(_val, maybe_id) =
                                block_info.fields.get("VARIABLE").ok_or_else(|| {
                                    make_hq_bad_proj!(
                                        "invalid project.json - missing field VARIABLE"
                                    )
                                })?
                            else {
                                hq_bad_proj!(
                                    "invalid project.json - missing variable id for VARIABLE field"
                                );
                            };
                            let id = maybe_id.clone().ok_or_else(|| {
                                make_hq_bad_proj!(
                                    "invalid project.json - null variable id for VARIABLE field"
                                )
                            })?;
                            let target = context
                                .target
                                .upgrade()
                                .ok_or_else(|| make_hq_bug!("couldn't upgrade Weak"))?;
                            let variable = if let Some(var) = target.variables().get(&id) {
                                var
                            } else if let Some(var) = context
                                .target()?
                                .project()
                                .upgrade()
                                .ok_or_else(|| make_hq_bug!("couldn't upgrade Weak<Project>"))?
                                .global_variables()
                                .get(&id)
                            {
                                &Rc::clone(var)
                            } else {
                                hq_bad_proj!("variable not found")
                            };
                            vec![IrOpcode::data_variable(DataVariableFields(RcVar(
                                Rc::clone(variable),
                            )))]
                        }
                        BlockOpcode::control_if => 'block: {
                            let BlockArrayOrId::Id(substack_id) =
                                match block_info.inputs.get("SUBSTACK") {
                                    Some(input) => input,
                                    None => break 'block vec![IrOpcode::hq_drop],
                                }
                                .get_1()
                                .ok_or_else(|| make_hq_bug!(""))?
                                .clone()
                                .ok_or_else(|| make_hq_bug!(""))?
                            else {
                                hq_bad_proj!("malformed SUBSTACK input")
                            };
                            let Some(substack_block) = blocks.get(&substack_id) else {
                                hq_bad_proj!("SUBSTACK block doesn't seem to exist")
                            };
                            generate_if_else(
                                (substack_block, substack_id),
                                None,
                                block_info,
                                &final_next_blocks,
                                blocks,
                                context,
                                &mut should_break,
                            )?
                        }
                        BlockOpcode::control_if_else => 'block: {
                            let BlockArrayOrId::Id(substack1_id) =
                                match block_info.inputs.get("SUBSTACK") {
                                    Some(input) => input,
                                    None => break 'block vec![IrOpcode::hq_drop],
                                }
                                .get_1()
                                .ok_or_else(|| make_hq_bug!(""))?
                                .clone()
                                .ok_or_else(|| make_hq_bug!(""))?
                            else {
                                hq_bad_proj!("malformed SUBSTACK input")
                            };
                            let Some(substack1_block) = blocks.get(&substack1_id) else {
                                hq_bad_proj!("SUBSTACK block doesn't seem to exist")
                            };
                            let BlockArrayOrId::Id(substack2_id) =
                                match block_info.inputs.get("SUBSTACK2") {
                                    Some(input) => input,
                                    None => break 'block vec![IrOpcode::hq_drop],
                                }
                                .get_1()
                                .ok_or_else(|| make_hq_bug!(""))?
                                .clone()
                                .ok_or_else(|| make_hq_bug!(""))?
                            else {
                                hq_bad_proj!("malformed SUBSTACK2 input")
                            };
                            let Some(substack2_block) = blocks.get(&substack2_id) else {
                                hq_bad_proj!("SUBSTACK2 block doesn't seem to exist")
                            };
                            generate_if_else(
                                (substack1_block, substack1_id),
                                Some((substack2_block, substack2_id)),
                                block_info,
                                &final_next_blocks,
                                blocks,
                                context,
                                &mut should_break,
                            )?
                        }
                        BlockOpcode::control_repeat => {
                            let variable = RcVar(Rc::new(Variable::new(
                                IrType::Int,
                                sb3::VarVal::Float(0.0),
                                context.warp,
                            )));
                            let condition_instructions = vec![
                                IrOpcode::data_variable(DataVariableFields(variable.clone())),
                                IrOpcode::hq_integer(HqIntegerFields(1)),
                                IrOpcode::operator_subtract,
                                IrOpcode::data_teevariable(DataTeevariableFields(variable.clone())),
                                IrOpcode::hq_integer(HqIntegerFields(0)),
                                IrOpcode::operator_gt,
                            ];
                            let first_condition_instructions = Some(vec![
                                IrOpcode::data_variable(DataVariableFields(variable.clone())),
                                IrOpcode::hq_integer(HqIntegerFields(0)),
                                IrOpcode::operator_gt,
                            ]);
                            let setup_instructions = vec![
                                IrOpcode::hq_cast(HqCastFields(IrType::Int)),
                                IrOpcode::data_setvariableto(DataSetvariabletoFields(variable)),
                            ];
                            generate_loop(
                                context.warp,
                                &mut should_break,
                                block_info,
                                blocks,
                                context,
                                final_next_blocks.clone(),
                                first_condition_instructions,
                                condition_instructions,
                                false,
                                setup_instructions,
                            )?
                        }
                        BlockOpcode::control_repeat_until => {
                            let condition_instructions =
                                inputs(block_info, blocks, context, &context.target()?.project())?;
                            let first_condition_instructions = None;
                            let setup_instructions = vec![IrOpcode::hq_drop];
                            generate_loop(
                                context.warp,
                                &mut should_break,
                                block_info,
                                blocks,
                                context,
                                final_next_blocks.clone(),
                                first_condition_instructions,
                                condition_instructions,
                                true,
                                setup_instructions,
                            )?
                        }
                        BlockOpcode::procedures_call => {
                            let target = context
                                .target
                                .upgrade()
                                .ok_or_else(|| make_hq_bug!("couldn't upgrade Weak<Target>"))?;
                            let procs = target.procedures()?;
                            let serde_json::Value::String(proccode) = block_info
                                .mutation
                                .mutations
                                .get("proccode")
                                .ok_or_else(|| {
                                    make_hq_bad_proj!("missing proccode on procedures_call")
                                })?
                            else {
                                hq_bad_proj!("non-string proccode on procedures_call")
                            };
                            let proc = procs.get(proccode.as_str()).ok_or_else(|| {
                                make_hq_bad_proj!("non-existant proccode on procedures_call")
                            })?;
                            let warp = context.warp || proc.always_warped();
                            if warp {
                                proc.compile_warped(blocks)?;
                                vec![IrOpcode::procedures_call_warp(ProceduresCallWarpFields {
                                    proc: Rc::clone(proc),
                                })]
                            } else {
                                hq_todo!("non-warped procedures")
                            }
                        }
                        BlockOpcode::argument_reporter_boolean => {
                            procedure_argument(ProcArgType::Boolean, block_info, context)?
                        }
                        BlockOpcode::argument_reporter_string_number => {
                            procedure_argument(ProcArgType::StringNumber, block_info, context)?
                        }
                        other => hq_todo!("unimplemented block: {:?}", other),
                    },
                )
                .collect(),
        );
        if should_break {
            break;
        }
        curr_block = if let Some(ref next_id) = block_info.next {
            let next_block = blocks
                .get(next_id)
                .ok_or_else(|| make_hq_bad_proj!("missing next block"))?;
            if opcodes
                .last()
                .is_some_and(super::super::instructions::IrOpcode::requests_screen_refresh)
                && !context.warp
            {
                opcodes.push(IrOpcode::hq_yield(HqYieldFields {
                    mode: YieldMode::Schedule(Rc::downgrade(&Step::from_block(
                        next_block,
                        next_id.clone(),
                        blocks,
                        context,
                        project,
                        final_next_blocks.clone(),
                    )?)),
                }));
                None
            } else {
                next_block.block_info()
            }
        } else if let (Some(popped_next), new_next_blocks_stack) =
            final_next_blocks.clone().pop_inner()
        {
            match popped_next.block {
                NextBlock::ID(id) => {
                    let next_block = blocks
                        .get(&id)
                        .ok_or_else(|| make_hq_bad_proj!("missing next block"))?;
                    if (popped_next.yield_first
                        || opcodes.last().is_some_and(
                            super::super::instructions::IrOpcode::requests_screen_refresh,
                        ))
                        && !context.warp
                    {
                        opcodes.push(IrOpcode::hq_yield(HqYieldFields {
                            mode: YieldMode::Schedule(Rc::downgrade(&Step::from_block(
                                next_block,
                                id.clone(),
                                blocks,
                                context,
                                project,
                                new_next_blocks_stack,
                            )?)),
                        }));
                        None
                    } else {
                        final_next_blocks = new_next_blocks_stack;
                        next_block.block_info()
                    }
                }
                NextBlock::Step(ref step) => {
                    if (popped_next.yield_first
                        || opcodes.last().is_some_and(
                            super::super::instructions::IrOpcode::requests_screen_refresh,
                        ))
                        && !context.warp
                    {
                        opcodes.push(IrOpcode::hq_yield(HqYieldFields {
                            mode: YieldMode::Schedule(Weak::clone(step)),
                        }));
                    } else {
                        opcodes.push(IrOpcode::hq_yield(HqYieldFields {
                            mode: YieldMode::Inline(
                                step.upgrade()
                                    .ok_or_else(|| make_hq_bug!("couldn't upgrade Weak<Step>"))?,
                            ),
                        }));
                    }
                    None
                }
            }
        } else if final_next_blocks.terminating() {
            opcodes.push(IrOpcode::hq_yield(HqYieldFields {
                mode: YieldMode::None,
            }));
            None
        } else {
            None
        }
    }
    Ok(opcodes.into_iter().collect())
}

fn from_special_block(block_array: &BlockArray, context: &StepContext) -> HQResult<IrOpcode> {
    Ok(match block_array {
        BlockArray::NumberOrAngle(ty, value) => match ty {
            // number, positive number or angle
            4 | 5 | 8 => {
                // proactively convert to an integer if possible;
                // if a float is needed, it will be cast at const-fold time (TODO),
                // and if integers are disabled (TODO) a float will be emitted anyway
                if value % 1.0 == 0.0 {
                    #[expect(
                        clippy::cast_possible_truncation,
                        reason = "integer-ness already confirmed; `as` is saturating."
                    )]
                    IrOpcode::hq_integer(HqIntegerFields(*value as i32))
                } else {
                    IrOpcode::hq_float(HqFloatFields(*value))
                }
            }
            // positive integer, integer
            6 | 7 => {
                hq_assert!(
                    value % 1.0 == 0.0,
                    "inputs of integer or positive integer types should be integers"
                );
                #[expect(
                    clippy::cast_possible_truncation,
                    reason = "integer-ness already confirmed; `as` is saturating."
                )]
                IrOpcode::hq_integer(HqIntegerFields(*value as i32))
            }
            _ => hq_bad_proj!("bad project json (block array of type ({}, f64))", ty),
        },
        // a string input should really be a colour or a string, but often numbers
        // are serialised as strings in the project.json
        BlockArray::ColorOrString(ty, value) => match ty {
            // number, positive number or integer
            4 | 5 | 8 => {
                let float = value
                    .parse()
                    .map_err(|_| make_hq_bug!("expected a float-parseable value"))?;
                // proactively convert to an integer if possible;
                // if a float is needed, it will be cast at const-fold time (TODO),
                // and if integers are disabled (TODO) a float will be emitted anyway
                if float % 1.0 == 0.0 {
                    #[expect(
                        clippy::cast_possible_truncation,
                        reason = "integer-ness already confirmed; `as` is saturating."
                    )]
                    IrOpcode::hq_integer(HqIntegerFields(float as i32))
                } else {
                    IrOpcode::hq_float(HqFloatFields(float))
                }
            }
            // integer, positive integer
            6 | 7 => IrOpcode::hq_integer(HqIntegerFields(
                value
                    .parse()
                    .map_err(|_| make_hq_bug!("expected and int-parseable value"))?,
            )),
            // colour
            9 => hq_todo!("colour inputs"),
            // string
            10 => 'textBlock: {
                // proactively convert to a number
                if let Ok(float) = value.parse::<f64>() {
                    if *float.to_string() == **value {
                        break 'textBlock if float % 1.0 == 0.0 {
                            #[expect(
                                clippy::cast_possible_truncation,
                                reason = "integer-ness already confirmed; `as` is saturating."
                            )]
                            IrOpcode::hq_integer(HqIntegerFields(float as i32))
                        } else {
                            IrOpcode::hq_float(HqFloatFields(float))
                        };
                    }
                }
                IrOpcode::hq_text(HqTextFields(value.clone()))
            }
            _ => hq_bad_proj!("bad project json (block array of type ({}, string))", ty),
        },
        BlockArray::Broadcast(ty, _name, id) | BlockArray::VariableOrList(ty, _name, id, _, _) => {
            match ty {
                11 => hq_todo!("broadcast input"),
                12 => {
                    let target = context
                        .target
                        .upgrade()
                        .ok_or_else(|| make_hq_bug!("couldn't upgrade Weak"))?;
                    let variable = if let Some(var) = target.variables().get(id) {
                        var
                    } else if let Some(var) = context
                        .target()?
                        .project()
                        .upgrade()
                        .ok_or_else(|| make_hq_bug!("couldn't upgrade Weak<Project>"))?
                        .global_variables()
                        .get(id)
                    {
                        &Rc::clone(var)
                    } else {
                        hq_bad_proj!("variable not found")
                    };
                    IrOpcode::data_variable(DataVariableFields(RcVar(Rc::clone(variable))))
                }
                13 => hq_todo!("list input"),
                _ => hq_bad_proj!(
                    "bad project json (block array of type ({}, string, string))",
                    ty
                ),
            }
        }
    })
}
