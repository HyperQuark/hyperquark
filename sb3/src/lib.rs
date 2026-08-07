pub mod raw;
pub mod structured;

use std::collections::BTreeMap;

pub use raw::*;
pub use structured::*;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ParseSb3ErrorKind {
    Syntax,
    Data,
    Eof,
    Io,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ParseSb3Error {
    pub kind: ParseSb3ErrorKind,
    pub line: usize,
    pub column: usize,
    pub message: String,
}

impl std::fmt::Display for ParseSb3Error {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(
            f,
            "{:?} error at project.json:{}:{}: {}",
            self.kind, self.line, self.column, self.message
        )
    }
}

impl std::error::Error for ParseSb3Error {}

impl TryFrom<raw::Sb3Project> for structured::StructuredProject {
    type Error = structured::StructuredConversionError;

    fn try_from(value: raw::Sb3Project) -> Result<Self, Self::Error> {
        let stage_count = value
            .targets
            .iter()
            .filter(|target| target.is_stage)
            .count();
        if stage_count == 0 {
            return Err(structured::StructuredConversionError::MissingStage);
        }
        if stage_count > 1 {
            return Err(structured::StructuredConversionError::DuplicateStage);
        }

        let stage = value
            .targets
            .iter()
            .find(|target| target.is_stage)
            .expect("checked above");

        let mut global_variables = structured::Registry::new();
        let mut global_lists = structured::Registry::new();
        let mut broadcasts = structured::Registry::new();
        let mut global_variable_ids = BTreeMap::new();
        let mut global_list_ids = BTreeMap::new();
        let mut broadcast_ids = BTreeMap::new();

        for (scratch_id, info) in &stage.variables {
            let id = global_variables.push(structured::Variable {
                scratch_id: scratch_id.clone(),
                info: info.clone(),
            });
            global_variable_ids.insert(scratch_id.clone(), id);
        }
        for (scratch_id, (name, value)) in &stage.lists {
            let id = global_lists.push(structured::List {
                scratch_id: scratch_id.clone(),
                name: name.clone(),
                value: value.clone(),
            });
            global_list_ids.insert(scratch_id.clone(), id);
        }
        for (scratch_id, name) in &stage.broadcasts {
            let id = broadcasts.push(structured::Broadcast {
                scratch_id: scratch_id.clone(),
                name: name.clone(),
            });
            broadcast_ids.insert(scratch_id.clone(), id);
        }

        let globals = GlobalRegistries {
            variable_ids: global_variable_ids,
            list_ids: global_list_ids,
            broadcast_ids,
        };

        let targets = value
            .targets
            .into_iter()
            .map(|target| {
                StructuredTargetParser {
                    global_variables: &globals.variable_ids,
                    global_lists: &globals.list_ids,
                    broadcasts: &globals.broadcast_ids,
                    target,
                }
                .parse()
            })
            .collect::<Result<Vec<_>, _>>()?;

        Ok(Self {
            global_variables,
            global_lists,
            broadcasts,
            targets,
            monitors: value.monitors,
            extensions: value.extensions,
            meta: value.meta,
        })
    }
}

impl TryFrom<structured::StructuredProject> for raw::Sb3Project {
    type Error = structured::StructuredConversionError;

    fn try_from(value: structured::StructuredProject) -> Result<Self, Self::Error> {
        let stage_count = value
            .targets
            .iter()
            .filter(|target| target.is_stage)
            .count();
        if stage_count == 0 {
            return Err(structured::StructuredConversionError::MissingStage);
        }
        if stage_count > 1 {
            return Err(structured::StructuredConversionError::DuplicateStage);
        }

        let raw_targets = value
            .targets
            .iter()
            .map(|target| RawTargetBuilder::new(&value, target).build())
            .collect::<Result<Vec<_>, _>>()?;

        Ok(Self {
            targets: raw_targets,
            monitors: value.monitors,
            extensions: value.extensions,
            meta: value.meta,
        })
    }
}

struct GlobalRegistries {
    variable_ids: BTreeMap<Box<str>, structured::GlobalVariableId>,
    list_ids: BTreeMap<Box<str>, structured::GlobalListId>,
    broadcast_ids: BTreeMap<Box<str>, structured::BroadcastId>,
}

struct StructuredTargetParser<'a> {
    global_variables: &'a BTreeMap<Box<str>, structured::GlobalVariableId>,
    global_lists: &'a BTreeMap<Box<str>, structured::GlobalListId>,
    broadcasts: &'a BTreeMap<Box<str>, structured::BroadcastId>,
    target: raw::Target,
}

impl StructuredTargetParser<'_> {
    fn parse(self) -> Result<structured::StructuredTarget, structured::StructuredConversionError> {
        let raw::Target {
            is_stage,
            name,
            variables,
            lists,
            broadcasts: _,
            blocks,
            comments,
            current_costume,
            costumes,
            sounds,
            layer_order,
            volume,
            tempo,
            video_state,
            video_transparency,
            text_to_speech_language,
            visible,
            x,
            y,
            size,
            direction,
            draggable,
            rotation_style,
        } = self.target;

        let mut local_variables = structured::Registry::new();
        let mut local_lists = structured::Registry::new();
        let mut local_procedures = structured::Registry::new();
        let mut local_variable_ids = BTreeMap::new();
        let mut local_list_ids = BTreeMap::new();
        let mut procedure_ids = BTreeMap::new();
        let mut prototype_ids = BTreeMap::new();
        let mut procedure_argument_ids = BTreeMap::new();

        if !is_stage {
            for (scratch_id, info) in variables {
                let id = local_variables.push(structured::Variable {
                    scratch_id: scratch_id.clone(),
                    info,
                });
                local_variable_ids.insert(scratch_id, id);
            }
            for (scratch_id, (list_name, value)) in lists {
                let id = local_lists.push(structured::List {
                    scratch_id: scratch_id.clone(),
                    name: list_name,
                    value,
                });
                local_list_ids.insert(scratch_id, id);
            }
        }

        for (block_id, block) in &blocks {
            let raw::Block::Normal { block_info, .. } = block else {
                continue;
            };
            if block_info.opcode != raw::BlockOpcode::procedures_prototype {
                continue;
            }
            let procedure = parse_procedure(block_id, block_info)?;
            let procedure_id = local_procedures.push(procedure);
            let procedure_info = local_procedures.get(procedure_id).expect("just inserted");
            procedure_ids.insert(procedure_info.proccode.clone(), procedure_id);
            prototype_ids.insert(block_id.clone(), procedure_id);

            let arg_ids =
                parse_mutation_string_array(block_id, &block_info.mutation, "argumentids")?;
            for (argument_index, arg_id) in arg_ids.into_iter().enumerate() {
                let argument_ref = structured::ProcedureArgumentRef {
                    procedure: procedure_id,
                    argument_index,
                };
                procedure_argument_ids.insert(arg_id.clone(), argument_ref);
                if let Some(raw::BlockArrayOrId::Id(reporter_id)) =
                    direct_input_ref(block_info, arg_id.as_ref())
                {
                    procedure_argument_ids.insert(reporter_id.clone(), argument_ref);
                }
            }
        }

        let parser = RawTargetParser {
            blocks: &blocks,
            global_variables: self.global_variables,
            local_variables: &local_variable_ids,
            global_lists: self.global_lists,
            local_lists: &local_list_ids,
            broadcasts: self.broadcasts,
            procedures: &procedure_ids,
            prototype_ids: &prototype_ids,
            procedure_argument_ids: &procedure_argument_ids,
            procedure_infos: &local_procedures,
        };

        let mut scripts = Vec::new();
        for (id, block) in &blocks {
            let raw::Block::Normal { x, y, block_info } = block else {
                continue;
            };
            if !block_info.top_level {
                continue;
            }

            let position = Some(structured::ScriptPosition { x: *x, y: *y });
            let script = match block_info.opcode {
                raw::BlockOpcode::event_whenflagclicked => structured::Script {
                    hat: Some(structured::Hat::WhenFlagClicked),
                    position,
                    body: parser.parse_stack(block_info.next.as_deref(), None)?,
                    top_reporter: None,
                },
                raw::BlockOpcode::event_whenbroadcastreceived => structured::Script {
                    hat: Some(structured::Hat::WhenBroadcastReceived {
                        broadcast: parser.parse_broadcast_field(
                            id,
                            block_info,
                            "BROADCAST_OPTION",
                        )?,
                    }),
                    position,
                    body: parser.parse_stack(block_info.next.as_deref(), None)?,
                    top_reporter: None,
                },
                raw::BlockOpcode::procedures_definition => {
                    let procedure = parser.parse_procedure_definition(id, block_info)?;
                    structured::Script {
                        hat: Some(structured::Hat::ProcedureDefinition { procedure }),
                        position,
                        body: parser.parse_stack(block_info.next.as_deref(), Some(procedure))?,
                        top_reporter: None,
                    }
                }
                _ => {
                    if let Ok(body) = parser.parse_stack(Some(id), None) {
                        structured::Script {
                            hat: None,
                            position,
                            body,
                            top_reporter: None,
                        }
                    } else {
                        structured::Script {
                            hat: None,
                            position,
                            body: vec![],
                            top_reporter: Some(parser.parse_value_ref(
                                id,
                                "top level block",
                                &raw::BlockArrayOrId::Id(id.clone()),
                                None,
                            )?),
                        }
                    }
                }
            };

            if script.body.is_empty() && script.hat.is_none() && script.top_reporter.is_none() {
                return Err(
                    structured::StructuredConversionError::UnexpectedTopLevelBlock {
                        block_id: id.clone(),
                        opcode: block_info.opcode.clone(),
                    },
                );
            }
            scripts.push(script);
        }

        Ok(structured::StructuredTarget {
            is_stage,
            name,
            local_variables,
            local_lists,
            local_procedures,
            scripts,
            comments: comments.into_iter().collect(),
            current_costume,
            costumes,
            sounds,
            layer_order,
            volume,
            tempo,
            video_state,
            video_transparency,
            text_to_speech_language,
            visible,
            x,
            y,
            size,
            direction,
            draggable,
            rotation_style,
        })
    }
}

struct RawTargetParser<'a> {
    blocks: &'a raw::BlockMap,
    global_variables: &'a BTreeMap<Box<str>, structured::GlobalVariableId>,
    local_variables: &'a BTreeMap<Box<str>, structured::LocalVariableId>,
    global_lists: &'a BTreeMap<Box<str>, structured::GlobalListId>,
    local_lists: &'a BTreeMap<Box<str>, structured::LocalListId>,
    broadcasts: &'a BTreeMap<Box<str>, structured::BroadcastId>,
    procedures: &'a BTreeMap<Box<str>, structured::ProcedureId>,
    prototype_ids: &'a BTreeMap<Box<str>, structured::ProcedureId>,
    procedure_argument_ids: &'a BTreeMap<Box<str>, structured::ProcedureArgumentRef>,
    procedure_infos: &'a structured::Registry<structured::ProcedureId, structured::Procedure>,
}

