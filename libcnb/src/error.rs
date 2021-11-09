use crate::data::launch::ProcessTypeError;
use crate::layer_lifecycle::LayerLifecycleError;
use crate::toml_file::TomlFileError;
use std::fmt::{Debug, Display};

/// A specialized Result type for libcnb.
///
/// This type is broadly used across libcnb for any operation which may produce an error.
pub type Result<T, E> = std::result::Result<T, Error<E>>;

/// An error that occurred during buildpack execution.
#[derive(thiserror::Error, Debug)]
pub enum Error<E: Debug + Display> {
    #[error("Layer lifecycle error: {0}")]
    LayerLifecycleError(#[from] LayerLifecycleError),

    #[error("Process type error: {0}")]
    ProcessTypeError(#[from] ProcessTypeError),

    #[error("Could not determine app directory: {0}")]
    CannotDetermineAppDirectory(std::io::Error),

    #[error("Could not determine buildpack directory: {0}")]
    CannotDetermineBuildpackDirectory(std::env::VarError),

    #[error("Could not determine stack id: {0}")]
    CannotDetermineStackId(std::env::VarError),

    #[error("Cannot create platform from platform path: {0}")]
    CannotCreatePlatformFromPath(std::io::Error),

    #[error("Cannot read buildpack plan: {0}")]
    CannotReadBuildpackPlan(TomlFileError),

    #[error("Cannot read buildpack descriptor (buildpack.toml): {0}")]
    CannotReadBuildpackDescriptor(TomlFileError),

    #[error("Cannot write build plan: {0}")]
    CannotWriteBuildPlan(TomlFileError),

    #[error("Cannot write launch.toml: {0}")]
    CannotWriteLaunch(TomlFileError),

    #[error("Cannot write store.toml: {0}")]
    CannotWriteStore(TomlFileError),

    #[error("Buildpack error: {0}")]
    BuildpackError(E),
}

#[cfg(feature = "anyhow")]
impl From<anyhow::Error> for Error<anyhow::Error> {
    fn from(error: anyhow::Error) -> Self {
        Error::BuildpackError(error)
    }
}
