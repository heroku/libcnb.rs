//! This crate provides a library to implement [Cloud Native Buildpacks](https://buildpacks.io/).

// Enable rustc and Clippy lints that are disabled by default.
// https://doc.rust-lang.org/rustc/lints/listing/allowed-by-default.html#unused-crate-dependencies
#![warn(unused_crate_dependencies)]
// https://rust-lang.github.io/rust-clippy/stable/index.html
#![warn(clippy::pedantic)]
// This lint is too noisy and enforces a style that reduces readability in many cases.
#![allow(clippy::module_name_repetitions)]
// This lint triggers when both layer_dir and layers_dir are present which are quite common.
#![allow(clippy::similar_names)]
// Re-disable pedantic lints that are currently failing, until they are triaged and fixed/wontfixed.
// https://github.com/Malax/libcnb.rs/issues/53
#![allow(clippy::missing_errors_doc)]
// https://github.com/Malax/libcnb.rs/issues/57
#![allow(clippy::must_use_candidate)]
// https://github.com/Malax/libcnb.rs/issues/63
#![allow(clippy::needless_pass_by_value)]
// https://github.com/Malax/libcnb.rs/issues/64
#![allow(clippy::unnecessary_wraps)]

pub mod build;
pub mod detect;
pub mod generic;
pub mod layer;
pub mod layer_env;

mod buildpack;
mod env;
mod error;
mod platform;
mod runtime;
mod toml_file;
mod util;

#[doc(inline)]
pub use libcnb_data as data;

pub use env::*;
pub use error::*;
pub use platform::*;
pub use toml_file::*;

pub use buildpack::Buildpack;
pub use runtime::libcnb_runtime;

const LIBCNB_SUPPORTED_BUILDPACK_API: data::buildpack::BuildpackApi =
    data::buildpack::BuildpackApi { major: 0, minor: 6 };

/// Generates a main function for the given buildpack.
///
/// It will create the main function and wires up the buildpack to the framework.
///
/// # Example:
/// ```
/// use libcnb::build::{BuildContext, BuildResult, BuildResultBuilder};
/// use libcnb::detect::{DetectContext, DetectResult, DetectResultBuilder};
/// use libcnb::generic::{GenericError, GenericMetadata, GenericPlatform};
/// use libcnb::{buildpack_main, data::build_plan::BuildPlan, Buildpack};
///
/// struct MyBuildpack;
///
/// impl Buildpack for MyBuildpack {
///     type Platform = GenericPlatform;
///     type Metadata = GenericMetadata;
///     type Error = GenericError;
///
///     fn detect(&self, context: DetectContext<Self>) -> libcnb::Result<DetectResult, Self::Error> {
///         Ok(DetectResultBuilder::pass().build())
///     }
///
///     fn build(&self, context: BuildContext<Self>) -> libcnb::Result<BuildResult, Self::Error> {
///         Ok(BuildResultBuilder::new().build())
///     }
/// }
///
/// buildpack_main!(MyBuildpack);
/// ```
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
