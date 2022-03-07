//! This crate provides a library to implement [Cloud Native Buildpacks](https://buildpacks.io/).

// Enable rustc and Clippy lints that are disabled by default.
// https://doc.rust-lang.org/rustc/lints/listing/allowed-by-default.html#unused-crate-dependencies
#![warn(unused_crate_dependencies)]
// https://rust-lang.github.io/rust-clippy/stable/index.html
#![warn(clippy::pedantic)]
// Most of libcnb's public API returns user-provided errors, making error docs redundant.
#![allow(clippy::missing_errors_doc)]
// This lint is too noisy and enforces a style that reduces readability in many cases.
#![allow(clippy::module_name_repetitions)]
// This lint triggers when both layer_dir and layers_dir are present which are quite common.
#![allow(clippy::similar_names)]

pub mod build;
pub mod detect;
pub mod exec_d;
pub mod generic;
pub mod layer;
pub mod layer_env;

// Internals that need to be public for macros
#[doc(hidden)]
pub mod internals;

mod buildpack;
mod env;
mod error;
mod platform;
mod runtime;
mod toml_file;
mod util;

pub use buildpack::Buildpack;
pub use env::*;
pub use error::*;
#[doc(inline)]
pub use libcnb_data as data;
pub use platform::*;
pub use runtime::*;
pub use toml_file::*;

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
///         DetectResultBuilder::pass().build()
///     }
///
///     fn build(&self, context: BuildContext<Self>) -> libcnb::Result<BuildResult, Self::Error> {
///         BuildResultBuilder::new().build()
///     }
/// }
///
/// buildpack_main!(MyBuildpack);
/// ```
#[macro_export]
macro_rules! buildpack_main {
    ($buildpack:expr) => {
        fn main() {
            ::libcnb::libcnb_runtime(&$buildpack);
        }
    };
}

/// Resolves the path to an additional buildpack binary by Cargo target name.
///
/// This can be used to copy additional binaries to layers or use them for exec.d.
///
/// To add an additional binary to a buildpack, add a new file with a main function to `bin/`.
/// Cargo will [automatically configure it as a binary target](https://doc.rust-lang.org/cargo/reference/cargo-targets.html#target-auto-discovery)
/// with the name of file.
///
/// **Note**: This only works properly if the buildpack is packaged with `libcnb-cargo`/`libcnb-test`.
///
/// ```no_run,compile_fail
/// use libcnb::additional_buildpack_binary_path;
/// # let layer_dir = std::path::PathBuf::from(".");
///
/// std::fs::copy(
///     // This would not compile in this doctest since there is no `runtime_tool` binary target.
///     additional_buildpack_binary_path!("runtime_tool"),
///     layer_dir.join("runtime_tool"),
/// )
/// .unwrap();
/// ```
#[macro_export]
macro_rules! additional_buildpack_binary_path {
    ($target_name:expr) => {
        ::libcnb::internals::verify_bin_target_exists!(
            $target_name,
            {
                ::std::env::var("CNB_BUILDPACK_DIR")
                    .map(::std::path::PathBuf::from)
                    .expect("Could not read CNB_BUILDPACK_DIR environment variable")
                    .join(".libcnb-cargo")
                    .join("additional-bin")
                    .join($target_name)
            },
            {
                compile_error!(concat!(
                    $target_name,
                    " is not a valid binary target in this buildpack crate!"
                ))
            }
        )
    };
}

// This runs the README.md as a doctest, ensuring the code examples in it are valid.
// It will not be part of the final crate.
#[cfg(doctest)]
#[doc = include_str!("../../README.md")]
pub struct ReadmeDoctests;
