use core::cell::{BorrowError, BorrowMutError};

use alloc::boxed::Box;
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
#[derive(Clone, Debug, PartialEq, Eq)] // todo: get rid of this once all expects are gone
pub enum HQErrorType {
    MalformedProject,
    InternalError,
    Unimplemented,
}

impl From<HQError> for JsValue {
    fn from(val: HQError) -> Self {
        Self::from_str(match val.err_type {
            HQErrorType::Unimplemented => format!("todo: {}<br>at {}:{}:{}<br>this is a bug or missing feature that is known and will be fixed or implemented in a future update", val.msg, val.file, val.line, val.column),
            HQErrorType::InternalError => format!("error: {}<br>at {}:{}:{}<br>this is probably a bug with HyperQuark itself. Please report this bug, with this error message, at <a href=\"https://github.com/hyperquark/hyperquark/issues/new\">https://github.com/hyperquark/hyperquark/issues/new</a>", val.msg, val.file, val.line, val.column),
            HQErrorType::MalformedProject => format!("error: {}<br>at {}:{}:{}<br>this is probably a problem with the project itself, but if it works in vanilla scratch then this is a bug; please report it, by creating an issue at <a href=\"https://github.com/hyperquark/hyperquark/issues/new\">https://github.com/hyperquark/hyperquark/issues/new</a>, including this error message", val.msg, val.file, val.line, val.column),
        }.as_str())
    }
}

#[cfg(feature = "panic")]
#[clippy::format_args]
macro_rules! maybe_panic {
    ($($args:tt)+) => {{
        panic!($($args)+);
    }}
}

#[cfg(not(feature = "panic"))]
#[clippy::format_args]
macro_rules! maybe_panic {
    ($($args:tt)+) => {{}};
}

impl From<BorrowError> for HQError {
    #[cfg_attr(
        feature = "panic",
        expect(unreachable_code, reason = "panic infrastructure only for debugging")
    )]
    fn from(_e: BorrowError) -> Self {
        maybe_panic!("couldn't borrow cell");
        Self {
            err_type: HQErrorType::InternalError,
            msg: "couldn't borrow cell".into(),
            file: file!().into(),
            line: line!(),
            column: column!(),
        }
    }
}

impl From<BorrowMutError> for HQError {
    #[cfg_attr(
        feature = "panic",
        expect(unreachable_code, reason = "panic infrastructure only for debugging")
    )]
    fn from(_e: BorrowMutError) -> Self {
        maybe_panic!("couldn't mutably borrow cell");
        Self {
            err_type: HQErrorType::InternalError,
            msg: "couldn't mutably borrow cell".into(),
            file: file!().into(),
            line: line!(),
            column: column!(),
        }
    }
}