impl<'a> RawTargetParser<'a> {
    fn normal_block(
        &self,
        id: &str,
    ) -> Result<&'a raw::BlockInfo, structured::StructuredConversionError> {
        match self.blocks.get(id) {
            Some(raw::Block::Normal { block_info, .. }) => Ok(block_info),
            Some(raw::Block::Special(_)) => {
                Err(structured::StructuredConversionError::ExpectedNormalBlock {
                    block_id: id.into(),
                })
            }
            None => Err(structured::StructuredConversionError::MissingBlock {
                block_id: id.into(),
            }),
        }
    }

    fn parse_stack(
        &self,
        first: Option<&str>,
        current_procedure: Option<structured::ProcedureId>,
    ) -> Result<Vec<structured::Statement>, structured::StructuredConversionError> {
        let mut current = first;
        let mut out = Vec::new();
        while let Some(id) = current {
            let info = self.normal_block(id)?;
            out.push(self.parse_statement(id, info, current_procedure)?);
            current = info.next.as_deref();
        }
        Ok(out)
    }

    fn parse_statement(
        &self,
        block_id: &str,
        info: &raw::BlockInfo,
        current_procedure: Option<structured::ProcedureId>,
    ) -> Result<structured::Statement, structured::StructuredConversionError> {
        use raw::BlockOpcode as Op;
        use structured::Statement as Stmt;

        match info.opcode {
            Op::data_setvariableto => Ok(Stmt::SetVariable {
                variable: self.parse_variable_field(block_id, info, "VARIABLE")?,
                value: self.parse_value_input(block_id, info, "VALUE", current_procedure)?,
            }),
            Op::data_changevariableby => Ok(Stmt::ChangeVariable {
                variable: self.parse_variable_field(block_id, info, "VARIABLE")?,
                value: self.parse_value_input(block_id, info, "VALUE", current_procedure)?,
            }),
            Op::data_showvariable => Ok(Stmt::ShowVariable {
                variable: self.parse_variable_field(block_id, info, "VARIABLE")?,
            }),
            Op::data_hidevariable => Ok(Stmt::HideVariable {
                variable: self.parse_variable_field(block_id, info, "VARIABLE")?,
            }),
            Op::data_addtolist => Ok(Stmt::AddToList {
                list: self.parse_list_field(block_id, info, "LIST")?,
                item: self.parse_value_input(block_id, info, "ITEM", current_procedure)?,
            }),
            Op::data_deleteoflist => Ok(Stmt::DeleteOfList {
                list: self.parse_list_field(block_id, info, "LIST")?,
                index: self.parse_value_input(block_id, info, "INDEX", current_procedure)?,
            }),
            Op::data_deletealloflist => Ok(Stmt::DeleteAllOfList {
                list: self.parse_list_field(block_id, info, "LIST")?,
            }),
            Op::data_insertatlist => Ok(Stmt::InsertAtList {
                list: self.parse_list_field(block_id, info, "LIST")?,
                index: self.parse_value_input(block_id, info, "INDEX", current_procedure)?,
                item: self.parse_value_input(block_id, info, "ITEM", current_procedure)?,
            }),
            Op::data_replaceitemoflist => Ok(Stmt::ReplaceItemOfList {
                list: self.parse_list_field(block_id, info, "LIST")?,
                index: self.parse_value_input(block_id, info, "INDEX", current_procedure)?,
                item: self.parse_value_input(block_id, info, "ITEM", current_procedure)?,
            }),
            Op::data_showlist => Ok(Stmt::ShowList {
                list: self.parse_list_field(block_id, info, "LIST")?,
            }),
            Op::data_hidelist => Ok(Stmt::HideList {
                list: self.parse_list_field(block_id, info, "LIST")?,
            }),
            Op::control_wait => Ok(Stmt::Wait {
                duration: self.parse_value_input(block_id, info, "DURATION", current_procedure)?,
            }),
            Op::control_wait_until => Ok(Stmt::WaitUntil {
                condition: self.parse_predicate_input(
                    block_id,
                    info,
                    "CONDITION",
                    current_procedure,
                )?,
            }),
            Op::control_if => Ok(Stmt::If {
                condition: self.parse_predicate_input(
                    block_id,
                    info,
                    "CONDITION",
                    current_procedure,
                )?,
                body: self.parse_substack(block_id, info, "SUBSTACK", current_procedure)?,
            }),
            Op::control_if_else => Ok(Stmt::IfElse {
                condition: self.parse_predicate_input(
                    block_id,
                    info,
                    "CONDITION",
                    current_procedure,
                )?,
                then_body: self.parse_substack(block_id, info, "SUBSTACK", current_procedure)?,
                else_body: self.parse_substack(block_id, info, "SUBSTACK2", current_procedure)?,
            }),
            Op::control_repeat => Ok(Stmt::Repeat {
                times: self.parse_value_input(block_id, info, "TIMES", current_procedure)?,
                body: self.parse_substack(block_id, info, "SUBSTACK", current_procedure)?,
            }),
            Op::control_repeat_until => Ok(Stmt::RepeatUntil {
                condition: self.parse_predicate_input(
                    block_id,
                    info,
                    "CONDITION",
                    current_procedure,
                )?,
                body: self.parse_substack(block_id, info, "SUBSTACK", current_procedure)?,
            }),
            Op::control_while => Ok(Stmt::While {
                condition: self.parse_predicate_input(
                    block_id,
                    info,
                    "CONDITION",
                    current_procedure,
                )?,
                body: self.parse_substack(block_id, info, "SUBSTACK", current_procedure)?,
            }),
            Op::control_forever => Ok(Stmt::Forever {
                body: self.parse_substack(block_id, info, "SUBSTACK", current_procedure)?,
            }),
            Op::control_stop => Ok(Stmt::Stop {
                option: self.parse_stop_option(block_id, info)?,
            }),
            Op::event_broadcast => Ok(Stmt::Broadcast {
                broadcast: self.parse_broadcast_input(block_id, info, "BROADCAST_INPUT")?,
            }),
            Op::event_broadcastandwait => Ok(Stmt::BroadcastAndWait {
                broadcast: self.parse_broadcast_input(block_id, info, "BROADCAST_INPUT")?,
            }),
            Op::sensing_askandwait => Ok(Stmt::AskAndWait {
                question: self.parse_value_input(block_id, info, "QUESTION", current_procedure)?,
            }),
            Op::sensing_resettimer => Ok(Stmt::ResetTimer),
            Op::looks_say => Ok(Stmt::Say {
                message: self.parse_value_input(block_id, info, "MESSAGE", current_procedure)?,
            }),
            Op::looks_sayforsecs => Ok(Stmt::SayForSecs {
                message: self.parse_value_input(block_id, info, "MESSAGE", current_procedure)?,
                seconds: self.parse_value_input(block_id, info, "SECS", current_procedure)?,
            }),
            Op::looks_think => Ok(Stmt::Think {
                message: self.parse_value_input(block_id, info, "MESSAGE", current_procedure)?,
            }),
            Op::looks_thinkforsecs => Ok(Stmt::ThinkForSecs {
                message: self.parse_value_input(block_id, info, "MESSAGE", current_procedure)?,
                seconds: self.parse_value_input(block_id, info, "SECS", current_procedure)?,
            }),
            Op::looks_show => Ok(Stmt::Show),
            Op::looks_hide => Ok(Stmt::Hide),
            Op::looks_switchcostumeto => Ok(Stmt::SwitchCostumeTo {
                costume: self.parse_value_input(block_id, info, "COSTUME", current_procedure)?,
            }),
            Op::looks_switchbackdropto => Ok(Stmt::SwitchBackdropTo {
                backdrop: self.parse_value_input(block_id, info, "BACKDROP", current_procedure)?,
            }),
            Op::looks_switchbackdroptoandwait => Ok(Stmt::SwitchBackdropToAndWait {
                backdrop: self.parse_value_input(block_id, info, "BACKDROP", current_procedure)?,
            }),
            Op::looks_nextcostume => Ok(Stmt::NextCostume),
            Op::looks_nextbackdrop => Ok(Stmt::NextBackdrop),
            Op::looks_changesizeby => Ok(Stmt::ChangeSizeBy {
                amount: self.parse_value_input(block_id, info, "CHANGE", current_procedure)?,
            }),
            Op::looks_setsizeto => Ok(Stmt::SetSizeTo {
                size: self.parse_value_input(block_id, info, "SIZE", current_procedure)?,
            }),
            Op::motion_movesteps => Ok(Stmt::MoveSteps {
                steps: self.parse_value_input(block_id, info, "STEPS", current_procedure)?,
            }),
            Op::motion_gotoxy => Ok(Stmt::GoToXY {
                x: self.parse_value_input(block_id, info, "X", current_procedure)?,
                y: self.parse_value_input(block_id, info, "Y", current_procedure)?,
            }),
            Op::motion_turnright => Ok(Stmt::TurnRight {
                degrees: self.parse_value_input(block_id, info, "DEGREES", current_procedure)?,
            }),
            Op::motion_turnleft => Ok(Stmt::TurnLeft {
                degrees: self.parse_value_input(block_id, info, "DEGREES", current_procedure)?,
            }),
            Op::motion_pointindirection => Ok(Stmt::PointInDirection {
                direction: self.parse_value_input(
                    block_id,
                    info,
                    "DIRECTION",
                    current_procedure,
                )?,
            }),
            Op::motion_changexby => Ok(Stmt::ChangeXBy {
                amount: self.parse_value_input(block_id, info, "DX", current_procedure)?,
            }),
            Op::motion_setx => Ok(Stmt::SetX {
                value: self.parse_value_input(block_id, info, "X", current_procedure)?,
            }),
            Op::motion_changeyby => Ok(Stmt::ChangeYBy {
                amount: self.parse_value_input(block_id, info, "DY", current_procedure)?,
            }),
            Op::motion_sety => Ok(Stmt::SetY {
                value: self.parse_value_input(block_id, info, "Y", current_procedure)?,
            }),
            Op::procedures_call => Ok(Stmt::CallProcedure {
                procedure: self.lookup_procedure(
                    block_id,
                    &parse_mutation_string(block_id, &info.mutation, "proccode")?,
                )?,
                arguments: self.parse_procedure_call_inputs(block_id, info, current_procedure)?,
            }),
            Op::pen_setPenColorToColor => Ok(Stmt::PenSetColorToColor {
                value: self.parse_value_input(block_id, info, "COLOR", current_procedure)?,
            }),
            Op::pen_changePenColorParamBy => Ok(Stmt::PenChangeColorParamBy {
                param: self.parse_value_input(block_id, info, "COLOR_PARAM", current_procedure)?,
                value: self.parse_value_input(block_id, info, "VALUE", current_procedure)?,
            }),
            Op::pen_setPenColorParamTo => Ok(Stmt::PenSetColorParamTo {
                param: self.parse_value_input(block_id, info, "COLOR_PARAM", current_procedure)?,
                value: self.parse_value_input(block_id, info, "VALUE", current_procedure)?,
            }),
            Op::pen_setPenSizeTo => Ok(Stmt::PenSetSizeTo {
                value: self.parse_value_input(block_id, info, "SIZE", current_procedure)?,
            }),
            Op::pen_penDown => Ok(Stmt::PenDown),
            Op::pen_penUp => Ok(Stmt::PenUp),
            Op::pen_clear => Ok(Stmt::PenClear),
            _ => {
                println!("unsupported statement op");
                Err(structured::StructuredConversionError::UnsupportedOpcode {
                    opcode: info.opcode.clone(),
                    block_id: block_id.into(),
                })
            }
        }
    }

    fn parse_substack(
        &self,
        block_id: &str,
        info: &raw::BlockInfo,
        input_name: &'static str,
        current_procedure: Option<structured::ProcedureId>,
    ) -> Result<Vec<structured::Statement>, structured::StructuredConversionError> {
        let Some(reference) = self.input_ref(info, input_name) else {
            return Ok(Vec::new());
        };
        match reference {
            raw::BlockArrayOrId::Id(id) => self.parse_stack(Some(id), current_procedure),
            raw::BlockArrayOrId::Array(arr) => {
                Err(structured::StructuredConversionError::InvalidLiteral {
                    block_id: block_id.into(),
                    context: input_name,
                })
            }
        }
    }

    fn parse_value_input(
        &self,
        block_id: &str,
        info: &raw::BlockInfo,
        input_name: &'static str,
        current_procedure: Option<structured::ProcedureId>,
    ) -> Result<Option<structured::Value>, structured::StructuredConversionError> {
        self.input_ref(info, input_name)
            .map(|reference| {
                self.parse_value_ref(block_id, input_name, reference, current_procedure)
            })
            .transpose()
    }

    fn parse_predicate_input(
        &self,
        block_id: &str,
        info: &raw::BlockInfo,
        input_name: &'static str,
        current_procedure: Option<structured::ProcedureId>,
    ) -> Result<Option<structured::Predicate>, structured::StructuredConversionError> {
        self.input_ref(info, input_name)
            .map(|reference| {
                self.parse_predicate_ref(block_id, input_name, reference, current_procedure)
            })
            .transpose()
    }

    fn parse_value_ref(
        &self,
        block_id: &str,
        context: &'static str,
        reference: &raw::BlockArrayOrId,
        current_procedure: Option<structured::ProcedureId>,
    ) -> Result<structured::Value, structured::StructuredConversionError> {
        use raw::BlockOpcode as Op;
        use structured::Value;

        match reference {
            raw::BlockArrayOrId::Array(raw::BlockArray::NumberOrAngle(_, number)) => {
                Ok(Value::Literal(structured::Literal::Number(*number)))
            }
            raw::BlockArrayOrId::Array(raw::BlockArray::ColorOrString(i, string)) => {
                Ok(Value::Literal((if *i == 9 {
                    structured::Literal::Color
                } else {
                    structured::Literal::String
                })(string.clone())))
            }
            raw::BlockArrayOrId::Array(
                raw::BlockArray::VariableOrList(12, _name, id, ..)
                | raw::BlockArray::Broadcast(12, _name, id),
            ) => Ok(Value::Variable(self.lookup_variable(block_id, id)?)),
            raw::BlockArrayOrId::Array(
                raw::BlockArray::VariableOrList(13, _name, id, ..)
                | raw::BlockArray::Broadcast(13, _name, id),
            ) => Ok(Value::ListContents(self.lookup_list(block_id, id)?)),
            raw::BlockArrayOrId::Array(
                raw::BlockArray::VariableOrList(..) | raw::BlockArray::Broadcast(..),
            ) => Err(structured::StructuredConversionError::InvalidLiteral {
                block_id: block_id.into(),
                context,
            }),
            raw::BlockArrayOrId::Id(id) => {
                let info = self.normal_block(id)?;
                match info.opcode {
                    Op::data_variable => Ok(Value::Variable(
                        self.parse_variable_field(id, info, "VARIABLE")?,
                    )),
                    Op::data_listcontents => Ok(Value::ListContents(
                        self.parse_list_field(id, info, "LIST")?,
                    )),
                    Op::data_itemoflist => Ok(Value::ItemOfList {
                        list: self.parse_list_field(id, info, "LIST")?,
                        index: self
                            .parse_value_input(id, info, "INDEX", current_procedure)?
                            .map(Box::new),
                    }),
                    Op::data_itemnumoflist => Ok(Value::ItemNumOfList {
                        list: self.parse_list_field(id, info, "LIST")?,
                        item: self
                            .parse_value_input(id, info, "ITEM", current_procedure)?
                            .map(Box::new),
                    }),
                    Op::data_lengthoflist => Ok(Value::LengthOfList(
                        self.parse_list_field(id, info, "LIST")?,
                    )),
                    Op::operator_add => Ok(Value::Add(
                        self.parse_value_input(id, info, "NUM1", current_procedure)?
                            .map(Box::new),
                        self.parse_value_input(id, info, "NUM2", current_procedure)?
                            .map(Box::new),
                    )),
                    Op::operator_subtract => Ok(Value::Subtract(
                        self.parse_value_input(id, info, "NUM1", current_procedure)?
                            .map(Box::new),
                        self.parse_value_input(id, info, "NUM2", current_procedure)?
                            .map(Box::new),
                    )),
                    Op::operator_multiply => Ok(Value::Multiply(
                        self.parse_value_input(id, info, "NUM1", current_procedure)?
                            .map(Box::new),
                        self.parse_value_input(id, info, "NUM2", current_procedure)?
                            .map(Box::new),
                    )),
                    Op::operator_divide => Ok(Value::Divide(
                        self.parse_value_input(id, info, "NUM1", current_procedure)?
                            .map(Box::new),
                        self.parse_value_input(id, info, "NUM2", current_procedure)?
                            .map(Box::new),
                    )),
                    Op::operator_random => Ok(Value::Random(
                        self.parse_value_input(id, info, "FROM", current_procedure)?
                            .map(Box::new),
                        self.parse_value_input(id, info, "TO", current_procedure)?
                            .map(Box::new),
                    )),
                    Op::operator_join => Ok(Value::Join(
                        self.parse_value_input(id, info, "STRING1", current_procedure)?
                            .map(Box::new),
                        self.parse_value_input(id, info, "STRING2", current_procedure)?
                            .map(Box::new),
                    )),
                    Op::operator_letter_of => Ok(Value::LetterOf {
                        letter: self
                            .parse_value_input(id, info, "LETTER", current_procedure)?
                            .map(Box::new),
                        text: self
                            .parse_value_input(id, info, "STRING", current_procedure)?
                            .map(Box::new),
                    }),
                    Op::operator_length => Ok(Value::Length(
                        self.parse_value_input(id, info, "STRING", current_procedure)?
                            .map(Box::new),
                    )),
                    Op::operator_contains => Ok(Value::Contains {
                        text: self
                            .parse_value_input(id, info, "STRING1", current_procedure)?
                            .map(Box::new),
                        search: self
                            .parse_value_input(id, info, "STRING2", current_procedure)?
                            .map(Box::new),
                    }),
                    Op::operator_mod => Ok(Value::Modulo(
                        self.parse_value_input(id, info, "NUM1", current_procedure)?
                            .map(Box::new),
                        self.parse_value_input(id, info, "NUM2", current_procedure)?
                            .map(Box::new),
                    )),
                    Op::operator_round => Ok(Value::Round(
                        self.parse_value_input(id, info, "NUM", current_procedure)?
                            .map(Box::new),
                    )),
                    Op::operator_mathop => Ok(Value::MathOp {
                        operator: self.parse_math_operator(id, info)?,
                        operand: self
                            .parse_value_input(id, info, "NUM", current_procedure)?
                            .map(Box::new),
                    }),
                    Op::argument_reporter_string_number => Ok(Value::ProcedureArgument(
                        self.lookup_procedure_argument_by_name(
                            block_id,
                            current_procedure,
                            &self.parse_string_field_value(id, info, "VALUE")?,
                        )?,
                    )),
                    Op::operator_lt
                    | Op::operator_equals
                    | Op::operator_gt
                    | Op::operator_and
                    | Op::operator_or
                    | Op::operator_not
                    | Op::sensing_mousedown
                    | Op::data_listcontainsitem => Ok(Value::Predicate(Box::new(
                        self.parse_predicate_ref(block_id, context, reference, current_procedure)?,
                    ))),
                    Op::sensing_answer => Ok(Value::Answer),
                    Op::sensing_mousex => Ok(Value::MouseX),
                    Op::sensing_mousey => Ok(Value::MouseY),
                    Op::sensing_timer => Ok(Value::Timer),
                    Op::sensing_dayssince2000 => Ok(Value::DaysSince2000),
                    Op::sensing_keyoptions => Ok(Value::KeyOptions(
                        self.parse_string_field_value(id, info, "KEY_OPTION")?,
                    )),
                    Op::motion_xposition => Ok(Value::XPosition),
                    Op::motion_yposition => Ok(Value::YPosition),
                    Op::motion_direction => Ok(Value::Direction),
                    Op::looks_size => Ok(Value::Size),
                    Op::looks_costumenumbername => match self.parse_number_name_field(id, info)? {
                        NumberName::Number => Ok(Value::CostumeNumber),
                        NumberName::Name => {
                            Err(structured::StructuredConversionError::UnsupportedOpcode {
                                opcode: info.opcode.clone(),
                                block_id: id.clone(),
                            })
                        }
                    },
                    Op::looks_backdropnumbername => match self.parse_number_name_field(id, info)? {
                        NumberName::Number => Ok(Value::BackdropNumber),
                        NumberName::Name => {
                            Err(structured::StructuredConversionError::UnsupportedOpcode {
                                opcode: info.opcode.clone(),
                                block_id: id.clone(),
                            })
                        }
                    },
                    Op::sound_volume => Ok(Value::Volume),
                    Op::pen_menu_colorParam => Ok(Value::PenMenuColorParam(
                        self.parse_string_field_value(id, info, "colorParam")?,
                    )),
                    _ => Err(structured::StructuredConversionError::UnsupportedOpcode {
                        opcode: info.opcode.clone(),
                        block_id: id.clone(),
                    }),
                }
            }
        }
    }

    fn parse_predicate_ref(
        &self,
        block_id: &str,
        context: &'static str,
        reference: &raw::BlockArrayOrId,
        current_procedure: Option<structured::ProcedureId>,
    ) -> Result<structured::Predicate, structured::StructuredConversionError> {
        use raw::BlockOpcode as Op;
        use structured::Predicate as Pred;

        match reference {
            raw::BlockArrayOrId::Id(id) => {
                let info = self.normal_block(id)?;
                match info.opcode {
                    Op::operator_lt => Ok(Pred::LessThan(
                        self.parse_value_input(id, info, "OPERAND1", current_procedure)?
                            .map(Box::new),
                        self.parse_value_input(id, info, "OPERAND2", current_procedure)?
                            .map(Box::new),
                    )),
                    Op::operator_equals => Ok(Pred::Equals(
                        self.parse_value_input(id, info, "OPERAND1", current_procedure)?
                            .map(Box::new),
                        self.parse_value_input(id, info, "OPERAND2", current_procedure)?
                            .map(Box::new),
                    )),
                    Op::operator_gt => Ok(Pred::GreaterThan(
                        self.parse_value_input(id, info, "OPERAND1", current_procedure)?
                            .map(Box::new),
                        self.parse_value_input(id, info, "OPERAND2", current_procedure)?
                            .map(Box::new),
                    )),
                    Op::operator_and => Ok(Pred::And(
                        self.parse_predicate_input(id, info, "OPERAND1", current_procedure)?
                            .map(Box::new),
                        self.parse_predicate_input(id, info, "OPERAND2", current_procedure)?
                            .map(Box::new),
                    )),
                    Op::operator_or => Ok(Pred::Or(
                        self.parse_predicate_input(id, info, "OPERAND1", current_procedure)?
                            .map(Box::new),
                        self.parse_predicate_input(id, info, "OPERAND2", current_procedure)?
                            .map(Box::new),
                    )),
                    Op::operator_not => Ok(Pred::Not(
                        self.parse_predicate_input(id, info, "OPERAND", current_procedure)?
                            .map(Box::new),
                    )),
                    Op::sensing_mousedown => Ok(Pred::MouseDown),
                    Op::data_listcontainsitem => Ok(Pred::ListContainsItem {
                        list: self.parse_list_field(id, info, "LIST")?,
                        item: self
                            .parse_value_input(id, info, "ITEM", current_procedure)?
                            .map(Box::new),
                    }),
                    Op::data_itemoflist => Ok(Pred::ItemOfList {
                        list: self.parse_list_field(id, info, "LIST")?,
                        index: self
                            .parse_value_input(id, info, "INDEX", current_procedure)?
                            .map(Box::new),
                    }),
                    Op::data_itemnumoflist => Ok(Pred::ItemNumOfList {
                        list: self.parse_list_field(id, info, "LIST")?,
                        item: self
                            .parse_value_input(id, info, "ITEM", current_procedure)?
                            .map(Box::new),
                    }),
                    Op::argument_reporter_boolean => Ok(Pred::ProcedureArgument(
                        self.lookup_procedure_argument_by_name(
                            block_id,
                            current_procedure,
                            &self.parse_string_field_value(id, info, "VALUE")?,
                        )?,
                    )),
                    Op::sensing_keypressed => Ok(Pred::KeyPressed(
                        self.parse_value_input(id, info, "KEY_OPTION", current_procedure)?
                            .map(Box::new),
                    )),
                    _ => Err(structured::StructuredConversionError::InvalidLiteral {
                        block_id: block_id.into(),
                        context,
                    }),
                }
            }
            _ => Err(structured::StructuredConversionError::InvalidLiteral {
                block_id: block_id.into(),
                context,
            }),
        }
    }

    fn parse_math_operator(
        &self,
        block_id: &str,
        info: &raw::BlockInfo,
    ) -> Result<structured::MathOperator, structured::StructuredConversionError> {
        let op = self.parse_string_field_value(block_id, info, "OPERATOR")?;
        Ok(match op.as_ref() {
            "abs" => structured::MathOperator::Abs,
            "floor" => structured::MathOperator::Floor,
            "ceiling" => structured::MathOperator::Ceiling,
            "sqrt" => structured::MathOperator::Sqrt,
            "sin" => structured::MathOperator::Sin,
            "cos" => structured::MathOperator::Cos,
            "tan" => structured::MathOperator::Tan,
            "asin" => structured::MathOperator::Asin,
            "acos" => structured::MathOperator::Acos,
            "atan" => structured::MathOperator::Atan,
            "ln" => structured::MathOperator::Ln,
            "log" => structured::MathOperator::Log,
            "e ^" => structured::MathOperator::Exp,
            "10 ^" => structured::MathOperator::Pow10,
            _ => {
                return Err(structured::StructuredConversionError::InvalidLiteral {
                    block_id: block_id.into(),
                    context: "OPERATOR",
                });
            }
        })
    }

    fn parse_stop_option(
        &self,
        block_id: &str,
        info: &raw::BlockInfo,
    ) -> Result<structured::StopOption, structured::StructuredConversionError> {
        Ok(
            match self
                .parse_string_field_value(block_id, info, "STOP_OPTION")?
                .as_ref()
            {
                "all" => structured::StopOption::All,
                "this script" => structured::StopOption::ThisScript,
                "other scripts in sprite" => structured::StopOption::OtherScriptsInSprite,
                _ => {
                    return Err(structured::StructuredConversionError::InvalidLiteral {
                        block_id: block_id.into(),
                        context: "STOP_OPTION",
                    });
                }
            },
        )
    }

    fn parse_number_name_field(
        &self,
        block_id: &str,
        info: &raw::BlockInfo,
    ) -> Result<NumberName, structured::StructuredConversionError> {
        Ok(
            match self
                .parse_string_field_value(block_id, info, "NUMBER_NAME")?
                .as_ref()
            {
                "number" => NumberName::Number,
                "name" => NumberName::Name,
                _ => {
                    return Err(structured::StructuredConversionError::InvalidLiteral {
                        block_id: block_id.into(),
                        context: "NUMBER_NAME",
                    });
                }
            },
        )
    }

    fn parse_string_field_value(
        &self,
        block_id: &str,
        info: &raw::BlockInfo,
        field_name: &'static str,
    ) -> Result<Box<str>, structured::StructuredConversionError> {
        let field = info.fields.get(field_name).ok_or_else(|| {
            structured::StructuredConversionError::MissingField {
                block_id: block_id.into(),
                field: field_name,
            }
        })?;
        let value = field.get_0().ok_or_else(|| {
            structured::StructuredConversionError::MissingFieldValue {
                block_id: block_id.into(),
                field: field_name,
            }
        })?;
        let raw::VarVal::String(value) = value else {
            return Err(structured::StructuredConversionError::InvalidLiteral {
                block_id: block_id.into(),
                context: field_name,
            });
        };
        Ok(value.clone())
    }

    fn parse_variable_field(
        &self,
        block_id: &str,
        info: &raw::BlockInfo,
        field_name: &'static str,
    ) -> Result<structured::VariableRef, structured::StructuredConversionError> {
        let (_name, id) = self.parse_named_id_field(block_id, info, field_name)?;
        self.lookup_variable(block_id, &id)
    }

    fn parse_list_field(
        &self,
        block_id: &str,
        info: &raw::BlockInfo,
        field_name: &'static str,
    ) -> Result<structured::ListRef, structured::StructuredConversionError> {
        let (_name, id) = self.parse_named_id_field(block_id, info, field_name)?;
        self.lookup_list(block_id, &id)
    }

    fn parse_broadcast_field(
        &self,
        block_id: &str,
        info: &raw::BlockInfo,
        field_name: &'static str,
    ) -> Result<structured::BroadcastId, structured::StructuredConversionError> {
        let (_name, id) = self.parse_named_id_field(block_id, info, field_name)?;
        self.lookup_broadcast(block_id, &id)
    }

    fn parse_named_id_field(
        &self,
        block_id: &str,
        info: &raw::BlockInfo,
        field_name: &'static str,
    ) -> Result<(Box<str>, Box<str>), structured::StructuredConversionError> {
        let field = info.fields.get(field_name).ok_or_else(|| {
            structured::StructuredConversionError::MissingField {
                block_id: block_id.into(),
                field: field_name,
            }
        })?;
        let name = field.get_0().ok_or_else(|| {
            structured::StructuredConversionError::MissingFieldValue {
                block_id: block_id.into(),
                field: field_name,
            }
        })?;
        let raw::VarVal::String(name) = name else {
            return Err(
                structured::StructuredConversionError::InvalidVariableField {
                    block_id: block_id.into(),
                    field: field_name,
                },
            );
        };
        let raw::Field::ValueId(_, Some(id)) = field else {
            return Err(
                structured::StructuredConversionError::InvalidVariableField {
                    block_id: block_id.into(),
                    field: field_name,
                },
            );
        };
        Ok((name.clone(), id.clone()))
    }

    fn parse_broadcast_input(
        &self,
        block_id: &str,
        info: &raw::BlockInfo,
        input_name: &'static str,
    ) -> Result<structured::BroadcastId, structured::StructuredConversionError> {
        let reference = self.input_ref(info, input_name).ok_or_else(|| {
            structured::StructuredConversionError::MissingInput {
                block_id: block_id.into(),
                input: input_name,
            }
        })?;
        match reference {
            raw::BlockArrayOrId::Array(raw::BlockArray::Broadcast(_, _name, id)) => {
                self.lookup_broadcast(block_id, id)
            }
            raw::BlockArrayOrId::Id(id) => {
                let info = self.normal_block(id)?;
                if info.opcode != raw::BlockOpcode::event_broadcast_menu {
                    return Err(
                        structured::StructuredConversionError::InvalidBroadcastInput {
                            block_id: block_id.into(),
                            input: input_name,
                        },
                    );
                }
                self.parse_broadcast_field(id, info, "BROADCAST_OPTION")
            }
            _ => Err(
                structured::StructuredConversionError::InvalidBroadcastInput {
                    block_id: block_id.into(),
                    input: input_name,
                },
            ),
        }
    }

    fn parse_procedure_definition(
        &self,
        block_id: &str,
        info: &raw::BlockInfo,
    ) -> Result<structured::ProcedureId, structured::StructuredConversionError> {
        let Some(raw::BlockArrayOrId::Id(id)) = self.input_ref(info, "custom_block") else {
            return Err(structured::StructuredConversionError::MissingInput {
                block_id: block_id.into(),
                input: "custom_block",
            });
        };
        self.prototype_ids.get(id).copied().ok_or_else(|| {
            structured::StructuredConversionError::UnknownProcedure {
                block_id: block_id.into(),
                proccode: id.clone(),
            }
        })
    }

    fn parse_procedure_call_inputs(
        &self,
        block_id: &str,
        info: &raw::BlockInfo,
        current_procedure: Option<ProcedureId>,
    ) -> Result<Vec<structured::ProcedureInput>, structured::StructuredConversionError> {
        let proccode = parse_mutation_string(block_id, &info.mutation, "proccode")?;
        let procedure_id = self.lookup_procedure(block_id, &proccode)?;
        let arg_ids = parse_mutation_string_array(block_id, &info.mutation, "argumentids")?;
        let mut out = Vec::new();
        for (argument_index, arg_id) in arg_ids.iter().enumerate() {
            let argument_ref = structured::ProcedureArgumentRef {
                procedure: procedure_id,
                argument_index,
            };
            let kind = self.procedure_argument_kind(block_id, argument_ref)?;
            out.push(match kind {
                structured::ProcedureArgumentKind::StringOrNumber => {
                    structured::ProcedureInput::Value(
                        self.input_ref(info, arg_id)
                            .map(|reference| {
                                self.parse_value_ref(
                                    block_id,
                                    "procedure_argument",
                                    reference,
                                    current_procedure,
                                )
                            })
                            .transpose()?,
                    )
                }
                structured::ProcedureArgumentKind::Boolean => {
                    structured::ProcedureInput::Predicate(
                        self.input_ref(info, arg_id)
                            .map(|reference| {
                                self.parse_predicate_ref(
                                    block_id,
                                    "procedure_argument",
                                    reference,
                                    current_procedure,
                                )
                            })
                            .transpose()?,
                    )
                }
            });
        }
        Ok(out)
    }

    fn lookup_variable(
        &self,
        block_id: &str,
        scratch_id: &str,
    ) -> Result<structured::VariableRef, structured::StructuredConversionError> {
        if let Some(id) = self.local_variables.get(scratch_id) {
            return Ok(structured::VariableRef::Local(*id));
        }
        if let Some(id) = self.global_variables.get(scratch_id) {
            return Ok(structured::VariableRef::Global(*id));
        }
        Err(structured::StructuredConversionError::UnknownVariable {
            block_id: block_id.into(),
            variable_id: scratch_id.into(),
        })
    }

    fn lookup_list(
        &self,
        block_id: &str,
        scratch_id: &str,
    ) -> Result<structured::ListRef, structured::StructuredConversionError> {
        if let Some(id) = self.local_lists.get(scratch_id) {
            return Ok(structured::ListRef::Local(*id));
        }
        if let Some(id) = self.global_lists.get(scratch_id) {
            return Ok(structured::ListRef::Global(*id));
        }
        Err(structured::StructuredConversionError::UnknownList {
            block_id: block_id.into(),
            list_id: scratch_id.into(),
        })
    }

    fn lookup_broadcast(
        &self,
        block_id: &str,
        scratch_id: &str,
    ) -> Result<structured::BroadcastId, structured::StructuredConversionError> {
        self.broadcasts.get(scratch_id).copied().ok_or_else(|| {
            structured::StructuredConversionError::UnknownBroadcast {
                block_id: block_id.into(),
                broadcast_id: scratch_id.into(),
            }
        })
    }

    fn lookup_procedure(
        &self,
        block_id: &str,
        proccode: &str,
    ) -> Result<structured::ProcedureId, structured::StructuredConversionError> {
        self.procedures.get(proccode).copied().ok_or_else(|| {
            structured::StructuredConversionError::UnknownProcedure {
                block_id: block_id.into(),
                proccode: proccode.into(),
            }
        })
    }

    fn lookup_procedure_argument_by_name(
        &self,
        block_id: &str,
        current_procedure: Option<structured::ProcedureId>,
        name: &str,
    ) -> Result<structured::ProcedureArgumentRef, structured::StructuredConversionError> {
        let Some(procedure) = current_procedure else {
            return Ok(structured::ProcedureArgumentRef {
                procedure: ProcedureId(usize::MAX),
                argument_index: usize::MAX,
            });
            // TODO temporary solution. should find some way to represent this properly
        };
        /*.ok_or_else(|| {
            structured::StructuredConversionError::UnknownProcedure {
                block_id: block_id.into(),
                proccode: format!("procedure context missing; looking for arg {}", name).into(),
            }
        })?;*/
        let procedure_info = self.procedure_infos.get(procedure).ok_or_else(|| {
            structured::StructuredConversionError::UnknownProcedure {
                block_id: block_id.into(),
                proccode: format!("{procedure:?}").into(),
            }
        })?;
        let Some((argument_index, _)) = procedure_info
            .arguments
            .iter()
            .enumerate()
            .find(|(_, argument)| argument.name.as_ref() == name)
        else {
            return Err(
                structured::StructuredConversionError::UnknownProcedureArgument {
                    block_id: block_id.into(),
                    procedure,
                    argument_name: name.into(),
                },
            );
        };
        Ok(structured::ProcedureArgumentRef {
            procedure,
            argument_index,
        })
    }

    fn procedure_argument_kind(
        &self,
        block_id: &str,
        argument: structured::ProcedureArgumentRef,
    ) -> Result<structured::ProcedureArgumentKind, structured::StructuredConversionError> {
        self.procedure_argument_ids
            .values()
            .find(|value| **value == argument)
            .map(|_| ())
            .ok_or_else(
                || structured::StructuredConversionError::UnknownProcedureArgument {
                    block_id: block_id.into(),
                    procedure: argument.procedure,
                    argument_name: argument.argument_index.to_string().into(),
                },
            )?;
        let is_boolean = self.procedure_argument_ids.iter().any(|(id, value)| {
            *value == argument
                && matches!(
                    self.blocks.get(id.as_ref()),
                    Some(raw::Block::Normal {
                        block_info: raw::BlockInfo {
                            opcode: raw::BlockOpcode::argument_reporter_boolean,
                            ..
                        },
                        ..
                    })
                )
        });
        Ok(if is_boolean {
            structured::ProcedureArgumentKind::Boolean
        } else {
            structured::ProcedureArgumentKind::StringOrNumber
        })
    }

    fn input_ref<'b>(
        &self,
        info: &'b raw::BlockInfo,
        input_name: &str,
    ) -> Option<&'b raw::BlockArrayOrId> {
        match info.inputs.get(input_name) {
            Some(raw::Input::Shadow(_, primary, fallback)) => {
                primary.as_ref().or(fallback.as_ref())
            }
            Some(raw::Input::NoShadow(_, value)) => value.as_ref(),
            None => None,
        }
    }
}

