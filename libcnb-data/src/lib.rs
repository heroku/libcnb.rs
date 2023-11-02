#![doc = include_str!("../README.md")]
#![warn(unused_crate_dependencies)]
#![warn(clippy::pedantic)]
#![warn(clippy::panic_in_result_fn)]
#![warn(clippy::unwrap_used)]
// This lint is too noisy and enforces a style that reduces readability in many cases.
#![allow(clippy::module_name_repetitions)]

pub mod build;
pub mod build_plan;
pub mod buildpack;
pub mod buildpack_plan;
pub mod exec_d;
pub mod generic;
pub mod launch;
pub mod layer;
pub mod layer_content_metadata;
pub mod package_descriptor;
pub mod sbom;
pub mod store;

mod newtypes;

// Internals that need to be public for macros
#[doc(hidden)]
pub mod internals;