#[macro_export]
#[clippy::format_args]
macro_rules! hq_todo {
    () => {#[cfg_attr(feature = "panic", expect(unreachable_code, reason = "panic infrastructure only for debugging"))]{
        maybe_panic!("todo");
        return Err($crate::HQError {
            err_type: $crate::HQErrorType::Unimplemented,
            msg: "todo".into(),
            file: file!().into(),
            line: line!(),
            column: column!()
        });
    }};
    ($($args:tt)+) => {#[cfg_attr(feature = "panic", expect(unreachable_code, reason = "panic infrastructure only for debugging"))]{
        maybe_panic!($($args)+);
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
#[clippy::format_args]
macro_rules! hq_bug {
    ($($args:tt)+) => {#[cfg_attr(feature = "panic", expect(unreachable_code, reason = "panic infrastructure only for debugging"))]{
        maybe_panic!($($args)+);
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
#[clippy::format_args]
macro_rules! hq_assert {
    ($expr:expr) => {#[cfg_attr(feature = "panic", expect(unreachable_code, reason = "panic infrastructure only for debugging"))]{
        if !($expr) {
            maybe_panic!("Assertion failed: {}", stringify!($expr));
            return Err($crate::HQError {
                err_type: $crate::HQErrorType::InternalError,
                msg: format!("Assertion failed: {}", stringify!($expr)).into(),
                file: file!().into(),
                line: line!(),
                column: column!()
            });
        };
        assert!($expr);
    }};
    ($expr:expr, $($args:tt)+) => {#[cfg_attr(feature = "panic", expect(unreachable_code, reason = "panic infrastructure only for debugging"))]{
        if !($expr) {
            maybe_panic!("Assertion failed: {}\nMessage: {}", stringify!($expr), format_args!($($args)*));
            return Err($crate::HQError {
                err_type: $crate::HQErrorType::InternalError,
                msg: format!("Assertion failed: {}\nMessage: {}", stringify!($expr), format_args!($($args)*)).into(),
                file: file!().into(),
                line: line!(),
                column: column!()
            });
        };
        assert!($expr);
    }};
}

#[macro_export]
#[clippy::format_args]
macro_rules! hq_assert_eq {
    ($l:expr, $r:expr) => {#[cfg_attr(feature = "panic", expect(unreachable_code, reason = "panic infrastructure only for debugging"))]{
        if $l != $r {
            maybe_panic!("Assertion failed: {} == {}\nLeft: {}\nRight: {}", stringify!($l), stringify!($r), $l, $r);
            return Err($crate::HQError {
                err_type: $crate::HQErrorType::InternalError,
                msg: format!("Assertion failed: {} == {}\nLeft: {}\nRight: {}", stringify!($l), stringify!($r), $l, $r).into(),
                file: file!().into(),
                line: line!(),
                column: column!()
            });
        };
        assert_eq!($l, $r);
    }};
    ($l:expr, $r:expr, $($args:tt)+) => {#[cfg_attr(feature = "panic", expect(unreachable_code, reason = "panic infrastructure only for debugging"))]{
        if $l != $r {
            maybe_panic!("Assertion failed: {}\nLeft: {}\nRight: {}\nMessage: {}", stringify!($l), stringify!($r), $l, $r, format_args!($($args)*));
            return Err($crate::HQError {
                err_type: $crate::HQErrorType::InternalError,
                msg: format!("Assertion failed: {}\nLeft: {}\nRight: {}\nMessage: {}", stringify!($l), stringify!($r), $l, $r, format_args!($($args)*)).into(),
                file: file!().into(),
                line: line!(),
                column: column!()
            });
        };
        assert_eq!($l, $r);
    }};
}

#[macro_export]
#[clippy::format_args]
macro_rules! hq_bad_proj {
    ($($args:tt)+) => {#[cfg_attr(feature = "panic", expect(unreachable_code, reason = "panic infrastructure only for debugging"))]{
        maybe_panic!($($args)+);
        return Err($crate::HQError {
            err_type: $crate::HQErrorType::MalformedProject,
            msg: format!("{}", format_args!($($args)*)).into(),
            file: file!().into(),
            line: line!(),
            column: column!()
        });
    }};
}

/// for use in `ok_or_else` and similar methods
#[macro_export]
#[clippy::format_args]
macro_rules! make_hq_todo {
    ($($args:tt)+) => {#[cfg_attr(feature = "panic", expect(unreachable_code, reason = "panic infrastructure only for debugging"))]{
        maybe_panic!($($args)+);
        use $crate::alloc::Box<str>::ToBox<str>;
        return $crate::HQError {
            err_type: $crate::HQErrorType::Unimplemented,
            msg: format!("{}", format_args!($($args)*)).into(),
            file: file!().into(),
            line: line!(),
            column: column!()
        }
    }};
}

/// for use in `ok_or_else` and similar methods
#[macro_export]
#[clippy::format_args]
macro_rules! make_hq_bug {
    ($($args:tt)+) => {#[cfg_attr(feature = "panic", expect(unreachable_code, reason = "panic infrastructure only for debugging"))]{
        maybe_panic!($($args)+);
        return $crate::HQError {
            err_type: $crate::HQErrorType::InternalError,
            msg: format!("{}", format_args!($($args)*)).into(),
            file: file!().into(),
            line: line!(),
            column: column!()
        }
    }};
}

/// for use in `ok_or_else` and similar methods
#[macro_export]
#[clippy::format_args]
macro_rules! make_hq_bad_proj {
    ($($args:tt)+) => {#[cfg_attr(feature = "panic", expect(unreachable_code, reason = "panic infrastructure only for debugging"))]{
        maybe_panic!($($args)+);
        return $crate::HQError {
            err_type: $crate::HQErrorType::MalformedProject,
            msg: format!("{}", format_args!($($args)*)).into(),
            file: file!().into(),
            line: line!(),
            column: column!()
        }
    }};
}