#[derive(Clone, Copy)]
enum NumberName {
    Number,
    Name,
}

fn direct_input_ref<'a>(
    info: &'a raw::BlockInfo,
    input_name: &str,
) -> Option<&'a raw::BlockArrayOrId> {
    match info.inputs.get(input_name) {
        Some(raw::Input::Shadow(_, primary, fallback)) => primary.as_ref().or(fallback.as_ref()),
        Some(raw::Input::NoShadow(_, value)) => value.as_ref(),
        None => None,
    }
}

fn parse_mutation_string(
    block_id: &str,
    mutation: &raw::Mutation,
    property: &'static str,
) -> Result<Box<str>, structured::StructuredConversionError> {
    let Some(value) = mutation.mutations.get(property) else {
        return Err(structured::StructuredConversionError::MissingMutation {
            block_id: block_id.into(),
            property,
        });
    };
    let Some(value) = value.as_str() else {
        return Err(structured::StructuredConversionError::InvalidMutation {
            block_id: block_id.into(),
            property,
        });
    };
    Ok(value.into())
}

fn parse_mutation_bool(
    block_id: &str,
    mutation: &raw::Mutation,
    property: &'static str,
) -> Result<bool, structured::StructuredConversionError> {
    match parse_mutation_string(block_id, mutation, property)?.as_ref() {
        "true" => Ok(true),
        "false" => Ok(false),
        _ => Err(structured::StructuredConversionError::InvalidMutation {
            block_id: block_id.into(),
            property,
        }),
    }
}

