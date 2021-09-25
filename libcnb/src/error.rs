use crate::data::launch::ProcessTypeError;
use crate::layer_lifecycle::LayerLifecycleError;
use crate::toml_file::TomlFileError;
use std::fmt::{Debug, Display};

/// Handles top-level buildpack errors.
pub trait ErrorHandler<E: Debug + Display> {
    fn handle_error(&self, error: Error<E>) -> i32;
}

/// A specialized Result type for libcnb.
///
/// This type is broadly used across libcnb for any operation which may produce an error.
pub type Result<T, E> = std::result::Result<T, Error<E>>;

/// An error that occurred during buildpack execution.
#[derive(thiserror::Error, Debug)]
pub enum Error<E: Debug + Display> {
    #[error("libcnb error: {0}")]
    LibError(LibError),
    #[error("data format error: {0}")]
    DataError(DataError),
    #[error("Buildpack error: {0}")]
    BuildpackError(E),
}

#[cfg(feature = "anyhow")]
impl From<anyhow::Error> for Error<anyhow::Error> {
    fn from(error: anyhow::Error) -> Self {
        Error::BuildpackError(error)
    }
}

impl<E: Debug + Display> From<LibError> for Error<E> {
    fn from(error: LibError) -> Self {
        Self::LibError(error)
    }
}

impl<E: Debug + Display> From<LayerLifecycleError> for Error<E> {
    fn from(error: LayerLifecycleError) -> Self {
        Self::LibError(LibError::LayerLifecycleError(error))
    }
}

/// An error that occurred from libcnb
#[derive(thiserror::Error, Debug)]
pub enum LibError {
    #[error("Layer lifecycle error: {0}")]
    LayerLifecycleError(#[from] LayerLifecycleError),

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
}

#[derive(thiserror::Error, Debug)]
pub enum DataError {
    #[error("Process type error: {0}")]
    ProcessTypeError(#[from] ProcessTypeError),
}
