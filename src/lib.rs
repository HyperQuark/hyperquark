#![cfg_attr(not(test), no_std)]
#![recursion_limit = "256"]

#[macro_use]
extern crate alloc;
extern crate enum_field_getter;

pub mod sb3;
pub mod targets;