fn parse_mutation_string_array(
    block_id: &str,
    mutation: &raw::Mutation,
    property: &'static str,
) -> Result<Vec<Box<str>>, structured::StructuredConversionError> {
    let raw = parse_mutation_string(block_id, mutation, property)?;
    serde_json::from_str::<Vec<String>>(&raw)
        .map(|items| items.into_iter().map(String::into_boxed_str).collect())
        .map_err(|_| structured::StructuredConversionError::InvalidMutation {
            block_id: block_id.into(),
            property,
        })
}

fn parse_mutation_defaults(
    block_id: &str,
    mutation: &raw::Mutation,
) -> Result<Vec<structured::ProcedureArgumentDefault>, structured::StructuredConversionError> {
    let raw = parse_mutation_string(block_id, mutation, "argumentdefaults")?;
    let values = serde_json::from_str::<Vec<serde_json::Value>>(&raw).map_err(|_| {
        structured::StructuredConversionError::InvalidMutation {
            block_id: block_id.into(),
            property: "argumentdefaults",
        }
    })?;
    values
        .into_iter()
        .map(|value| match value {
            serde_json::Value::Bool(value) => {
                Ok(structured::ProcedureArgumentDefault::Boolean(value))
            }
            serde_json::Value::String(value) => Ok(structured::ProcedureArgumentDefault::String(
                value.into_boxed_str(),
            )),
            serde_json::Value::Number(number) => Ok(structured::ProcedureArgumentDefault::String(
                number.to_string().into_boxed_str(),
            )),
            _ => Err(structured::StructuredConversionError::InvalidMutation {
                block_id: block_id.into(),
                property: "argumentdefaults",
            }),
        })
        .collect()
}

fn parse_procedure(
    block_id: &str,
    info: &raw::BlockInfo,
) -> Result<structured::Procedure, structured::StructuredConversionError> {
    let proccode = parse_mutation_string(block_id, &info.mutation, "proccode")?;
    let arg_names = parse_mutation_string_array(block_id, &info.mutation, "argumentnames")?;
    let arg_ids = parse_mutation_string_array(block_id, &info.mutation, "argumentids")?;
    let defaults = parse_mutation_defaults(block_id, &info.mutation)?;
    let warp = parse_mutation_bool(block_id, &info.mutation, "warp")?;

    if arg_names.len() != arg_ids.len() || arg_names.len() > defaults.len() {
        return Err(structured::StructuredConversionError::InvalidMutation {
            block_id: block_id.into(),
            property: "argument metadata",
        });
    }

    let mut arguments = Vec::with_capacity(arg_names.len());
    for ((name, arg_id), default) in arg_names.into_iter().zip(arg_ids).zip(defaults) {
        let Some(raw::BlockArrayOrId::Id(reporter_id)) = direct_input_ref(info, arg_id.as_ref())
        else {
            return Err(structured::StructuredConversionError::MissingInput {
                block_id: block_id.into(),
                input: "procedure argument reporter",
            });
        };
        let kind = match info.inputs.get(arg_id.as_ref()) {
            Some(_) => match &default {
                structured::ProcedureArgumentDefault::Boolean(_) => {
                    structured::ProcedureArgumentKind::Boolean
                }
                structured::ProcedureArgumentDefault::String(_) => {
                    let _ = reporter_id;
                    structured::ProcedureArgumentKind::StringOrNumber
                }
            },
            None => {
                return Err(structured::StructuredConversionError::MissingInput {
                    block_id: block_id.into(),
                    input: "procedure argument reporter",
                });
            }
        };
        arguments.push(structured::ProcedureArgument {
            name,
            kind,
            default,
        });
    }

    Ok(structured::Procedure {
        proccode,
        arguments,
        warp,
    })
}

struct RawTargetBuilder<'a> {
    project: &'a structured::StructuredProject,
    target: &'a structured::StructuredTarget,
    blocks: raw::BlockMap,
    procedure_layouts: BTreeMap<structured::ProcedureId, EmittedProcedureLayout>,
    next_id: usize,
}

struct EmittedProcedureLayout {
    argument_ids: Vec<Box<str>>,
}

impl<'a> RawTargetBuilder<'a> {
    fn new(
        project: &'a structured::StructuredProject,
        target: &'a structured::StructuredTarget,
    ) -> Self {
        Self {
            project,
            target,
            blocks: BTreeMap::new(),
            procedure_layouts: BTreeMap::new(),
            next_id: 0,
        }
    }

    fn build(mut self) -> Result<raw::Target, structured::StructuredConversionError> {
        for script in &self.target.scripts {
            self.emit_script(script)?;
        }

        Ok(raw::Target {
            is_stage: self.target.is_stage,
            name: self.target.name.clone(),
            variables: self.raw_variables(),
            lists: self.raw_lists(),
            broadcasts: if self.target.is_stage {
                self.project
                    .broadcasts
                    .items
                    .iter()
                    .map(|broadcast| (broadcast.scratch_id.clone(), broadcast.name.clone()))
                    .collect()
            } else {
                BTreeMap::new()
            },
            blocks: self.blocks,
            comments: self.target.comments.iter().cloned().collect(),
            current_costume: self.target.current_costume,
            costumes: self.target.costumes.clone(),
            sounds: self.target.sounds.clone(),
            layer_order: self.target.layer_order,
            volume: self.target.volume,
            tempo: self.target.tempo,
            video_state: self.target.video_state.clone(),
            video_transparency: self.target.video_transparency,
            text_to_speech_language: self.target.text_to_speech_language.clone(),
            visible: self.target.visible,
            x: self.target.x,
            y: self.target.y,
            size: self.target.size,
            direction: self.target.direction,
            draggable: self.target.draggable,
            rotation_style: self.target.rotation_style.clone(),
        })
    }

    fn raw_variables(&self) -> BTreeMap<Box<str>, raw::VariableInfo> {
        if self.target.is_stage {
            self.project
                .global_variables
                .items
                .iter()
                .map(|variable| (variable.scratch_id.clone(), variable.info.clone()))
                .collect()
        } else {
            self.target
                .local_variables
                .items
                .iter()
                .map(|variable| (variable.scratch_id.clone(), variable.info.clone()))
                .collect()
        }
    }

    fn raw_lists(&self) -> BTreeMap<Box<str>, (Box<str>, Vec<raw::VarVal>)> {
        if self.target.is_stage {
            self.project
                .global_lists
                .items
                .iter()
                .map(|list| {
                    (
                        list.scratch_id.clone(),
                        (list.name.clone(), list.value.clone()),
                    )
                })
                .collect()
        } else {
            self.target
                .local_lists
                .items
                .iter()
                .map(|list| {
                    (
                        list.scratch_id.clone(),
                        (list.name.clone(), list.value.clone()),
                    )
                })
                .collect()
        }
    }

