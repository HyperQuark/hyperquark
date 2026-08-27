mod arbitrary;
mod mutate;

use arbitrary::{Arbitrary, ArbitraryWithContext, TargetContext, Unstructured, arbitrary_context};
#[cfg(feature = "fuzz")]
use libafl::inputs::{HasTargetBytes, Input};
#[cfg(feature = "fuzz")]
use libafl_bolts::HasLen;
use mutatis::Mutate;
use serde::{Deserialize, Serialize};

#[cfg(feature = "fuzz")]
use crate::raw::Sb3Project;
use crate::raw::{Comment, Costume, Meta, Monitor, Sound, VarVal, VariableInfo};

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum StructuredConversionError {
    UnsupportedOpcode {
        opcode: crate::raw::BlockOpcode,
        block_id: Box<str>,
    },
    MissingBlock {
        block_id: Box<str>,
    },
    ExpectedNormalBlock {
        block_id: Box<str>,
    },
    MissingInput {
        block_id: Box<str>,
        input: &'static str,
    },
    MissingField {
        block_id: Box<str>,
        field: &'static str,
    },
    MissingFieldValue {
        block_id: Box<str>,
        field: &'static str,
    },
    InvalidBroadcastInput {
        block_id: Box<str>,
        input: &'static str,
    },
    InvalidVariableField {
        block_id: Box<str>,
        field: &'static str,
    },
    InvalidLiteral {
        block_id: Box<str>,
        context: &'static str,
    },
    UnexpectedTopLevelBlock {
        block_id: Box<str>,
        opcode: crate::raw::BlockOpcode,
    },
    MissingStage,
    DuplicateStage,
    UnknownVariable {
        block_id: Box<str>,
        variable_id: Box<str>,
    },
    UnknownList {
        block_id: Box<str>,
        list_id: Box<str>,
    },
    UnknownBroadcast {
        block_id: Box<str>,
        broadcast_id: Box<str>,
    },
    MissingMutation {
        block_id: Box<str>,
        property: &'static str,
    },
    InvalidMutation {
        block_id: Box<str>,
        property: &'static str,
    },
    UnknownProcedure {
        block_id: Box<str>,
        proccode: Box<str>,
    },
    UnknownProcedureArgument {
        block_id: Box<str>,
        procedure: ProcedureId,
        argument_name: Box<str>,
    },
}

impl std::fmt::Display for StructuredConversionError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::UnsupportedOpcode { opcode, block_id } => {
                write!(f, "unsupported opcode {opcode:?} at block {block_id}")
            }
            Self::MissingBlock { block_id } => write!(f, "missing block {block_id}"),
            Self::ExpectedNormalBlock { block_id } => {
                write!(f, "expected normal block object for {block_id}")
            }
            Self::MissingInput { block_id, input } => {
                write!(f, "missing input {input} at block {block_id}")
            }
            Self::MissingField { block_id, field } => {
                write!(f, "missing field {field} at block {block_id}")
            }
            Self::MissingFieldValue { block_id, field } => {
                write!(f, "missing field value {field} at block {block_id}")
            }
            Self::InvalidBroadcastInput { block_id, input } => {
                write!(f, "invalid broadcast input {input} at block {block_id}")
            }
            Self::InvalidVariableField { block_id, field } => {
                write!(f, "invalid variable field {field} at block {block_id}")
            }
            Self::InvalidLiteral { block_id, context } => {
                write!(f, "invalid literal for {context} at block {block_id}")
            }
            Self::UnexpectedTopLevelBlock { block_id, opcode } => write!(
                f,
                "unexpected top-level opcode {opcode:?} at block {block_id}"
            ),
            Self::MissingStage => write!(f, "missing stage target"),
            Self::DuplicateStage => write!(f, "multiple stage targets"),
            Self::UnknownVariable {
                block_id,
                variable_id,
            } => write!(f, "unknown variable {variable_id} at block {block_id}"),
            Self::UnknownList { block_id, list_id } => {
                write!(f, "unknown list {list_id} at block {block_id}")
            }
            Self::UnknownBroadcast {
                block_id,
                broadcast_id,
            } => write!(f, "unknown broadcast {broadcast_id} at block {block_id}"),
            Self::MissingMutation { block_id, property } => {
                write!(
                    f,
                    "missing mutation property {property} at block {block_id}"
                )
            }
            Self::InvalidMutation { block_id, property } => {
                write!(
                    f,
                    "invalid mutation property {property} at block {block_id}"
                )
            }
            Self::UnknownProcedure { block_id, proccode } => {
                write!(f, "unknown procedure {proccode} at block {block_id}")
            }
            Self::UnknownProcedureArgument {
                block_id,
                procedure,
                argument_name,
            } => write!(
                f,
                "unknown procedure argument {argument_name} for {procedure:?} at block {block_id}"
            ),
        }
    }
}

