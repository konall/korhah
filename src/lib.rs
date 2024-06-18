#![no_std]
#![doc = include_str!(concat!(env!("CARGO_MANIFEST_DIR"), "/README.md"))]

extern crate alloc;

mod compat;
/// Events that are emitted as a result of built-in CRUD actions
pub mod events;
mod listener;
mod system;
mod variable;

pub(crate) type Id = u128;

pub use listener::{Listener, Vote, Votes};
pub use system::System;
pub use variable::Variable;
