//! Low-level representations for Cloud Native Buildpack data types.

// Enable rustc and Clippy lints that are disabled by default.
// https://doc.rust-lang.org/rustc/lints/listing/allowed-by-default.html#unused-crate-dependencies
#![warn(unused_crate_dependencies)]
// https://rust-lang.github.io/rust-clippy/stable/index.html
#![warn(clippy::pedantic)]
// Re-disable pedantic lints that are currently failing, until they are triaged and fixed/wontfixed.
// https://github.com/Malax/libcnb.rs/issues/53
#![allow(clippy::missing_errors_doc)]
// https://github.com/Malax/libcnb.rs/issues/83
#![allow(clippy::module_name_repetitions)]
// https://github.com/Malax/libcnb.rs/issues/57
#![allow(clippy::must_use_candidate)]
// https://github.com/Malax/libcnb.rs/issues/61
#![allow(clippy::redundant_closure_for_method_calls)]

pub mod bom;
pub mod build;
pub mod build_plan;
pub mod buildpack;
pub mod buildpack_plan;
pub mod defaults;
pub mod launch;
pub mod layer_content_metadata;
pub mod stack_id;
pub mod store;
