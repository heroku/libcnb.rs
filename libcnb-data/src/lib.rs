//! Low-level representations for Cloud Native Buildpack data types.

// Enable rustc and Clippy lints that are disabled by default.
// https://doc.rust-lang.org/rustc/lints/listing/allowed-by-default.html#unused-crate-dependencies
#![warn(unused_crate_dependencies)]
// https://rust-lang.github.io/rust-clippy/stable/index.html
#![warn(clippy::pedantic)]
// This lint is too noisy and enforces a style that reduces readability in many cases.
#![allow(clippy::module_name_repetitions)]

pub mod bom;
pub mod build;
pub mod build_plan;
pub mod buildpack;
pub mod buildpack_plan;
pub mod launch;
pub mod layer;
pub mod layer_content_metadata;
pub mod store;

mod newtypes;

// Internals that need to be public for macros
#[doc(hidden)]
pub mod internals;