    fn emit_script(
        &mut self,
        script: &structured::Script,
    ) -> Result<(), structured::StructuredConversionError> {
        let position = script
            .position
            .unwrap_or(structured::ScriptPosition { x: 0, y: 0 });
        match &script.hat {
            Some(structured::Hat::ProcedureDefinition { procedure }) => {
                let definition_id = self.fresh_id("hat");
                let prototype_id =
                    self.emit_procedure_prototype(definition_id.as_ref(), *procedure)?;
                let body_first =
                    self.emit_statement_chain(Some(definition_id.clone()), &script.body, None)?;
                let mut inputs = BTreeMap::new();
                inputs.insert(
                    "custom_block".into(),
                    raw::Input::NoShadow(1, Some(raw::BlockArrayOrId::Id(prototype_id))),
                );
                self.blocks.insert(
                    definition_id,
                    raw::Block::Normal {
                        x: position.x,
                        y: position.y,
                        block_info: raw::BlockInfo {
                            opcode: raw::BlockOpcode::procedures_definition,
                            next: body_first,
                            parent: None,
                            inputs,
                            fields: BTreeMap::new(),
                            shadow: false,
                            top_level: true,
                            mutation: raw::Mutation::default(),
                        },
                    },
                );
            }
            Some(hat) => {
                let hat_id = self.fresh_id("hat");
                let body_first =
                    self.emit_statement_chain(Some(hat_id.clone()), &script.body, None)?;
                let (opcode, fields) = self.hat_to_raw(hat)?;
                self.blocks.insert(
                    hat_id,
                    raw::Block::Normal {
                        x: position.x,
                        y: position.y,
                        block_info: raw::BlockInfo {
                            opcode,
                            next: body_first,
                            parent: None,
                            inputs: BTreeMap::new(),
                            fields,
                            shadow: false,
                            top_level: true,
                            mutation: raw::Mutation::default(),
                        },
                    },
                );
            }
            None => {
                self.emit_statement_chain(None, &script.body, Some(position))?;
            }
        }
        Ok(())
    }

    fn hat_to_raw(
        &self,
        hat: &structured::Hat,
    ) -> Result<
        (raw::BlockOpcode, BTreeMap<Box<str>, raw::Field>),
        structured::StructuredConversionError,
    > {
        let mut fields = BTreeMap::new();
        let opcode = match hat {
            structured::Hat::WhenFlagClicked => raw::BlockOpcode::event_whenflagclicked,
            structured::Hat::WhenBroadcastReceived { broadcast } => {
                fields.insert("BROADCAST_OPTION".into(), self.broadcast_field(*broadcast)?);
                raw::BlockOpcode::event_whenbroadcastreceived
            }
            structured::Hat::ProcedureDefinition { .. } => {
                return Err(structured::StructuredConversionError::InvalidLiteral {
                    block_id: "emit_procedure_definition".into(),
                    context: "hat",
                });
            }
        };
        Ok((opcode, fields))
    }

    fn emit_statement_chain(
        &mut self,
        first_parent: Option<Box<str>>,
        statements: &[structured::Statement],
        top_level_position: Option<structured::ScriptPosition>,
    ) -> Result<Option<Box<str>>, structured::StructuredConversionError> {
        let mut first_id = None;
        let mut previous_id = None;
        for (index, statement) in statements.iter().enumerate() {
            let parent = if index == 0 {
                first_parent.clone()
            } else {
                previous_id.clone()
            };
            let current_id = self.emit_statement(parent, statement)?;
            if first_id.is_none() {
                first_id = Some(current_id.clone());
                if let Some(position) = top_level_position {
                    self.make_top_level(current_id.as_ref(), position);
                }
            }
            if let Some(previous) = &previous_id {
                self.set_next(previous.as_ref(), current_id.clone());
            }
            previous_id = Some(current_id);
        }
        Ok(first_id)
    }

    fn emit_statement(
        &mut self,
        parent: Option<Box<str>>,
        statement: &structured::Statement,
    ) -> Result<Box<str>, structured::StructuredConversionError> {
        use structured::Statement as Stmt;

        let id = self.fresh_id("stmt");
        let mut info = raw::BlockInfo {
            opcode: raw::BlockOpcode::control_wait,
            next: None,
            parent,
            inputs: BTreeMap::new(),
            fields: BTreeMap::new(),
            shadow: false,
            top_level: false,
            mutation: raw::Mutation::default(),
        };

        match statement {
            Stmt::SetVariable { variable, value } => {
                info.opcode = raw::BlockOpcode::data_setvariableto;
                info.fields
                    .insert("VARIABLE".into(), self.variable_field(variable)?);
                info.inputs
                    .insert("VALUE".into(), self.value_input(&id, value)?);
            }
            Stmt::ChangeVariable { variable, value } => {
                info.opcode = raw::BlockOpcode::data_changevariableby;
                info.fields
                    .insert("VARIABLE".into(), self.variable_field(variable)?);
                info.inputs
                    .insert("VALUE".into(), self.value_input(&id, value)?);
            }
            Stmt::ShowVariable { variable } => {
                info.opcode = raw::BlockOpcode::data_showvariable;
                info.fields
                    .insert("VARIABLE".into(), self.variable_field(variable)?);
            }
            Stmt::HideVariable { variable } => {
                info.opcode = raw::BlockOpcode::data_hidevariable;
                info.fields
                    .insert("VARIABLE".into(), self.variable_field(variable)?);
            }
            Stmt::AddToList { list, item } => {
                info.opcode = raw::BlockOpcode::data_addtolist;
                info.fields.insert("LIST".into(), self.list_field(list)?);
                info.inputs
                    .insert("ITEM".into(), self.value_input(&id, item)?);
            }
            Stmt::DeleteOfList { list, index } => {
                info.opcode = raw::BlockOpcode::data_deleteoflist;
                info.fields.insert("LIST".into(), self.list_field(list)?);
                info.inputs
                    .insert("INDEX".into(), self.value_input(&id, index)?);
            }
            Stmt::DeleteAllOfList { list } => {
                info.opcode = raw::BlockOpcode::data_deletealloflist;
                info.fields.insert("LIST".into(), self.list_field(list)?);
            }
            Stmt::InsertAtList { list, index, item } => {
                info.opcode = raw::BlockOpcode::data_insertatlist;
                info.fields.insert("LIST".into(), self.list_field(list)?);
                info.inputs
                    .insert("INDEX".into(), self.value_input(&id, index)?);
                info.inputs
                    .insert("ITEM".into(), self.value_input(&id, item)?);
            }
            Stmt::ReplaceItemOfList { list, index, item } => {
                info.opcode = raw::BlockOpcode::data_replaceitemoflist;
                info.fields.insert("LIST".into(), self.list_field(list)?);
                info.inputs
                    .insert("INDEX".into(), self.value_input(&id, index)?);
                info.inputs
                    .insert("ITEM".into(), self.value_input(&id, item)?);
            }
            Stmt::ShowList { list } => {
                info.opcode = raw::BlockOpcode::data_showlist;
                info.fields.insert("LIST".into(), self.list_field(list)?);
            }
            Stmt::HideList { list } => {
                info.opcode = raw::BlockOpcode::data_hidelist;
                info.fields.insert("LIST".into(), self.list_field(list)?);
            }
            Stmt::Wait { duration } => {
                info.opcode = raw::BlockOpcode::control_wait;
                info.inputs
                    .insert("DURATION".into(), self.value_input(&id, duration)?);
            }
            Stmt::WaitUntil { condition } => {
                info.opcode = raw::BlockOpcode::control_wait_until;
                info.inputs
                    .insert("CONDITION".into(), self.predicate_input(&id, condition)?);
            }
            Stmt::If { condition, body } => {
                info.opcode = raw::BlockOpcode::control_if;
                info.inputs
                    .insert("CONDITION".into(), self.predicate_input(&id, condition)?);
                info.inputs.insert(
                    "SUBSTACK".into(),
                    stack_input(self.emit_statement_chain(Some(id.clone()), body, None)?),
                );
            }
            Stmt::IfElse {
                condition,
                then_body,
                else_body,
            } => {
                info.opcode = raw::BlockOpcode::control_if_else;
                info.inputs
                    .insert("CONDITION".into(), self.predicate_input(&id, condition)?);
                info.inputs.insert(
                    "SUBSTACK".into(),
                    stack_input(self.emit_statement_chain(Some(id.clone()), then_body, None)?),
                );
                info.inputs.insert(
                    "SUBSTACK2".into(),
                    stack_input(self.emit_statement_chain(Some(id.clone()), else_body, None)?),
                );
            }
            Stmt::Repeat { times, body } => {
                info.opcode = raw::BlockOpcode::control_repeat;
                info.inputs
                    .insert("TIMES".into(), self.value_input(&id, times)?);
                info.inputs.insert(
                    "SUBSTACK".into(),
                    stack_input(self.emit_statement_chain(Some(id.clone()), body, None)?),
                );
            }
            Stmt::RepeatUntil { condition, body } => {
                info.opcode = raw::BlockOpcode::control_repeat_until;
                info.inputs
                    .insert("CONDITION".into(), self.predicate_input(&id, condition)?);
                info.inputs.insert(
                    "SUBSTACK".into(),
                    stack_input(self.emit_statement_chain(Some(id.clone()), body, None)?),
                );
            }
            Stmt::While { condition, body } => {
                info.opcode = raw::BlockOpcode::control_while;
                info.inputs
                    .insert("CONDITION".into(), self.predicate_input(&id, condition)?);
                info.inputs.insert(
                    "SUBSTACK".into(),
                    stack_input(self.emit_statement_chain(Some(id.clone()), body, None)?),
                );
            }
            Stmt::Forever { body } => {
                info.opcode = raw::BlockOpcode::control_forever;
                info.inputs.insert(
                    "SUBSTACK".into(),
                    stack_input(self.emit_statement_chain(Some(id.clone()), body, None)?),
                );
            }
            Stmt::Stop { option } => {
                info.opcode = raw::BlockOpcode::control_stop;
                info.fields.insert(
                    "STOP_OPTION".into(),
                    raw::Field::Value(
                        (Some(raw::VarVal::String(stop_option_name(option).into())),),
                    ),
                );
            }
            Stmt::Broadcast { broadcast } => {
                info.opcode = raw::BlockOpcode::event_broadcast;
                info.inputs
                    .insert("BROADCAST_INPUT".into(), self.broadcast_input(*broadcast)?);
            }
            Stmt::BroadcastAndWait { broadcast } => {
                info.opcode = raw::BlockOpcode::event_broadcastandwait;
                info.inputs
                    .insert("BROADCAST_INPUT".into(), self.broadcast_input(*broadcast)?);
            }
            Stmt::AskAndWait { question } => {
                info.opcode = raw::BlockOpcode::sensing_askandwait;
                info.inputs
                    .insert("QUESTION".into(), self.value_input(&id, question)?);
            }
            Stmt::ResetTimer => info.opcode = raw::BlockOpcode::sensing_resettimer,
            Stmt::Say { message } => {
                info.opcode = raw::BlockOpcode::looks_say;
                info.inputs
                    .insert("MESSAGE".into(), self.value_input(&id, message)?);
            }
            Stmt::SayForSecs { message, seconds } => {
                info.opcode = raw::BlockOpcode::looks_sayforsecs;
                info.inputs
                    .insert("MESSAGE".into(), self.value_input(&id, message)?);
                info.inputs
                    .insert("SECS".into(), self.value_input(&id, seconds)?);
            }
            Stmt::Think { message } => {
                info.opcode = raw::BlockOpcode::looks_think;
                info.inputs
                    .insert("MESSAGE".into(), self.value_input(&id, message)?);
            }
            Stmt::ThinkForSecs { message, seconds } => {
                info.opcode = raw::BlockOpcode::looks_thinkforsecs;
                info.inputs
                    .insert("MESSAGE".into(), self.value_input(&id, message)?);
                info.inputs
                    .insert("SECS".into(), self.value_input(&id, seconds)?);
            }
            Stmt::Show => info.opcode = raw::BlockOpcode::looks_show,
            Stmt::Hide => info.opcode = raw::BlockOpcode::looks_hide,
            Stmt::SwitchCostumeTo { costume } => {
                info.opcode = raw::BlockOpcode::looks_switchcostumeto;
                info.inputs
                    .insert("COSTUME".into(), self.value_input(&id, costume)?);
            }
            Stmt::SwitchBackdropTo { backdrop } => {
                info.opcode = raw::BlockOpcode::looks_switchbackdropto;
                info.inputs
                    .insert("BACKDROP".into(), self.value_input(&id, backdrop)?);
            }
            Stmt::SwitchBackdropToAndWait { backdrop } => {
                info.opcode = raw::BlockOpcode::looks_switchbackdroptoandwait;
                info.inputs
                    .insert("BACKDROP".into(), self.value_input(&id, backdrop)?);
            }
            Stmt::NextCostume => info.opcode = raw::BlockOpcode::looks_nextcostume,
            Stmt::NextBackdrop => info.opcode = raw::BlockOpcode::looks_nextbackdrop,
            Stmt::ChangeSizeBy { amount } => {
                info.opcode = raw::BlockOpcode::looks_changesizeby;
                info.inputs
                    .insert("CHANGE".into(), self.value_input(&id, amount)?);
            }
            Stmt::SetSizeTo { size } => {
                info.opcode = raw::BlockOpcode::looks_setsizeto;
                info.inputs
                    .insert("SIZE".into(), self.value_input(&id, size)?);
            }
            Stmt::MoveSteps { steps } => {
                info.opcode = raw::BlockOpcode::motion_movesteps;
                info.inputs
                    .insert("STEPS".into(), self.value_input(&id, steps)?);
            }
            Stmt::GoToXY { x, y } => {
                info.opcode = raw::BlockOpcode::motion_gotoxy;
                info.inputs.insert("X".into(), self.value_input(&id, x)?);
                info.inputs.insert("Y".into(), self.value_input(&id, y)?);
            }
            Stmt::TurnRight { degrees } => {
                info.opcode = raw::BlockOpcode::motion_turnright;
                info.inputs
                    .insert("DEGREES".into(), self.value_input(&id, degrees)?);
            }
            Stmt::TurnLeft { degrees } => {
                info.opcode = raw::BlockOpcode::motion_turnleft;
                info.inputs
                    .insert("DEGREES".into(), self.value_input(&id, degrees)?);
            }
            Stmt::PointInDirection { direction } => {
                info.opcode = raw::BlockOpcode::motion_pointindirection;
                info.inputs
                    .insert("DIRECTION".into(), self.value_input(&id, direction)?);
            }
            Stmt::ChangeXBy { amount } => {
                info.opcode = raw::BlockOpcode::motion_changexby;
                info.inputs
                    .insert("DX".into(), self.value_input(&id, amount)?);
            }
            Stmt::SetX { value } => {
                info.opcode = raw::BlockOpcode::motion_setx;
                info.inputs
                    .insert("X".into(), self.value_input(&id, value)?);
            }
            Stmt::ChangeYBy { amount } => {
                info.opcode = raw::BlockOpcode::motion_changeyby;
                info.inputs
                    .insert("DY".into(), self.value_input(&id, amount)?);
            }
            Stmt::SetY { value } => {
                info.opcode = raw::BlockOpcode::motion_sety;
                info.inputs
                    .insert("Y".into(), self.value_input(&id, value)?);
            }
            Stmt::PenSetColorToColor { value } => {
                info.opcode = raw::BlockOpcode::pen_setPenColorToColor;
                info.inputs
                    .insert("COLOR".into(), self.value_input(&id, value)?);
            }
            Stmt::PenChangeColorParamBy { param, value } => {
                info.opcode = raw::BlockOpcode::pen_changePenColorParamBy;
                info.inputs
                    .insert("COLOR_PARAM".into(), self.value_input(&id, param)?);
                info.inputs
                    .insert("VALUE".into(), self.value_input(&id, value)?);
            }
            Stmt::PenSetColorParamTo { param, value } => {
                info.opcode = raw::BlockOpcode::pen_setPenColorParamTo;
                info.inputs
                    .insert("COLOR_PARAM".into(), self.value_input(&id, param)?);
                info.inputs
                    .insert("VALUE".into(), self.value_input(&id, value)?);
            }
            Stmt::PenSetSizeTo { value } => {
                info.opcode = raw::BlockOpcode::pen_setPenSizeTo;
                info.inputs
                    .insert("SIZE".into(), self.value_input(&id, value)?);
            }
            Stmt::PenDown => {
                info.opcode = raw::BlockOpcode::pen_penDown;
            }
            Stmt::PenUp => {
                info.opcode = raw::BlockOpcode::pen_penUp;
            }
            Stmt::PenClear => {
                info.opcode = raw::BlockOpcode::pen_clear;
            }
            Stmt::CallProcedure {
                procedure,
                arguments,
            } => {
                info.opcode = raw::BlockOpcode::procedures_call;
                let argument_ids = self
                    .ensure_procedure_layout(*procedure)?
                    .argument_ids
                    .clone();
                let procedure_arguments = self.lookup_procedure(*procedure)?.arguments.clone();
                println!(
                    "{}, {}, {}, {}",
                    self.lookup_procedure(*procedure)?.proccode,
                    arguments.len(),
                    argument_ids.len(),
                    procedure_arguments.len()
                );
                for ((argument, arg_id), procedure_argument) in arguments
                    .iter()
                    .zip(argument_ids.iter())
                    .zip(procedure_arguments.iter())
                {
                    let input = match (argument, procedure_argument.kind) {
                        (
                            structured::ProcedureInput::Value(value),
                            structured::ProcedureArgumentKind::StringOrNumber,
                        ) => self.value_input(&id, value)?,
                        (structured::ProcedureInput::Predicate(predicate), _) => {
                            self.predicate_input(&id, predicate)?
                        }
                        (
                            structured::ProcedureInput::Value(_),
                            structured::ProcedureArgumentKind::Boolean,
                        ) => {
                            return Err(structured::StructuredConversionError::InvalidLiteral {
                                block_id: id.clone(),
                                context: "procedure argument kind",
                            });
                        }
                    };
                    info.inputs.insert(arg_id.clone(), input);
                }
                info.mutation = self.procedure_mutation(*procedure)?;
            }
        }

        self.blocks.insert(
            id.clone(),
            raw::Block::Normal {
                x: 0,
                y: 0,
                block_info: info,
            },
        );
        Ok(id)
    }

