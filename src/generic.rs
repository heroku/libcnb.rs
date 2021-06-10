use std::path::Path;

use crate::build::BuildContext;
use crate::detect::DetectContext;
use crate::layer_lifecycle::LayerLifecycle;
use crate::platform::{Platform, PlatformEnv};

pub type GenericMetadata = Option<toml::value::Table>;

pub type GenericBuildContext = BuildContext<GenericPlatform, GenericMetadata>;

pub type GenericDetectContext = DetectContext<GenericPlatform, GenericMetadata>;

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
