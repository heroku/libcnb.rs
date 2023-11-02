#![doc = include_str!("../README.md")]
#![warn(unused_crate_dependencies)]
#![warn(clippy::pedantic)]
#![warn(clippy::panic_in_result_fn)]
#![warn(clippy::unwrap_used)]
// This lint is too noisy and enforces a style that reduces readability in many cases.
#![allow(clippy::module_name_repetitions)]

pub mod toml_file;