    fn value_input(
        &mut self,
        parent_id: &str,
        value: &Option<structured::Value>,
    ) -> Result<raw::Input, structured::StructuredConversionError> {
        Ok(raw::Input::NoShadow(
            1,
            value
                .as_ref()
                .map(|value| self.emit_value_ref(parent_id, value))
                .transpose()?,
        ))
    }

    fn predicate_input(
        &mut self,
        parent_id: &str,
        predicate: &Option<structured::Predicate>,
    ) -> Result<raw::Input, structured::StructuredConversionError> {
        Ok(raw::Input::NoShadow(
            1,
            predicate
                .as_ref()
                .map(|predicate| self.emit_predicate_ref(parent_id, predicate))
                .transpose()?,
        ))
    }

    fn emit_value_ref(
        &mut self,
        parent_id: &str,
        value: &structured::Value,
    ) -> Result<raw::BlockArrayOrId, structured::StructuredConversionError> {
        use structured::Value;

        match value {
            Value::Literal(structured::Literal::Number(number)) => Ok(raw::BlockArrayOrId::Array(
                raw::BlockArray::NumberOrAngle(4, *number),
            )),
            Value::Literal(structured::Literal::String(string)) => Ok(raw::BlockArrayOrId::Array(
                raw::BlockArray::ColorOrString(10, string.clone()),
            )),
            Value::Literal(structured::Literal::Color(color)) => Ok(raw::BlockArrayOrId::Array(
                raw::BlockArray::ColorOrString(9, color.clone()),
            )),
            Value::Predicate(predicate) => self.emit_predicate_ref(parent_id, predicate),
            Value::Variable(variable) => self.emit_field_reporter(
                parent_id,
                raw::BlockOpcode::data_variable,
                "VARIABLE",
                self.variable_field(variable)?,
            ),
            Value::ListContents(list) => self.emit_field_reporter(
                parent_id,
                raw::BlockOpcode::data_listcontents,
                "LIST",
                self.list_field(list)?,
            ),
            Value::Add(lhs, rhs) => self.emit_binary_value(
                parent_id,
                raw::BlockOpcode::operator_add,
                "NUM1",
                lhs,
                "NUM2",
                rhs,
            ),
            Value::Subtract(lhs, rhs) => self.emit_binary_value(
                parent_id,
                raw::BlockOpcode::operator_subtract,
                "NUM1",
                lhs,
                "NUM2",
                rhs,
            ),
            Value::Multiply(lhs, rhs) => self.emit_binary_value(
                parent_id,
                raw::BlockOpcode::operator_multiply,
                "NUM1",
                lhs,
                "NUM2",
                rhs,
            ),
            Value::Divide(lhs, rhs) => self.emit_binary_value(
                parent_id,
                raw::BlockOpcode::operator_divide,
                "NUM1",
                lhs,
                "NUM2",
                rhs,
            ),
            Value::Random(lhs, rhs) => self.emit_binary_value(
                parent_id,
                raw::BlockOpcode::operator_random,
                "FROM",
                lhs,
                "TO",
                rhs,
            ),
            Value::Join(lhs, rhs) => self.emit_binary_value(
                parent_id,
                raw::BlockOpcode::operator_join,
                "STRING1",
                lhs,
                "STRING2",
                rhs,
            ),
            Value::LetterOf { letter, text } => self.emit_two_input_value(
                parent_id,
                raw::BlockOpcode::operator_letter_of,
                "LETTER",
                letter,
                "STRING",
                text,
            ),
            Value::Length(value) => self.emit_unary_value(
                parent_id,
                raw::BlockOpcode::operator_length,
                "STRING",
                value,
            ),
            Value::Contains { text, search } => self.emit_two_input_value(
                parent_id,
                raw::BlockOpcode::operator_contains,
                "STRING1",
                text,
                "STRING2",
                search,
            ),
            Value::Modulo(lhs, rhs) => self.emit_binary_value(
                parent_id,
                raw::BlockOpcode::operator_mod,
                "NUM1",
                lhs,
                "NUM2",
                rhs,
            ),
            Value::Round(value) => {
                self.emit_unary_value(parent_id, raw::BlockOpcode::operator_round, "NUM", value)
            }
            Value::MathOp { operator, operand } => {
                let id = self.fresh_id("value");
                let mut fields = BTreeMap::new();
                fields.insert(
                    "OPERATOR".into(),
                    raw::Field::Value((Some(raw::VarVal::String(
                        math_operator_name(operator).into(),
                    )),)),
                );
                let mut inputs = BTreeMap::new();
                inputs.insert(
                    "NUM".into(),
                    self.value_input(&id, &operand.as_ref().map(|value| *value.clone()))?,
                );
                self.insert_reporter(
                    id.clone(),
                    parent_id,
                    raw::BlockOpcode::operator_mathop,
                    inputs,
                    fields,
                );
                Ok(raw::BlockArrayOrId::Id(id))
            }
            Value::ItemOfList { list, index } => self.emit_list_reporter(
                parent_id,
                raw::BlockOpcode::data_itemoflist,
                list,
                "INDEX",
                index,
            ),
            Value::ItemNumOfList { list, item } => self.emit_list_reporter(
                parent_id,
                raw::BlockOpcode::data_itemnumoflist,
                list,
                "ITEM",
                item,
            ),
            Value::LengthOfList(list) => self.emit_field_reporter(
                parent_id,
                raw::BlockOpcode::data_lengthoflist,
                "LIST",
                self.list_field(list)?,
            ),
            Value::Answer => {
                self.emit_zero_input_reporter(parent_id, raw::BlockOpcode::sensing_answer)
            }
            Value::MouseX => {
                self.emit_zero_input_reporter(parent_id, raw::BlockOpcode::sensing_mousex)
            }
            Value::MouseY => {
                self.emit_zero_input_reporter(parent_id, raw::BlockOpcode::sensing_mousey)
            }
            Value::Timer => {
                self.emit_zero_input_reporter(parent_id, raw::BlockOpcode::sensing_timer)
            }
            Value::DaysSince2000 => {
                self.emit_zero_input_reporter(parent_id, raw::BlockOpcode::sensing_dayssince2000)
            }
            Value::KeyOptions(key_option) => {
                let id = self.fresh_id("value");
                let mut fields = BTreeMap::new();
                fields.insert(
                    "KEY_OPTION".into(),
                    raw::Field::Value((Some(raw::VarVal::String(key_option.clone())),)),
                );
                self.insert_reporter(
                    id.clone(),
                    parent_id,
                    raw::BlockOpcode::sensing_keyoptions,
                    BTreeMap::new(),
                    fields,
                );
                Ok(raw::BlockArrayOrId::Id(id))
            }
            Value::XPosition => {
                self.emit_zero_input_reporter(parent_id, raw::BlockOpcode::motion_xposition)
            }
            Value::YPosition => {
                self.emit_zero_input_reporter(parent_id, raw::BlockOpcode::motion_yposition)
            }
            Value::Direction => {
                self.emit_zero_input_reporter(parent_id, raw::BlockOpcode::motion_direction)
            }
            Value::Size => self.emit_zero_input_reporter(parent_id, raw::BlockOpcode::looks_size),
            Value::CostumeNumber => {
                self.emit_number_name_reporter(parent_id, raw::BlockOpcode::looks_costumenumbername)
            }
            Value::BackdropNumber => self
                .emit_number_name_reporter(parent_id, raw::BlockOpcode::looks_backdropnumbername),
            Value::Volume => {
                self.emit_zero_input_reporter(parent_id, raw::BlockOpcode::sound_volume)
            }
            Value::ProcedureArgument(argument) => self.emit_procedure_argument_reporter(
                parent_id,
                *argument,
                raw::BlockOpcode::argument_reporter_string_number,
            ),
            Value::PenMenuColorParam(color_param) => {
                let id = self.fresh_id("value");
                let mut fields = BTreeMap::new();
                fields.insert(
                    "colorParam".into(),
                    raw::Field::Value((Some(raw::VarVal::String(color_param.clone())),)),
                );
                self.insert_reporter(
                    id.clone(),
                    parent_id,
                    raw::BlockOpcode::pen_menu_colorParam,
                    BTreeMap::new(),
                    fields,
                );
                Ok(raw::BlockArrayOrId::Id(id))
            }
        }
    }

    fn emit_predicate_ref(
        &mut self,
        parent_id: &str,
        predicate: &structured::Predicate,
    ) -> Result<raw::BlockArrayOrId, structured::StructuredConversionError> {
        use structured::Predicate;

        match predicate {
            Predicate::LessThan(lhs, rhs) => self.emit_binary_value(
                parent_id,
                raw::BlockOpcode::operator_lt,
                "OPERAND1",
                lhs,
                "OPERAND2",
                rhs,
            ),
            Predicate::Equals(lhs, rhs) => self.emit_binary_value(
                parent_id,
                raw::BlockOpcode::operator_equals,
                "OPERAND1",
                lhs,
                "OPERAND2",
                rhs,
            ),
            Predicate::GreaterThan(lhs, rhs) => self.emit_binary_value(
                parent_id,
                raw::BlockOpcode::operator_gt,
                "OPERAND1",
                lhs,
                "OPERAND2",
                rhs,
            ),
            Predicate::And(lhs, rhs) => self.emit_binary_predicate(
                parent_id,
                raw::BlockOpcode::operator_and,
                "OPERAND1",
                lhs,
                "OPERAND2",
                rhs,
            ),
            Predicate::Or(lhs, rhs) => self.emit_binary_predicate(
                parent_id,
                raw::BlockOpcode::operator_or,
                "OPERAND1",
                lhs,
                "OPERAND2",
                rhs,
            ),
            Predicate::Not(value) => self.emit_unary_predicate(
                parent_id,
                raw::BlockOpcode::operator_not,
                "OPERAND",
                value,
            ),
            Predicate::MouseDown => {
                self.emit_zero_input_reporter(parent_id, raw::BlockOpcode::sensing_mousedown)
            }
            Predicate::ListContainsItem { list, item } => self.emit_list_reporter(
                parent_id,
                raw::BlockOpcode::data_listcontainsitem,
                list,
                "ITEM",
                item,
            ),
            Predicate::ItemOfList { list, index } => self.emit_list_reporter(
                parent_id,
                raw::BlockOpcode::data_itemoflist,
                list,
                "INDEX",
                index,
            ),
            Predicate::ItemNumOfList { list, item } => self.emit_list_reporter(
                parent_id,
                raw::BlockOpcode::data_itemnumoflist,
                list,
                "ITEM",
                item,
            ),
            Predicate::ProcedureArgument(argument) => self.emit_procedure_argument_reporter(
                parent_id,
                *argument,
                raw::BlockOpcode::argument_reporter_boolean,
            ),
            Predicate::KeyPressed(key_option) => self.emit_unary_value(
                parent_id,
                raw::BlockOpcode::sensing_keypressed,
                "KEY_OPTION",
                key_option,
            ),
        }
    }

