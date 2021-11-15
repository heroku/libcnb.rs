//! Low-level representations for Cloud Native Buildpack data types.

// Enable rustc and Clippy lints that are disabled by default.
// https://doc.rust-lang.org/rustc/lints/listing/allowed-by-default.html#unused-crate-dependencies
#![warn(unused_crate_dependencies)]
// https://rust-lang.github.io/rust-clippy/stable/index.html
#![warn(clippy::pedantic)]
// This lint is too noisy and enforces a style that reduces readability in many cases.
#![allow(clippy::module_name_repetitions)]
// Re-disable pedantic lints that are currently failing, until they are triaged and fixed/wontfixed.
// https://github.com/Malax/libcnb.rs/issues/53
#![allow(clippy::missing_errors_doc)]
// https://github.com/Malax/libcnb.rs/issues/57
#![allow(clippy::must_use_candidate)]

pub mod bom;
pub mod build;
pub mod build_plan;
pub mod buildpack;
pub mod buildpack_plan;
pub mod defaults;
pub mod launch;
pub mod layer_content_metadata;
pub mod store;
