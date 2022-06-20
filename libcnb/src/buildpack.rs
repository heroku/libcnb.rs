use crate::build::{BuildContext, BuildResult};
use crate::detect::{DetectContext, DetectResult};
use crate::Platform;
use serde::de::DeserializeOwned;
use std::fmt::Debug;

/// Represents a buildpack written with the libcnb framework.
///
/// To implement a buildpack with this framework, start by implementing this trait. Besides the main
/// build and detect methods, it also holds associated types for the buildpack: the [`Platform`] it
/// is targeting, the type for its metadata and the custom error type.
pub trait Buildpack {
    /// The platform targeted by this buildpack. If no specific platform is targeted, consider using
    /// [`GenericPlatform`](crate::generic::GenericPlatform) as the type.
    type Platform: Platform;

    /// The metadata type for this buildpack. This is the data within `[metadata]` of the buildpacks
    /// `buildpack.toml`. The framework will attempt to parse the data and will only continue if
    /// parsing succeeded. If you wish to use raw, untyped, TOML data instead, use
    /// [`GenericMetadata`](crate::generic::GenericMetadata).
    type Metadata: DeserializeOwned;

    /// The error type for buildpack specific errors, usually an enum. Examples of values inside the
    /// enum are: `MavenExecutionFailed`, `InvalidGemfileLock`, `IncompatiblePythonVersion`. The
    /// framework itself has its [own error type](crate::error::Error) that contains more low-level errors that can occur
    /// during buildpack execution.
    type Error: Debug;

    /// Detect logic for this buildpack. Directly corresponds to
    /// [detect in the CNB buildpack interface](https://github.com/buildpacks/spec/blob/platform/v0.6/buildpack.md#detection).
    fn detect(&self, context: DetectContext<Self>) -> crate::Result<DetectResult, Self::Error>;

    /// Build logic for this buildpack. Directly corresponds to
    /// [build in the CNB buildpack interface](https://github.com/buildpacks/spec/blob/platform/v0.6/buildpack.md#build).
    fn build(&self, context: BuildContext<Self>) -> crate::Result<BuildResult, Self::Error>;

    /// If an unhandled error occurred within the framework or the buildpack, this method will be
    /// called by the framework to allow custom, buildpack specific, code to run before exiting.
    /// Usually, this method is implemented by logging the error in a user friendly manner.
    ///
    /// Implementations are not limited to just logging, for example, buildpacks might want to
    /// collect and send metrics about occurring errors to a central system.
    ///
    /// The default implementation will simply print the error
    /// (using its [`Debug`](std::fmt::Debug) implementation) to stderr.
    fn on_error(&self, error: crate::Error<Self::Error>) {
        eprintln!("Unhandled error:");
        eprintln!("> {:?}", error);
        eprintln!("Buildpack will exit!");
    }
}