    fn emit_zero_input_reporter(
        &mut self,
        parent_id: &str,
        opcode: raw::BlockOpcode,
    ) -> Result<raw::BlockArrayOrId, structured::StructuredConversionError> {
        let id = self.fresh_id("value");
        self.insert_reporter(
            id.clone(),
            parent_id,
            opcode,
            BTreeMap::new(),
            BTreeMap::new(),
        );
        Ok(raw::BlockArrayOrId::Id(id))
    }

    fn emit_number_name_reporter(
        &mut self,
        parent_id: &str,
        opcode: raw::BlockOpcode,
    ) -> Result<raw::BlockArrayOrId, structured::StructuredConversionError> {
        let id = self.fresh_id("value");
        let mut fields = BTreeMap::new();
        fields.insert(
            "NUMBER_NAME".into(),
            raw::Field::Value((Some(raw::VarVal::String("number".into())),)),
        );
        self.insert_reporter(id.clone(), parent_id, opcode, BTreeMap::new(), fields);
        Ok(raw::BlockArrayOrId::Id(id))
    }

    fn emit_field_reporter(
        &mut self,
        parent_id: &str,
        opcode: raw::BlockOpcode,
        field_name: &'static str,
        field: raw::Field,
    ) -> Result<raw::BlockArrayOrId, structured::StructuredConversionError> {
        let id = self.fresh_id("value");
        let mut fields = BTreeMap::new();
        fields.insert(field_name.into(), field);
        self.insert_reporter(id.clone(), parent_id, opcode, BTreeMap::new(), fields);
        Ok(raw::BlockArrayOrId::Id(id))
    }

    fn emit_unary_value(
        &mut self,
        parent_id: &str,
        opcode: raw::BlockOpcode,
        input_name: &'static str,
        value: &Option<Box<structured::Value>>,
    ) -> Result<raw::BlockArrayOrId, structured::StructuredConversionError> {
        let id = self.fresh_id("value");
        let mut inputs = BTreeMap::new();
        inputs.insert(
            input_name.into(),
            self.value_input(&id, &value.as_ref().map(|value| *value.clone()))?,
        );
        self.insert_reporter(id.clone(), parent_id, opcode, inputs, BTreeMap::new());
        Ok(raw::BlockArrayOrId::Id(id))
    }

    fn emit_two_input_value(
        &mut self,
        parent_id: &str,
        opcode: raw::BlockOpcode,
        left_name: &'static str,
        left: &Option<Box<structured::Value>>,
        right_name: &'static str,
        right: &Option<Box<structured::Value>>,
    ) -> Result<raw::BlockArrayOrId, structured::StructuredConversionError> {
        self.emit_binary_value(parent_id, opcode, left_name, left, right_name, right)
    }

    fn emit_binary_value(
        &mut self,
        parent_id: &str,
        opcode: raw::BlockOpcode,
        left_name: &'static str,
        left: &Option<Box<structured::Value>>,
        right_name: &'static str,
        right: &Option<Box<structured::Value>>,
    ) -> Result<raw::BlockArrayOrId, structured::StructuredConversionError> {
        let id = self.fresh_id("value");
        let mut inputs = BTreeMap::new();
        inputs.insert(
            left_name.into(),
            self.value_input(&id, &left.as_ref().map(|value| *value.clone()))?,
        );
        inputs.insert(
            right_name.into(),
            self.value_input(&id, &right.as_ref().map(|value| *value.clone()))?,
        );
        self.insert_reporter(id.clone(), parent_id, opcode, inputs, BTreeMap::new());
        Ok(raw::BlockArrayOrId::Id(id))
    }

    fn emit_binary_predicate(
        &mut self,
        parent_id: &str,
        opcode: raw::BlockOpcode,
        left_name: &'static str,
        left: &Option<Box<structured::Predicate>>,
        right_name: &'static str,
        right: &Option<Box<structured::Predicate>>,
    ) -> Result<raw::BlockArrayOrId, structured::StructuredConversionError> {
        let id = self.fresh_id("pred");
        let mut inputs = BTreeMap::new();
        inputs.insert(
            left_name.into(),
            self.predicate_input(&id, &left.as_ref().map(|value| *value.clone()))?,
        );
        inputs.insert(
            right_name.into(),
            self.predicate_input(&id, &right.as_ref().map(|value| *value.clone()))?,
        );
        self.insert_reporter(id.clone(), parent_id, opcode, inputs, BTreeMap::new());
        Ok(raw::BlockArrayOrId::Id(id))
    }

    fn emit_unary_predicate(
        &mut self,
        parent_id: &str,
        opcode: raw::BlockOpcode,
        input_name: &'static str,
        value: &Option<Box<structured::Predicate>>,
    ) -> Result<raw::BlockArrayOrId, structured::StructuredConversionError> {
        let id = self.fresh_id("pred");
        let mut inputs = BTreeMap::new();
        inputs.insert(
            input_name.into(),
            self.predicate_input(&id, &value.as_ref().map(|value| *value.clone()))?,
        );
        self.insert_reporter(id.clone(), parent_id, opcode, inputs, BTreeMap::new());
        Ok(raw::BlockArrayOrId::Id(id))
    }

    fn emit_list_reporter<T>(
        &mut self,
        parent_id: &str,
        opcode: raw::BlockOpcode,
        list: &structured::ListRef,
        input_name: &'static str,
        value: &Option<Box<T>>,
    ) -> Result<raw::BlockArrayOrId, structured::StructuredConversionError>
    where
        T: Clone,
        Self: ListInputEmitter<T>,
    {
        let id = self.fresh_id("value");
        let mut fields = BTreeMap::new();
        fields.insert("LIST".into(), self.list_field(list)?);
        let mut inputs = BTreeMap::new();
        inputs.insert(
            input_name.into(),
            <Self as ListInputEmitter<T>>::emit_input(self, &id, value)?,
        );
        self.insert_reporter(id.clone(), parent_id, opcode, inputs, fields);
        Ok(raw::BlockArrayOrId::Id(id))
    }

    fn emit_procedure_argument_reporter(
        &mut self,
        parent_id: &str,
        argument: structured::ProcedureArgumentRef,
        opcode: raw::BlockOpcode,
    ) -> Result<raw::BlockArrayOrId, structured::StructuredConversionError> {
        let procedure = self.lookup_procedure(argument.procedure)?.clone();
        let Some(argument_info) = procedure.arguments.get(argument.argument_index) else {
            return Err(
                structured::StructuredConversionError::UnknownProcedureArgument {
                    block_id: parent_id.into(),
                    procedure: argument.procedure,
                    argument_name: argument.argument_index.to_string().into(),
                },
            );
        };
        let argument_name = argument_info.name.clone();
        let id = self.fresh_id("value");
        let mut fields = BTreeMap::new();
        fields.insert(
            "VALUE".into(),
            raw::Field::Value((Some(raw::VarVal::String(argument_name)),)),
        );
        self.insert_reporter(id.clone(), parent_id, opcode, BTreeMap::new(), fields);
        Ok(raw::BlockArrayOrId::Id(id))
    }

    fn insert_reporter(
        &mut self,
        id: Box<str>,
        parent_id: &str,
        opcode: raw::BlockOpcode,
        inputs: BTreeMap<Box<str>, raw::Input>,
        fields: BTreeMap<Box<str>, raw::Field>,
    ) {
        self.blocks.insert(
            id,
            raw::Block::Normal {
                x: 0,
                y: 0,
                block_info: raw::BlockInfo {
                    opcode,
                    next: None,
                    parent: Some(parent_id.into()),
                    inputs,
                    fields,
                    shadow: false,
                    top_level: false,
                    mutation: raw::Mutation::default(),
                },
            },
        );
    }

    fn variable_field(
        &self,
        variable: &structured::VariableRef,
    ) -> Result<raw::Field, structured::StructuredConversionError> {
        let variable = self.lookup_variable(*variable)?;
        Ok(raw::Field::ValueId(
            Some(raw::VarVal::String(variable_name(&variable.info).into())),
            Some(variable.scratch_id.clone()),
        ))
    }

    fn list_field(
        &self,
        list: &structured::ListRef,
    ) -> Result<raw::Field, structured::StructuredConversionError> {
        let list = self.lookup_list(*list)?;
        Ok(raw::Field::ValueId(
            Some(raw::VarVal::String(list.name.clone())),
            Some(list.scratch_id.clone()),
        ))
    }

    fn broadcast_field(
        &self,
        broadcast: structured::BroadcastId,
    ) -> Result<raw::Field, structured::StructuredConversionError> {
        let broadcast = self.lookup_broadcast(broadcast)?;
        Ok(raw::Field::ValueId(
            Some(raw::VarVal::String(broadcast.name.clone())),
            Some(broadcast.scratch_id.clone()),
        ))
    }

    fn broadcast_input(
        &self,
        broadcast: structured::BroadcastId,
    ) -> Result<raw::Input, structured::StructuredConversionError> {
        let broadcast = self.lookup_broadcast(broadcast)?;
        Ok(raw::Input::NoShadow(
            1,
            Some(raw::BlockArrayOrId::Array(raw::BlockArray::Broadcast(
                11,
                broadcast.name.clone(),
                broadcast.scratch_id.clone(),
            ))),
        ))
    }

    fn ensure_procedure_layout(
        &mut self,
        procedure: structured::ProcedureId,
    ) -> Result<&EmittedProcedureLayout, structured::StructuredConversionError> {
        if !self.procedure_layouts.contains_key(&procedure) {
            let procedure_info = self.lookup_procedure(procedure)?;
            let argument_ids = (0..procedure_info.arguments.len())
                .map(|index| format!("procarg_{}_{}", procedure.0, index).into_boxed_str())
                .collect();
            self.procedure_layouts
                .insert(procedure, EmittedProcedureLayout { argument_ids });
        }
        Ok(self
            .procedure_layouts
            .get(&procedure)
            .expect("inserted above"))
    }

    fn procedure_mutation(
        &mut self,
        procedure: structured::ProcedureId,
    ) -> Result<raw::Mutation, structured::StructuredConversionError> {
        let procedure_info = self.lookup_procedure(procedure)?.clone();
        let argument_ids = self
            .ensure_procedure_layout(procedure)?
            .argument_ids
            .clone();
        let argument_names = procedure_info
            .arguments
            .iter()
            .map(|argument| argument.name.as_ref())
            .collect::<Vec<_>>();
        let argument_defaults = procedure_info
            .arguments
            .iter()
            .map(|argument| match &argument.default {
                structured::ProcedureArgumentDefault::String(value) => {
                    serde_json::Value::String(value.to_string())
                }
                structured::ProcedureArgumentDefault::Boolean(value) => {
                    serde_json::Value::Bool(*value)
                }
            })
            .collect::<Vec<_>>();
        let mut mutations = BTreeMap::new();
        mutations.insert(
            "proccode".into(),
            serde_json::Value::String(procedure_info.proccode.to_string()),
        );
        mutations.insert(
            "argumentids".into(),
            serde_json::Value::String(
                serde_json::to_string(&argument_ids)
                    .expect("serializing procedure argument ids should not fail"),
            ),
        );
        mutations.insert(
            "argumentnames".into(),
            serde_json::Value::String(
                serde_json::to_string(&argument_names)
                    .expect("serializing procedure argument names should not fail"),
            ),
        );
        mutations.insert(
            "argumentdefaults".into(),
            serde_json::Value::String(
                serde_json::to_string(&argument_defaults)
                    .expect("serializing procedure argument defaults should not fail"),
            ),
        );
        mutations.insert(
            "warp".into(),
            serde_json::Value::String(if procedure_info.warp { "true" } else { "false" }.into()),
        );
        Ok(raw::Mutation {
            tag_name: "mutation".into(),
            children: vec![],
            mutations,
        })
    }

    fn emit_procedure_prototype(
        &mut self,
        parent_id: &str,
        procedure: structured::ProcedureId,
    ) -> Result<Box<str>, structured::StructuredConversionError> {
        let procedure_info = self.lookup_procedure(procedure)?.clone();
        let argument_ids = self
            .ensure_procedure_layout(procedure)?
            .argument_ids
            .clone();
        let prototype_id = self.fresh_id("prototype");
        let mutation = self.procedure_mutation(procedure)?;
        let mut inputs = BTreeMap::new();
        for (index, argument) in procedure_info.arguments.iter().enumerate() {
            let reporter_id = self.fresh_id("arg");
            let opcode = match argument.kind {
                structured::ProcedureArgumentKind::StringOrNumber => {
                    raw::BlockOpcode::argument_reporter_string_number
                }
                structured::ProcedureArgumentKind::Boolean => {
                    raw::BlockOpcode::argument_reporter_boolean
                }
            };
            let mut fields = BTreeMap::new();
            fields.insert(
                "VALUE".into(),
                raw::Field::Value((Some(raw::VarVal::String(argument.name.clone())),)),
            );
            self.blocks.insert(
                reporter_id.clone(),
                raw::Block::Normal {
                    x: 0,
                    y: 0,
                    block_info: raw::BlockInfo {
                        opcode,
                        next: None,
                        parent: Some(prototype_id.clone()),
                        inputs: BTreeMap::new(),
                        fields,
                        shadow: true,
                        top_level: false,
                        mutation: raw::Mutation::default(),
                    },
                },
            );
            inputs.insert(
                argument_ids[index].clone(),
                raw::Input::NoShadow(1, Some(raw::BlockArrayOrId::Id(reporter_id))),
            );
        }
        self.blocks.insert(
            prototype_id.clone(),
            raw::Block::Normal {
                x: 0,
                y: 0,
                block_info: raw::BlockInfo {
                    opcode: raw::BlockOpcode::procedures_prototype,
                    next: None,
                    parent: Some(parent_id.into()),
                    inputs,
                    fields: BTreeMap::new(),
                    shadow: true,
                    top_level: false,
                    mutation,
                },
            },
        );
        Ok(prototype_id)
    }

    fn lookup_variable(
        &self,
        variable: structured::VariableRef,
    ) -> Result<&structured::Variable, structured::StructuredConversionError> {
        match variable {
            structured::VariableRef::Global(id) => self.project.global_variables.get(id),
            structured::VariableRef::Local(id) => self.target.local_variables.get(id),
        }
        .ok_or_else(|| structured::StructuredConversionError::UnknownVariable {
            block_id: "emit".into(),
            variable_id: format!("{variable:?}").into(),
        })
    }

