//! This crate provides a library to implement [Cloud Native Buildpacks](https://buildpacks.io/).

// Enable rustc and Clippy lints that are disabled by default.
// https://doc.rust-lang.org/rustc/lints/listing/allowed-by-default.html#unused-crate-dependencies
#![warn(unused_crate_dependencies)]
// https://rust-lang.github.io/rust-clippy/stable/index.html
#![warn(clippy::pedantic)]
// Re-disable pedantic lints that are currently failing, until they are triaged and fixed/wontfixed.
// https://github.com/Malax/libcnb.rs/issues/53
#![allow(clippy::missing_errors_doc)]
// https://github.com/Malax/libcnb.rs/issues/54
#![allow(clippy::missing_panics_doc)]
// https://github.com/Malax/libcnb.rs/issues/83
#![allow(clippy::module_name_repetitions)]
// https://github.com/Malax/libcnb.rs/issues/57
#![allow(clippy::must_use_candidate)]
// https://github.com/Malax/libcnb.rs/issues/63
#![allow(clippy::needless_pass_by_value)]
// https://github.com/Malax/libcnb.rs/issues/64
#![allow(clippy::unnecessary_wraps)]

pub mod build;
pub mod detect;
pub mod layer_env;
pub mod layer_lifecycle;

use crate::data::buildpack::BuildpackApi;
pub use buildpack::Buildpack;

pub use env::*;
pub use error::*;
pub use generic::*;
pub use libcnb_data as data;
pub use platform::*;
pub use runtime::libcnb_runtime;
pub use toml_file::*;

mod buildpack;
mod env;
mod error;
mod generic;
mod platform;
mod runtime;
mod toml_file;

const LIBCNB_SUPPORTED_BUILDPACK_API: BuildpackApi = BuildpackApi { major: 0, minor: 6 };

#[macro_export]
macro_rules! buildpack_main {
    ($buildpack:expr) => {
        fn main() {
            ::libcnb::libcnb_runtime($buildpack);
        }
    };
}

// This runs the README.md as a doctest, ensuring the code examples in it are valid.
// It will not be part of the final crate.
#[cfg(doctest)]
#[doc = include_str!("../../README.md")]
pub struct ReadmeDoctests;
