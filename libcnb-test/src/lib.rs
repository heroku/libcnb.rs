#![doc = include_str!("../README.md")]
// Enable lints that are disabled by default.
#![warn(unused_crate_dependencies)]
#![warn(clippy::pedantic)]
// This lint is too noisy and enforces a style that reduces readability in many cases.
#![allow(clippy::module_name_repetitions)]

mod app;
mod build;
mod container_config;
mod container_context;
mod container_port_mapping;
mod log;
mod macros;
mod pack;
mod test_config;
mod test_context;
mod test_runner;
mod util;

pub use crate::container_config::*;
pub use crate::container_context::*;
pub use crate::log::*;
pub use crate::test_config::*;
pub use crate::test_context::*;
pub use crate::test_runner::*;

// Suppress warnings due to the `unused_crate_dependencies` lint not handling integration tests well.
#[cfg(test)]
use indoc as _;
#[cfg(test)]
use ureq as _;