    fn lookup_list(
        &self,
        list: structured::ListRef,
    ) -> Result<&structured::List, structured::StructuredConversionError> {
        match list {
            structured::ListRef::Global(id) => self.project.global_lists.get(id),
            structured::ListRef::Local(id) => self.target.local_lists.get(id),
        }
        .ok_or_else(|| structured::StructuredConversionError::UnknownList {
            block_id: "emit".into(),
            list_id: format!("{list:?}").into(),
        })
    }

    fn lookup_broadcast(
        &self,
        id: structured::BroadcastId,
    ) -> Result<&structured::Broadcast, structured::StructuredConversionError> {
        self.project.broadcasts.get(id).ok_or_else(|| {
            structured::StructuredConversionError::UnknownBroadcast {
                block_id: "emit".into(),
                broadcast_id: format!("{id:?}").into(),
            }
        })
    }

    fn lookup_procedure(
        &self,
        id: structured::ProcedureId,
    ) -> Result<&structured::Procedure, structured::StructuredConversionError> {
        self.target.local_procedures.get(id).ok_or_else(|| {
            structured::StructuredConversionError::UnknownProcedure {
                block_id: "emit".into(),
                proccode: format!("{id:?}").into(),
            }
        })
    }

    fn make_top_level(&mut self, id: &str, position: structured::ScriptPosition) {
        let Some(raw::Block::Normal { x, y, block_info }) = self.blocks.get_mut(id) else {
            return;
        };
        *x = position.x;
        *y = position.y;
        block_info.top_level = true;
        block_info.parent = None;
    }

    fn set_next(&mut self, id: &str, next: Box<str>) {
        let Some(raw::Block::Normal { block_info, .. }) = self.blocks.get_mut(id) else {
            return;
        };
        block_info.next = Some(next);
    }

    fn fresh_id(&mut self, prefix: &str) -> Box<str> {
        let id = format!("{prefix}_{}", self.next_id);
        self.next_id += 1;
        id.into_boxed_str()
    }
}

trait ListInputEmitter<T> {
    fn emit_input(
        builder: &mut RawTargetBuilder<'_>,
        parent_id: &str,
        value: &Option<Box<T>>,
    ) -> Result<raw::Input, structured::StructuredConversionError>;
}

impl ListInputEmitter<structured::Value> for RawTargetBuilder<'_> {
    fn emit_input(
        builder: &mut RawTargetBuilder<'_>,
        parent_id: &str,
        value: &Option<Box<structured::Value>>,
    ) -> Result<raw::Input, structured::StructuredConversionError> {
        builder.value_input(parent_id, &value.as_ref().map(|value| *value.clone()))
    }
}

impl ListInputEmitter<structured::Predicate> for RawTargetBuilder<'_> {
    fn emit_input(
        builder: &mut RawTargetBuilder<'_>,
        parent_id: &str,
        value: &Option<Box<structured::Predicate>>,
    ) -> Result<raw::Input, structured::StructuredConversionError> {
        builder.predicate_input(parent_id, &value.as_ref().map(|value| *value.clone()))
    }
}

fn stack_input(first_block: Option<Box<str>>) -> raw::Input {
    raw::Input::NoShadow(2, first_block.map(raw::BlockArrayOrId::Id))
}

fn variable_name(info: &raw::VariableInfo) -> &str {
    match info {
        raw::VariableInfo::CloudVar(name, ..) | raw::VariableInfo::LocalVar(name, ..) => name,
    }
}

fn stop_option_name(option: &structured::StopOption) -> &'static str {
    match option {
        structured::StopOption::All => "all",
        structured::StopOption::ThisScript => "this script",
        structured::StopOption::OtherScriptsInSprite => "other scripts in sprite",
    }
}

fn math_operator_name(operator: &structured::MathOperator) -> &'static str {
    match operator {
        structured::MathOperator::Abs => "abs",
        structured::MathOperator::Floor => "floor",
        structured::MathOperator::Ceiling => "ceiling",
        structured::MathOperator::Sqrt => "sqrt",
        structured::MathOperator::Sin => "sin",
        structured::MathOperator::Cos => "cos",
        structured::MathOperator::Tan => "tan",
        structured::MathOperator::Asin => "asin",
        structured::MathOperator::Acos => "acos",
        structured::MathOperator::Atan => "atan",
        structured::MathOperator::Ln => "ln",
        structured::MathOperator::Log => "log",
        structured::MathOperator::Exp => "e ^",
        structured::MathOperator::Pow10 => "10 ^",
    }
}

#[cfg(test)]
mod tests {
    use pretty_assertions::assert_eq;

    use super::*;

    #[test]
    fn structured_round_trip_through_raw() {
        let mut global_variables = structured::Registry::new();
        let global_var = global_variables.push(structured::Variable {
            scratch_id: "var-id".into(),
            info: raw::VariableInfo::LocalVar("score".into(), raw::VarVal::Int(0)),
        });
        let mut global_lists = structured::Registry::new();
        let global_list = global_lists.push(structured::List {
            scratch_id: "list-id".into(),
            name: "items".into(),
            value: Vec::new(),
        });
        let mut broadcasts = structured::Registry::new();
        let broadcast = broadcasts.push(structured::Broadcast {
            scratch_id: "broadcast-id".into(),
            name: "go".into(),
        });

        let project = structured::StructuredProject {
            global_variables,
            global_lists,
            broadcasts,
            targets: vec![structured::StructuredTarget {
                is_stage: true,
                name: "Stage".into(),
                local_variables: structured::Registry::new(),
                local_lists: structured::Registry::new(),
                local_procedures: structured::Registry::new(),
                scripts: vec![
                    structured::Script {
                        hat: Some(structured::Hat::WhenFlagClicked),
                        position: Some(structured::ScriptPosition { x: 12, y: 34 }),
                        body: vec![
                            structured::Statement::SetVariable {
                                variable: structured::VariableRef::Global(global_var),
                                value: Some(structured::Value::Add(
                                    Some(Box::new(structured::Value::Literal(
                                        structured::Literal::Number(1.0),
                                    ))),
                                    Some(Box::new(structured::Value::Round(Some(Box::new(
                                        structured::Value::Literal(structured::Literal::Number(
                                            2.6,
                                        )),
                                    ))))),
                                )),
                            },
                            structured::Statement::IfElse {
                                condition: Some(structured::Predicate::GreaterThan(
                                    Some(Box::new(structured::Value::Variable(
                                        structured::VariableRef::Global(global_var),
                                    ))),
                                    Some(Box::new(structured::Value::Literal(
                                        structured::Literal::Number(0.0),
                                    ))),
                                )),
                                then_body: vec![structured::Statement::BroadcastAndWait {
                                    broadcast,
                                }],
                                else_body: vec![structured::Statement::WaitUntil {
                                    condition: Some(structured::Predicate::ListContainsItem {
                                        list: structured::ListRef::Global(global_list),
                                        item: Some(Box::new(structured::Value::Literal(
                                            structured::Literal::String("x".into()),
                                        ))),
                                    }),
                                }],
                            },
                        ],
                    },
                    structured::Script {
                        hat: None,
                        position: Some(structured::ScriptPosition { x: 50, y: 60 }),
                        body: vec![
                            structured::Statement::MoveSteps {
                                steps: Some(structured::Value::Literal(
                                    structured::Literal::Number(10.0),
                                )),
                            },
                            structured::Statement::AddToList {
                                list: structured::ListRef::Global(global_list),
                                item: Some(structured::Value::Predicate(Box::new(
                                    structured::Predicate::MouseDown,
                                ))),
                            },
                        ],
                    },
                ],
                comments: Vec::new(),
                current_costume: 0,
                costumes: Vec::new(),
                sounds: Vec::new(),
                layer_order: 0,
                volume: 100.0,
                tempo: 60.0,
                video_state: None,
                video_transparency: 50.0,
                text_to_speech_language: None,
                visible: true,
                x: 0.0,
                y: 0.0,
                size: 100.0,
                direction: 90.0,
                draggable: false,
                rotation_style: "all around".into(),
            }],
            monitors: Vec::new(),
            extensions: Vec::new(),
            meta: raw::Meta {
                semver: "3.0.0".into(),
                vm: "vm".into(),
                agent: "tests".into(),
            },
        };

        let raw = raw::Sb3Project::try_from(project.clone()).expect("structured -> raw");
        let reparsed = structured::StructuredProject::try_from(raw).expect("raw -> structured");
        assert_eq!(reparsed, project);
    }

    #[test]
    fn procedures_round_trip_through_raw() {
        let mut local_procedures = structured::Registry::new();
        let procedure = local_procedures.push(structured::Procedure {
            proccode: "demo %s %b".into(),
            arguments: vec![
                structured::ProcedureArgument {
                    name: "text".into(),
                    kind: structured::ProcedureArgumentKind::StringOrNumber,
                    default: structured::ProcedureArgumentDefault::String("".into()),
                },
                structured::ProcedureArgument {
                    name: "flag".into(),
                    kind: structured::ProcedureArgumentKind::Boolean,
                    default: structured::ProcedureArgumentDefault::Boolean(false),
                },
            ],
            warp: true,
        });

        let project = structured::StructuredProject {
            global_variables: structured::Registry::new(),
            global_lists: structured::Registry::new(),
            broadcasts: structured::Registry::new(),
            targets: vec![
                structured::StructuredTarget {
                    is_stage: true,
                    name: "Stage".into(),
                    local_variables: structured::Registry::new(),
                    local_lists: structured::Registry::new(),
                    local_procedures: structured::Registry::new(),
                    scripts: Vec::new(),
                    comments: Vec::new(),
                    current_costume: 0,
                    costumes: Vec::new(),
                    sounds: Vec::new(),
                    layer_order: 0,
                    volume: 100.0,
                    tempo: 60.0,
                    video_state: None,
                    video_transparency: 50.0,
                    text_to_speech_language: None,
                    visible: true,
                    x: 0.0,
                    y: 0.0,
                    size: 100.0,
                    direction: 90.0,
                    draggable: false,
                    rotation_style: "all around".into(),
                },
                structured::StructuredTarget {
                    is_stage: false,
                    name: "Sprite1".into(),
                    local_variables: structured::Registry::new(),
                    local_lists: structured::Registry::new(),
                    local_procedures,
                    scripts: vec![
                        structured::Script {
                            hat: Some(structured::Hat::ProcedureDefinition { procedure }),
                            position: Some(structured::ScriptPosition { x: 10, y: 20 }),
                            body: vec![
                                structured::Statement::Say {
                                    message: Some(structured::Value::ProcedureArgument(
                                        structured::ProcedureArgumentRef {
                                            procedure,
                                            argument_index: 0,
                                        },
                                    )),
                                },
                                structured::Statement::WaitUntil {
                                    condition: Some(structured::Predicate::ProcedureArgument(
                                        structured::ProcedureArgumentRef {
                                            procedure,
                                            argument_index: 1,
                                        },
                                    )),
                                },
                            ],
                        },
                        structured::Script {
                            hat: Some(structured::Hat::WhenFlagClicked),
                            position: Some(structured::ScriptPosition { x: 40, y: 60 }),
                            body: vec![structured::Statement::CallProcedure {
                                procedure,
                                arguments: vec![
                                    structured::ProcedureInput::Value(Some(
                                        structured::Value::Literal(structured::Literal::String(
                                            "hello".into(),
                                        )),
                                    )),
                                    structured::ProcedureInput::Predicate(Some(
                                        structured::Predicate::MouseDown,
                                    )),
                                ],
                            }],
                            top_reporter: None,
                        },
                    ],
                    comments: Vec::new(),
                    current_costume: 0,
                    costumes: Vec::new(),
                    sounds: Vec::new(),
                    layer_order: 1,
                    volume: 100.0,
                    tempo: 60.0,
                    video_state: None,
                    video_transparency: 50.0,
                    text_to_speech_language: None,
                    visible: true,
                    x: 0.0,
                    y: 0.0,
                    size: 100.0,
                    direction: 90.0,
                    draggable: false,
                    rotation_style: "all around".into(),
                },
            ],
            monitors: Vec::new(),
            extensions: Vec::new(),
            meta: raw::Meta {
                semver: "3.0.0".into(),
                vm: "vm".into(),
                agent: "tests".into(),
            },
        };

        let raw = raw::Sb3Project::try_from(project.clone()).expect("structured -> raw");
        let reparsed = structured::StructuredProject::try_from(raw).expect("raw -> structured");
        assert_eq!(reparsed, project);
    }

    #[test]
    fn unsupported_top_level_opcode_errors() {
        let raw = raw::Sb3Project {
            targets: vec![raw::Target {
                is_stage: true,
                name: "Stage".into(),
                variables: BTreeMap::new(),
                lists: BTreeMap::new(),
                broadcasts: BTreeMap::new(),
                blocks: BTreeMap::from([(
                    "top".into(),
                    raw::Block::Normal {
                        x: 0,
                        y: 0,
                        block_info: raw::BlockInfo {
                            opcode: raw::BlockOpcode::sound_play,
                            next: None,
                            parent: None,
                            inputs: BTreeMap::new(),
                            fields: BTreeMap::new(),
                            shadow: false,
                            top_level: true,
                            mutation: raw::Mutation::default(),
                        },
                    },
                )]),
                comments: BTreeMap::new(),
                current_costume: 0,
                costumes: Vec::new(),
                sounds: Vec::new(),
                layer_order: 0,
                volume: 100.0,
                tempo: 60.0,
                video_state: None,
                video_transparency: 50.0,
                text_to_speech_language: None,
                visible: true,
                x: 0.0,
                y: 0.0,
                size: 100.0,
                direction: 90.0,
                draggable: false,
                rotation_style: "all around".into(),
            }],
            monitors: Vec::new(),
            extensions: Vec::new(),
            meta: raw::Meta {
                semver: "3.0.0".into(),
                vm: "vm".into(),
                agent: "tests".into(),
            },
        };

        let err = structured::StructuredProject::try_from(raw).expect_err("expected error");
        assert!(matches!(
            err,
            structured::StructuredConversionError::UnexpectedTopLevelBlock { .. }
                | structured::StructuredConversionError::UnsupportedOpcode { .. }
        ));
    }
}
