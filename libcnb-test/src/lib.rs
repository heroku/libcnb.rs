// Enable rustc and Clippy lints that are disabled by default.
// https://doc.rust-lang.org/rustc/lints/listing/allowed-by-default.html#unused-crate-dependencies
#![warn(unused_crate_dependencies)]
// https://rust-lang.github.io/rust-clippy/stable/index.html
#![warn(clippy::pedantic)]
// This lint is too noisy and enforces a style that reduces readability in many cases.
#![allow(clippy::module_name_repetitions)]

mod app;
mod build;
mod container_context;
mod container_port_mapping;
mod log;
mod macros;
mod pack;
mod test_config;
mod test_context;
mod test_runner;
mod util;

pub use crate::container_context::*;
pub use crate::log::*;
pub use crate::test_config::*;
pub use crate::test_context::*;
pub use crate::test_runner::*;

// This runs the README.md as a doctest, ensuring the code examples in it are valid.
// It will not be part of the final crate.
#[doc = include_str!("../README.md")]
#[cfg(doctest)]
pub struct ReadmeDoctests;
