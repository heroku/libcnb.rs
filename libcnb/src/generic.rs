//! Generic implementations for some libcnb types.

use crate::platform::Platform;
use crate::{read_platform_env, Env};
use std::fmt::{Debug, Display, Formatter};
use std::path::Path;

/// Generic TOML metadata.
pub type GenericMetadata = Option<toml::value::Table>;

#[derive(Debug)]
pub enum GenericError {}

impl Display for GenericError {
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        f.write_str("GenericError")
    }
}

/// A generic platform that only provides access to environment variables.
pub struct GenericPlatform {
    env: Env,
}

impl GenericPlatform {
    #[must_use]
    pub fn new(env: Env) -> Self {
        Self { env }
    }
}

impl Platform for GenericPlatform {
    fn env(&self) -> &Env {
        &self.env
    }

    fn from_path(platform_dir: impl AsRef<Path>) -> std::io::Result<Self> {
        read_platform_env(platform_dir.as_ref()).map(|env| GenericPlatform { env })
    }
}
