use crate::data::launch::ProcessTypeError;
use crate::layer_lifecycle::LayerLifecycleError;
use crate::shared::TomlFileError;
use std::error::Error;

pub trait LibCnbErrorHandle<E: Error> {
    fn handle_error(&self, error: LibCnbError<E>) -> i32;
}

#[derive(thiserror::Error, Debug)]
pub enum LibCnbError<E: Error> {
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

    #[error("Buildpack error: {0}")]
    BuildpackError(E),
}
