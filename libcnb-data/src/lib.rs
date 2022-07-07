#![doc = include_str!("../README.md")]
#![warn(clippy::pedantic)]
#![warn(unused_crate_dependencies)]
// This lint is too noisy and enforces a style that reduces readability in many cases.
#![allow(clippy::module_name_repetitions)]

pub mod bom;
pub mod build;
pub mod build_plan;
pub mod buildpack;
pub mod buildpack_plan;
pub mod exec_d;
pub mod launch;
pub mod layer;
pub mod layer_content_metadata;
pub mod store;

mod newtypes;

// Internals that need to be public for macros
#[doc(hidden)]
pub mod internals;
