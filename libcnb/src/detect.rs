//! Provides detect phase specific types and helpers.

use crate::buildpack::Buildpack;
use crate::{data::build_plan::BuildPlan, data::buildpack::ComponentBuildpackDescriptor};
use std::fmt::Debug;
use std::path::PathBuf;

/// Context for the detect phase execution.
pub struct DetectContext<B: Buildpack + ?Sized> {
    pub app_dir: PathBuf,
    pub buildpack_dir: PathBuf,
    /// The name of the target operating system.
    ///
    /// The value should conform to [Go's `$GOOS`](https://golang.org/doc/install/source#environment), for example
    /// `linux` or `windows`.
    ///
    /// CNB `lifecycle` sources this value from the build OCI image's [`os` property](https://github.com/opencontainers/image-spec/blob/main/config.md#properties).
    pub target_os: String,
    /// The name of the target CPU architecture.
    ///
    /// The value should conform to [Go's $GOARCH](https://golang.org/doc/install/source#environment), for example
    /// `amd64` or `arm64`.
    ///
    /// CNB `lifecycle` sources this value from the build OCI image's [`architecture` property](https://github.com/opencontainers/image-spec/blob/main/config.md#properties).
    /// ``
    pub target_arch: String,
    /// The variant of the specified CPU architecture.
    ///
    /// The value should conform to [OCI image spec platform variants](https://github.com/opencontainers/image-spec/blob/main/image-index.md#platform-variants), for example
    /// `v7` or `v8`.
    ///
    /// CNB `lifecycle` sources this value from the build OCI image's [`variant` property](https://github.com/opencontainers/image-spec/blob/main/config.md#properties).
    pub target_arch_variant: Option<String>,
    /// The name of the operating system distribution. Should be empty for windows.
    ///
    /// For example: `ubuntu` or `arch`.
    ///
    /// CNB `lifecycle` sources this value from the build OCI image's `io.buildpacks.distro.name` label.
    pub target_distro_name: Option<String>,
    /// The version of the operating system distribution.
    ///
    /// For example: `18.02` or `2024.02.01`.
    ///
    /// CNB `lifecycle` sources this value from the build OCI image's `io.buildpacks.distro.version` label.
    pub target_distro_version: Option<String>,
    pub platform: B::Platform,
    pub buildpack_descriptor: ComponentBuildpackDescriptor<B::Metadata>,
}

/// Describes the result of the detect phase.
///
/// Besides indicating passing or failing detection, it also contains detect phase output such as
/// the build plan. To construct values of this type, use a [`DetectResultBuilder`].
#[derive(Debug)]
#[must_use]
pub struct DetectResult(pub(crate) InnerDetectResult);

#[derive(Debug)]
pub(crate) enum InnerDetectResult {
    Fail,
    Pass { build_plan: Option<BuildPlan> },
}

/// Constructs [`DetectResult`] values.
///
/// # Examples:
/// ```
/// use libcnb::detect::{DetectResult, DetectResultBuilder};
/// use libcnb_data::build_plan::BuildPlanBuilder;
///
/// let simple_pass: Result<DetectResult, ()> = DetectResultBuilder::pass().build();
/// let simple_fail: Result<DetectResult, ()> = DetectResultBuilder::fail().build();
///
/// let with_build_plan: Result<DetectResult, ()> = DetectResultBuilder::pass()
///     .build_plan(BuildPlanBuilder::new().provides("something").build())
///     .build();
/// ```
#[must_use]
pub struct DetectResultBuilder;

impl DetectResultBuilder {
    pub fn pass() -> PassDetectResultBuilder {
        PassDetectResultBuilder { build_plan: None }
    }

    pub fn fail() -> FailDetectResultBuilder {
        FailDetectResultBuilder {}
    }
}

/// Constructs [`DetectResult`] values for a passed detection. Can't be used directly, use
/// a [`DetectResultBuilder`] to create an instance.
#[must_use]
pub struct PassDetectResultBuilder {
    build_plan: Option<BuildPlan>,
}

impl PassDetectResultBuilder {
    /// Builds the final [`DetectResult`].
    ///
    /// This method returns the [`DetectResult`] wrapped in a [`Result`] even though its technically
    /// not fallible. This is done to simplify using this method in the context it's most often used
    /// in: a buildpack's [detect method](crate::Buildpack::detect).
    ///
    /// See [`build_unwrapped`](Self::build_unwrapped) for an unwrapped version of this method.
    pub fn build<E>(self) -> Result<DetectResult, E> {
        Ok(self.build_unwrapped())
    }

    pub fn build_unwrapped(self) -> DetectResult {
        DetectResult(InnerDetectResult::Pass {
            build_plan: self.build_plan,
        })
    }

    pub fn build_plan(mut self, build_plan: BuildPlan) -> Self {
        self.build_plan = Some(build_plan);
        self
    }
}

/// Constructs [`DetectResult`] values for a failed detection. Can't be used directly, use
/// a [`DetectResultBuilder`] to create an instance.
#[must_use]
pub struct FailDetectResultBuilder;

impl FailDetectResultBuilder {
    /// Builds the final [`DetectResult`].
    ///
    /// This method returns the [`DetectResult`] wrapped in a [`Result`] even though its technically
    /// not fallible. This is done to simplify using this method in the context it's most often used
    /// in: a buildpack's [detect method](crate::Buildpack::detect).
    ///
    /// See [`build_unwrapped`](Self::build_unwrapped) for an unwrapped version of this method.
    pub fn build<E>(self) -> Result<DetectResult, E> {
        Ok(self.build_unwrapped())
    }

    #[allow(clippy::unused_self)]
    pub fn build_unwrapped(self) -> DetectResult {
        DetectResult(InnerDetectResult::Fail)
    }
}
