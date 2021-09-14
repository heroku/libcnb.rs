use std::collections::HashMap;
use std::fs;
use std::path::Path;
use std::process::Command;

use anyhow::Error;
use libcnb::{BuildContext, GenericPlatform};
use libcnb::data::layer_content_metadata::LayerContentMetadata;
use libcnb::layer_lifecycle::{LayerLifecycle, ValidateResult};
use serde::Deserialize;
use serde::Serialize;
use sha2::Digest;

use crate::RubyBuildpackMetadata;

pub struct BundlerLayerLifecycle {
    pub ruby_env: HashMap<String, String>,
}

#[derive(Deserialize, Serialize, Debug)]
pub struct BundlerLayerMetadata {
    checksum: String,
}

impl LayerLifecycle<GenericPlatform, RubyBuildpackMetadata, BundlerLayerMetadata, Option<()>, anyhow::Error> for BundlerLayerLifecycle {
    fn validate(&self, layer_path: &Path, layer_content_metadata: &LayerContentMetadata<BundlerLayerMetadata>, build_context: &BuildContext<GenericPlatform, RubyBuildpackMetadata>) -> ValidateResult {
        let checksum_matches = sha256_checksum(build_context.app_dir.join("Gemfile.lock"))
            .map(|local_checksum| local_checksum == layer_content_metadata.metadata.checksum)
            .unwrap_or(false);

        if checksum_matches {
            ValidateResult::KeepLayer
        } else {
            ValidateResult::UpdateLayer
        }
    }

    fn update(&self, layer_path: &Path, layer_content_metadata: LayerContentMetadata<BundlerLayerMetadata>, build_context: &BuildContext<GenericPlatform, RubyBuildpackMetadata>) -> Result<LayerContentMetadata<BundlerLayerMetadata>, Error> {
        println!("---> Reusing gems");
        Command::new("bundle")
            .args(&[
                "config",
                "--local",
                "path",
                layer_path.to_str().unwrap(),
            ])
            .envs(&self.ruby_env)
            .spawn()?
            .wait()?;

        Command::new("bundle")
            .args(&[
                "config",
                "--local",
                "bin",
                layer_path.join("bin").as_path().to_str().unwrap(),
            ])
            .envs(&self.ruby_env)
            .spawn()?
            .wait()?;

        Ok(layer_content_metadata)
    }

    fn create(&self, layer_path: &Path, build_context: &BuildContext<GenericPlatform, RubyBuildpackMetadata>) -> Result<LayerContentMetadata<BundlerLayerMetadata>, Error> {
        println!("---> Installing gems");

        let cmd = Command::new("bundle")
            .args(&[
                "install",
                "--path",
                layer_path.to_str().unwrap(),
                "--binstubs",
                layer_path.join("bin").to_str().unwrap(),
            ])
            .envs(&self.ruby_env)
            .spawn()?
            .wait()?;
        if !cmd.success() {
            anyhow::anyhow!("Could not bundle install");
        }

        Ok(LayerContentMetadata::default().launch(true).cache(true).metadata(BundlerLayerMetadata {
            checksum: sha256_checksum(build_context.app_dir.join("Gemfile.lock"))?
        }))
    }
}

fn sha256_checksum(path: impl AsRef<Path>) -> anyhow::Result<String> {
    Ok(fs::read(path)
        .map(|bytes| sha2::Sha256::digest(bytes.as_ref()))
        .map(|bytes| format!("{:x}", bytes))?)
}
