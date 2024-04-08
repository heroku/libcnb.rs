#![doc = include_str!("../README.md")]

mod app;
mod build;
mod build_config;
mod container_config;
mod container_context;
mod docker;
mod log;
mod macros;
mod pack;
mod test_context;
mod test_runner;
mod util;

pub use crate::build_config::*;
pub use crate::container_config::*;
pub use crate::container_context::*;
pub use crate::log::*;
pub use crate::test_context::*;
pub use crate::test_runner::*;

// Suppress warnings due to the `unused_crate_dependencies` lint not handling integration tests well.
#[cfg(test)]
use indoc as _;
#[cfg(test)]
use libcnb as _;
#[cfg(test)]
use regex_lite as _;
#[cfg(test)]
use ureq as _;
