use alloc::boxed::Box;
#[cfg(target_family = "wasm")]
use wasm_bindgen::JsValue;

pub type HQResult<T> = Result<T, HQError>;

#[derive(Clone, Debug)] // todo: get rid of this once all expects are gone
pub struct HQError {
    pub err_type: HQErrorType,
    pub msg: Box<str>,
    pub file: Box<str>,
    pub line: u32,
    pub column: u32,
}
#[derive(Clone, Debug, PartialEq)] // todo: get rid of this once all expects are gone
pub enum HQErrorType {
    MalformedProject,
    InternalError,
    Unimplemented,
}

#[cfg(target_family = "wasm")]
impl From<HQError> for JsValue {
    fn from(val: HQError) -> JsValue {
        JsValue::from_str(match val.err_type {
            HQErrorType::Unimplemented => format!("todo: {}<br>at {}:{}:{}<br>this is a bug or missing feature that is known and will be fixed or implemented in a future update", val.msg, val.file, val.line, val.column),
            HQErrorType::InternalError => format!("error: {}<br>at {}:{}:{}<br>this is probably a bug with HyperQuark itself. Please report this bug, with this error message, at <a href=\"https://github.com/hyperquark/hyperquark/issues/new\">https://github.com/hyperquark/hyperquark/issues/new</a>", val.msg, val.file, val.line, val.column),
            HQErrorType::MalformedProject => format!("error: {}<br>at {}:{}:{}<br>this is probably a problem with the project itself, but if it works in vanilla scratch then this is a bug; please report it, by creating an issue at <a href=\"https://github.com/hyperquark/hyperquark/issues/new\">https://github.com/hyperquark/hyperquark/issues/new</a>, including this error message", val.msg, val.file, val.line, val.column),
        }.as_str())
    }
}

#[macro_export]
macro_rules! hq_todo {
    () => {{
        return Err($crate::HQError {
            err_type: $crate::HQErrorType::Unimplemented,
            msg: "todo".into(),
            file: file!().into(),
            line: line!(),
            column: column!()
        });
    }};
    ($($args:tt)+) => {{
        return Err($crate::HQError {
            err_type: $crate::HQErrorType::Unimplemented,
            msg: format!("{}", format_args!($($args)*)).into(),
            file: file!().into(),
            line: line!(),
            column: column!()
        });
    }};
}

#[macro_export]
macro_rules! hq_bug {
    ($($args:tt)+) => {{
        return Err($crate::HQError {
            err_type: $crate::HQErrorType::InternalError,
            msg: format!("{}", format_args!($($args)*)).into(),
            file: file!().into(),
            line: line!(),
            column: column!()
        });
    }};
}

#[macro_export]
macro_rules! hq_bad_proj {
    ($($args:tt)+) => {{
        return Err($crate::HQError {
            err_type: $crate::HQErrorType::MalformedProject,
            msg: format!("{}", format_args!($($args)*)).into(),
            file: file!().into(),
            line: line!(),
            column: column!()
        });
    }};
}

#[macro_export]
macro_rules! make_hq_todo {
    ($($args:tt)+) => {{
        use $crate::alloc::Box<str>::ToBox<str>;
        $crate::HQError {
            err_type: $crate::HQErrorType::Unimplemented,
            msg: format!("{}", format_args!($($args)*)).into(),
            file: file!().into(),
            line: line!(),
            column: column!()
        }
    }};
}

#[macro_export]
macro_rules! make_hq_bug {
    ($($args:tt)+) => {{
        $crate::HQError {
            err_type: $crate::HQErrorType::InternalError,
            msg: format!("{}", format_args!($($args)*)).into(),
            file: file!().into(),
            line: line!(),
            column: column!()
        }
    }};
}

#[macro_export]
macro_rules! make_hq_bad_proj {
    ($($args:tt)+) => {{
        $crate::HQError {
            err_type: $crate::HQErrorType::MalformedProject,
            msg: format!("{}", format_args!($($args)*)).into(),
            file: file!().into(),
            line: line!(),
            column: column!()
        }
    }};
}
