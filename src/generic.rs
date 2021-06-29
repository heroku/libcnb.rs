use std::path::Path;

use crate::build::BuildContext;
use crate::detect::DetectContext;
use crate::error::{Error, ErrorHandler};
use crate::platform::{Platform, PlatformEnv};
use std::fmt::{Debug, Display};

/// Generic TOML metadata.
pub type GenericMetadata = Option<toml::value::Table>;

/// A build context for a buildpack that uses a generic platform and metadata.
pub type GenericBuildContext = BuildContext<GenericPlatform, GenericMetadata>;

/// A build detect for a buildpack that uses a generic platform and metadata.
pub type GenericDetectContext = DetectContext<GenericPlatform, GenericMetadata>;

/// Generic output type for layer lifecycles.
pub type GenericLayerLifecycleOutput = ();

/// A generic platform that only provides access to environment variables.
pub struct GenericPlatform {
    env: PlatformEnv,
}

impl Platform for GenericPlatform {
    fn env(&self) -> &PlatformEnv {
        &self.env
    }

    fn from_path(platform_dir: impl AsRef<Path>) -> std::io::Result<Self> {
        Ok(GenericPlatform {
            env: PlatformEnv::from_path(platform_dir)?,
        })
    }
}

/// Generic implementation of [`ErrorHandler`] that logs errors on stderr based on their [`Display`](std::fmt::Display) representation.
pub struct GenericErrorHandler;

impl<E: Debug + Display> ErrorHandler<E> for GenericErrorHandler {
    fn handle_error(&self, error: Error<E>) -> i32 {
        eprintln!("Unhandled error:");
        eprintln!("> {}", error);
        eprintln!("Buildpack will exit!");
        100
    }
}