impl std::error::Error for StructuredConversionError {}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Deserialize, Serialize)]
pub struct GlobalVariableId(pub usize);
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Deserialize, Serialize)]
pub struct LocalVariableId(pub usize);
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Deserialize, Serialize)]
pub struct GlobalListId(pub usize);
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Deserialize, Serialize)]
pub struct LocalListId(pub usize);
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Deserialize, Serialize)]
pub struct BroadcastId(pub usize);
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash, Deserialize, Serialize)]
pub struct ProcedureId(pub usize);

#[derive(Debug, Clone, Copy, PartialEq, Eq, Deserialize, Serialize)]
#[arbitrary_context]
pub enum VariableRef {
    Global(GlobalVariableId),
    Local(LocalVariableId),
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Deserialize, Serialize)]
#[arbitrary_context]
pub enum ListRef {
    Global(GlobalListId),
    Local(LocalListId),
}

#[derive(Arbitrary, Debug, Clone, PartialEq, Deserialize, Serialize)]
pub struct Variable {
    pub scratch_id: Box<str>,
    pub info: VariableInfo,
}

#[derive(Arbitrary, Debug, Clone, PartialEq, Deserialize, Serialize)]
pub struct List {
    pub scratch_id: Box<str>,
    pub name: Box<str>,
    pub value: Vec<VarVal>,
}

#[derive(Arbitrary, Debug, Clone, PartialEq, Eq, Deserialize, Serialize)]
pub struct Broadcast {
    pub scratch_id: Box<str>,
    pub name: Box<str>,
}

#[derive(Arbitrary, Debug, Clone, PartialEq, Eq, Deserialize, Serialize)]
pub struct Procedure {
    pub proccode: Box<str>,
    pub arguments: Vec<ProcedureArgument>,
    pub warp: bool,
}

#[derive(Arbitrary, Debug, Clone, PartialEq, Eq, Deserialize, Serialize)]
pub struct ProcedureArgument {
    pub name: Box<str>,
    pub kind: ProcedureArgumentKind,
    pub default: ProcedureArgumentDefault,
}

#[derive(Arbitrary, Mutate, Debug, Clone, Copy, PartialEq, Eq, Deserialize, Serialize)]
pub enum ProcedureArgumentKind {
    StringOrNumber,
    Boolean,
}

#[derive(Arbitrary, Debug, Clone, PartialEq, Eq, Deserialize, Serialize)]
pub enum ProcedureArgumentDefault {
    String(Box<str>),
    Boolean(bool),
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Deserialize, Serialize)]
#[arbitrary_context]
pub struct ProcedureArgumentRef {
    pub procedure: ProcedureId,
    // TODO: make this an arena ID, and make ProcedureContext
    pub argument_index: usize,
}

#[derive(Debug, Clone, PartialEq, Eq, Default, Deserialize, Serialize)]
pub struct Registry<I, T> {
    pub items: Vec<T>,
    _marker: std::marker::PhantomData<I>,
}

pub trait RegistryId: Copy {
    fn from_index(index: usize) -> Self;
    fn into_index(self) -> usize;
}

impl RegistryId for GlobalVariableId {
    fn from_index(index: usize) -> Self {
        Self(index)
    }

    fn into_index(self) -> usize {
        self.0
    }
}

impl RegistryId for LocalVariableId {
    fn from_index(index: usize) -> Self {
        Self(index)
    }

    fn into_index(self) -> usize {
        self.0
    }
}

impl RegistryId for GlobalListId {
    fn from_index(index: usize) -> Self {
        Self(index)
    }

