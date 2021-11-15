//! Generic implementations for some libcnb types.

use std::path::Path;

use crate::platform::{Platform, PlatformEnv};
use std::fmt::{Debug, Display, Formatter};

/// Generic TOML metadata.
pub type GenericMetadata = Option<toml::value::Table>;

/// Generic output type for layer lifecycles.
pub type GenericLayerLifecycleOutput = ();

#[derive(Debug)]
pub enum GenericError {}

impl Display for GenericError {
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        f.write_str("GenericError")
    }
}

/// A generic platform that only provides access to environment variables.
pub struct GenericPlatform {
    env: PlatformEnv,
}

impl Platform for GenericPlatform {
    fn env(&self) -> &PlatformEnv {
        &self.env
    }

    fn from_path(platform_dir: impl AsRef<Path>) -> std::io::Result<Self> {
        Ok(Self {
            env: PlatformEnv::from_path(platform_dir)?,
        })
    }
}
