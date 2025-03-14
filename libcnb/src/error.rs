use crate::data::launch::ProcessTypeError;
use crate::layer::LayerError;
use libcnb_common::toml_file::TomlFileError;
use std::fmt::Debug;

/// A specialized Result type for libcnb.
///
/// This type is broadly used across libcnb for any operation which may produce an error.
pub type Result<T, E> = std::result::Result<T, Error<E>>;

/// An error that occurred during buildpack execution.
#[derive(thiserror::Error, Debug)]
pub enum Error<E> {
    #[error("Layer error: {0}")]
    LayerError(#[from] LayerError),

    #[error("Process type error: {0}")]
    ProcessTypeError(#[from] ProcessTypeError),

    #[error("Couldn't determine app directory: {0}")]
    CannotDetermineAppDirectory(std::io::Error),

    #[error("Couldn't determine buildpack directory: {0}")]
    CannotDetermineBuildpackDirectory(std::env::VarError),

    #[error("Couldn't determine target os: {0}")]
    CannotDetermineTargetOs(std::env::VarError),

    #[error("Couldn't determine target arch: {0}")]
    CannotDetermineTargetArch(std::env::VarError),

    #[error(
        "Couldn't determine target distro name: {0}. Ensure the `io.buildpacks.base.distro.*` Docker labels are set on the base image."
    )]
    CannotDetermineTargetDistroName(std::env::VarError),

    #[error(
        "Couldn't determine target distro version: {0}. Ensure the `io.buildpacks.base.distro.*` Docker labels are set on the base image."
    )]
    CannotDetermineTargetDistroVersion(std::env::VarError),

    #[error("Couldn't create platform from platform path: {0}")]
    CannotCreatePlatformFromPath(std::io::Error),

    #[error("Couldn't read buildpack plan: {0}")]
    CannotReadBuildpackPlan(TomlFileError),

    #[error("Couldn't read buildpack.toml: {0}")]
    CannotReadBuildpackDescriptor(TomlFileError),

    #[error("Couldn't read store.toml: {0}")]
    CannotReadStore(TomlFileError),

    #[error("Couldn't write build plan: {0}")]
    CannotWriteBuildPlan(TomlFileError),

    #[error("Couldn't write launch.toml: {0}")]
    CannotWriteLaunch(TomlFileError),

    #[error("Couldn't write store.toml: {0}")]
    CannotWriteStore(TomlFileError),

    #[error("Couldn't write build SBOM files: {0}")]
    CannotWriteBuildSbom(std::io::Error),

    #[error("Couldn't write launch SBOM files: {0}")]
    CannotWriteLaunchSbom(std::io::Error),

    #[error("Buildpack error: {0:?}")]
    BuildpackError(E),
}

#[cfg(feature = "anyhow")]
impl From<anyhow::Error> for Error<anyhow::Error> {
    fn from(error: anyhow::Error) -> Self {
        Self::BuildpackError(error)
    }
}