    fn into_index(self) -> usize {
        self.0
    }
}

impl RegistryId for LocalListId {
    fn from_index(index: usize) -> Self {
        Self(index)
    }

    fn into_index(self) -> usize {
        self.0
    }
}

impl RegistryId for BroadcastId {
    fn from_index(index: usize) -> Self {
        Self(index)
    }

    fn into_index(self) -> usize {
        self.0
    }
}

impl RegistryId for ProcedureId {
    fn from_index(index: usize) -> Self {
        Self(index)
    }

    fn into_index(self) -> usize {
        self.0
    }
}

impl<I: RegistryId, T> Registry<I, T> {
    #[must_use]
    pub const fn new() -> Self {
        Self {
            items: Vec::new(),
            _marker: std::marker::PhantomData,
        }
    }

    pub fn push(&mut self, value: T) -> I {
        let id = I::from_index(self.items.len());
        self.items.push(value);
        id
    }

    pub fn get(&self, id: I) -> Option<&T> {
        self.items.get(id.into_index())
    }
}

impl<Id: RegistryId, T> FromIterator<T> for Registry<Id, T> {
    fn from_iter<I: IntoIterator<Item = T>>(iter: I) -> Self {
        let mut registry = Self::new();
        for item in iter {
            registry.push(item);
        }
        registry
    }
}

#[derive(Debug, Clone, PartialEq, Deserialize, Serialize)]
pub struct StructuredProject {
    pub global_variables: Registry<GlobalVariableId, Variable>,
    pub global_lists: Registry<GlobalListId, List>,
    pub broadcasts: Registry<BroadcastId, Broadcast>,
    pub targets: Vec<StructuredTarget>,
    pub monitors: Vec<Monitor>,
    pub extensions: Vec<Box<str>>,
    pub meta: Meta,
}

impl StructuredProject {
    pub fn broadcast(&self, broadcast: BroadcastId) -> Option<&Broadcast> {
        self.broadcasts.get(broadcast)
    }
}

impl StructuredTarget {
    pub fn variable(&self, variable: LocalVariableId) -> Option<&Variable> {
        self.local_variables.get(variable)
    }

    pub fn list(&self, list: LocalListId) -> Option<&List> {
        self.local_lists.get(list)
    }

    pub fn procedure(&self, procedure: ProcedureId) -> Option<&Procedure> {
        self.local_procedures.get(procedure)
    }
}

#[derive(Debug, Clone, PartialEq, Deserialize, Serialize)]
pub struct StructuredTarget {
    pub is_stage: bool,
    pub name: Box<str>,
    pub local_variables: Registry<LocalVariableId, Variable>,
    pub local_lists: Registry<LocalListId, List>,
    pub local_procedures: Registry<ProcedureId, Procedure>,
    pub scripts: Vec<Script>,
    pub comments: Vec<(Box<str>, Comment)>,
    pub current_costume: u32,
    pub costumes: Vec<Costume>,
    pub sounds: Vec<Sound>,
    pub layer_order: i32,
    pub volume: f64,
    pub tempo: f64,
    pub video_state: Option<Box<str>>,
    pub video_transparency: f64,
    pub text_to_speech_language: Option<Box<str>>,
    pub visible: bool,
    pub x: f64,
    pub y: f64,
    pub size: f64,
    pub direction: f64,
    pub draggable: bool,
    pub rotation_style: Box<str>,
}

#[derive(Debug, Clone, PartialEq, Deserialize, Serialize)]
#[arbitrary_context]
pub struct Script {
    pub hat: Option<Hat>,
    pub position: Option<ScriptPosition>,
    pub body: Vec<Statement>,
    // TODO: enforce body nonempty XOR top_reporter nonempty
    pub top_reporter: Option<Value>,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Arbitrary, Deserialize, Serialize)]
#[arbitrary_context]
pub struct ScriptPosition {
    pub x: i32,
    pub y: i32,
}

#[derive(Debug, Clone, PartialEq, Eq, Deserialize, Serialize)]
#[arbitrary_context]
pub enum Hat {
    WhenFlagClicked,
    WhenBroadcastReceived {
        broadcast: BroadcastId,
    },
    #[arbitrary(skip)]
    ProcedureDefinition {
        procedure: ProcedureId,
    },
}

