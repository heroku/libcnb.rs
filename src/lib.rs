use std::error::Error;
use std::fmt::Debug;

use thiserror;

use crate::data::launch::ProcessTypeError;
use crate::layer_lifecycle::LayerLifecycleError;

pub mod build;
pub mod data;
pub mod detect;
pub mod error;
pub mod generic;
pub mod layer_lifecycle;
pub mod platform;
pub mod runtime;
pub mod shared;

#[derive(thiserror::Error, Debug)]
pub enum LibCnbError<E: Error> {
    #[error("Layer lifecycle error: {0}")]
    LayerLifecycleError(#[from] LayerLifecycleError),

    #[error("Process type error: {0}")]
    ProcessTypeError(#[from] ProcessTypeError),

    #[error("Buildpack error: {0}")]
    BuildpackError(E),
}