#[derive(Debug, Clone, PartialEq, Deserialize, Serialize)]
#[arbitrary_context]
pub enum Statement {
    SetVariable {
        variable: VariableRef,
        value: Option<Value>,
    },
    ChangeVariable {
        variable: VariableRef,
        value: Option<Value>,
    },
    ShowVariable {
        variable: VariableRef,
    },
    HideVariable {
        variable: VariableRef,
    },
    AddToList {
        list: ListRef,
        item: Option<Value>,
    },
    DeleteOfList {
        list: ListRef,
        index: Option<Value>,
    },
    DeleteAllOfList {
        list: ListRef,
    },
    InsertAtList {
        list: ListRef,
        index: Option<Value>,
        item: Option<Value>,
    },
    ReplaceItemOfList {
        list: ListRef,
        index: Option<Value>,
        item: Option<Value>,
    },
    ShowList {
        list: ListRef,
    },
    HideList {
        list: ListRef,
    },
    Wait {
        duration: Option<Value>,
    },
    WaitUntil {
        condition: Option<Predicate>,
    },
    If {
        condition: Option<Predicate>,
        body: Vec<Statement>,
    },
    IfElse {
        condition: Option<Predicate>,
        then_body: Vec<Statement>,
        else_body: Vec<Statement>,
    },
    Repeat {
        times: Option<Value>,
        body: Vec<Statement>,
    },
    RepeatUntil {
        condition: Option<Predicate>,
        body: Vec<Statement>,
    },
    While {
        condition: Option<Predicate>,
        body: Vec<Statement>,
    },
    Forever {
        body: Vec<Statement>,
    },
    Stop {
        option: StopOption,
    },
    Broadcast {
        broadcast: BroadcastId,
    },
    BroadcastAndWait {
        broadcast: BroadcastId,
    },
    AskAndWait {
        question: Option<Value>,
    },
    ResetTimer,
    Say {
        message: Option<Value>,
    },
    SayForSecs {
        message: Option<Value>,
        seconds: Option<Value>,
    },
    Think {
        message: Option<Value>,
    },
    ThinkForSecs {
        message: Option<Value>,
        seconds: Option<Value>,
    },
    Show,
    Hide,
    SwitchCostumeTo {
        costume: Option<Value>,
    },
    SwitchBackdropTo {
        backdrop: Option<Value>,
    },
    SwitchBackdropToAndWait {
        backdrop: Option<Value>,
    },
    NextCostume,
    NextBackdrop,
    ChangeSizeBy {
        amount: Option<Value>,
    },
    SetSizeTo {
        size: Option<Value>,
    },
    MoveSteps {
        steps: Option<Value>,
    },
    GoToXY {
        x: Option<Value>,
        y: Option<Value>,
    },
    TurnRight {
        degrees: Option<Value>,
    },
    TurnLeft {
        degrees: Option<Value>,
    },
    PointInDirection {
        direction: Option<Value>,
    },
    ChangeXBy {
        amount: Option<Value>,
    },
    SetX {
        value: Option<Value>,
    },
    ChangeYBy {
        amount: Option<Value>,
    },
    SetY {
        value: Option<Value>,
    },
    CallProcedure {
        procedure: ProcedureId,
        arguments: Vec<ProcedureInput>,
    },
    PenSetColorToColor {
        value: Option<Value>,
    },
    PenChangeColorParamBy {
        param: Option<Value>,
        value: Option<Value>,
    },
    PenSetColorParamTo {
        param: Option<Value>,
        value: Option<Value>,
    },
    PenSetSizeTo {
        value: Option<Value>,
    },
    PenDown,
    PenUp,
    PenClear,
}

#[derive(Debug, Clone, PartialEq, Deserialize, Serialize)]
#[arbitrary_context]
pub enum ProcedureInput {
    Value(Option<Value>),
    Predicate(Option<Predicate>),
}

#[derive(Arbitrary, Debug, Clone, PartialEq, Eq, Deserialize, Serialize)]
#[arbitrary_context]
pub enum StopOption {
    All,
    ThisScript,
    OtherScriptsInSprite,
}

#[derive(Debug, Clone, PartialEq, Deserialize, Serialize)]
#[arbitrary_context]
pub enum Value {
    Variable(VariableRef),
    ListContents(ListRef),
    Literal(Literal),
    Predicate(Box<Predicate>),
    Add(Option<Box<Value>>, Option<Box<Value>>),
    Subtract(Option<Box<Value>>, Option<Box<Value>>),
    Multiply(Option<Box<Value>>, Option<Box<Value>>),
    Divide(Option<Box<Value>>, Option<Box<Value>>),
    Random(Option<Box<Value>>, Option<Box<Value>>),
    Join(Option<Box<Value>>, Option<Box<Value>>),
    LetterOf {
        letter: Option<Box<Value>>,
        text: Option<Box<Value>>,
    },
    Length(Option<Box<Value>>),
    Contains {
        text: Option<Box<Value>>,
        search: Option<Box<Value>>,
    },
    Modulo(Option<Box<Value>>, Option<Box<Value>>),
    Round(Option<Box<Value>>),
    MathOp {
        operator: MathOperator,
        operand: Option<Box<Value>>,
    },
    ItemOfList {
        list: ListRef,
        index: Option<Box<Value>>,
    },
    ItemNumOfList {
        list: ListRef,
        item: Option<Box<Value>>,
    },
    LengthOfList(ListRef),
    Answer,
    MouseX,
    MouseY,
    Timer,
    DaysSince2000,
    XPosition,
    YPosition,
    Direction,
    Size,
    CostumeNumber,
    BackdropNumber,
    Volume,
    ProcedureArgument(ProcedureArgumentRef),
    PenMenuColorParam(Box<str>),
    KeyOptions(Box<str>),
}

#[derive(Debug, Clone, PartialEq, Deserialize, Serialize)]
#[arbitrary_context]
pub enum Predicate {
    LessThan(Option<Box<Value>>, Option<Box<Value>>),
    Equals(Option<Box<Value>>, Option<Box<Value>>),
    GreaterThan(Option<Box<Value>>, Option<Box<Value>>),
    And(Option<Box<Predicate>>, Option<Box<Predicate>>),
    Or(Option<Box<Predicate>>, Option<Box<Predicate>>),
    Not(Option<Box<Predicate>>),
    MouseDown,
    ListContainsItem {
        list: ListRef,
        item: Option<Box<Value>>,
    },
    ItemOfList {
        list: ListRef,
        index: Option<Box<Value>>,
    },
    ItemNumOfList {
        list: ListRef,
        item: Option<Box<Value>>,
    },
    ProcedureArgument(ProcedureArgumentRef),
    KeyPressed(Option<Box<Value>>),
}

#[derive(Arbitrary, Debug, Clone, PartialEq, Deserialize, Serialize)]
#[arbitrary_context]
pub enum Literal {
    Number(f64),
    String(Box<str>),
    Color(Box<str>),
}

#[derive(Arbitrary, Debug, Clone, PartialEq, Eq, Deserialize, Serialize)]
#[arbitrary_context]
pub enum MathOperator {
    Abs,
    Floor,
    Ceiling,
    Sqrt,
    Sin,
    Cos,
    Tan,
    Asin,
    Acos,
    Atan,
    Ln,
    Log,
    Exp,
    Pow10,
}

#[cfg(feature = "fuzz")]
impl HasLen for StructuredProject {
    fn len(&self) -> usize {
        let sb3: Sb3Project = self.clone().try_into().unwrap();
        serde_json::to_vec(&sb3).unwrap().len()
    }
}

#[cfg(feature = "fuzz")]
impl HasTargetBytes for StructuredProject {
    fn target_bytes(&'_ self) -> libafl_bolts::ownedref::OwnedSlice<'_, u8> {
        serde_json::to_vec(&Sb3Project::try_from(self.clone()).unwrap())
            .unwrap()
            .into()
    }
}

#[cfg(feature = "fuzz")]
impl core::hash::Hash for StructuredProject {
    fn hash<H: std::hash::Hasher>(&self, state: &mut H) {
        state.write(
            serde_json::to_vec(&Sb3Project::try_from(self.clone()).unwrap())
                .unwrap()
                .as_slice(),
        )
    }
}

#[cfg(feature = "fuzz")]
impl Input for StructuredProject {}
