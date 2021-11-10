use libcnb::data::layer_content_metadata::LayerTypes;
use serde::Deserialize;
use serde::Serialize;
use sha2::Digest;
use std::fs;
use std::path::Path;
use std::process::Command;

use crate::RubyBuildpack;
use libcnb::build::BuildContext;
use libcnb::{Env, Layer, LayerData, LayerResult, LayerResultBuilder};

#[derive(Deserialize, Serialize, Debug, Clone)]
pub struct BundlerLayerMetadata {
    gemfile_lock_checksum: String,
}

pub struct BundlerLayer {
    pub ruby_env: Env,
}

impl Layer for BundlerLayer {
    type Buildpack = RubyBuildpack;
    type Metadata = BundlerLayerMetadata;

    fn types(&self) -> LayerTypes {
        LayerTypes {
            build: false,
            launch: true,
            cache: true,
        }
    }

    fn create(
        &self,
        context: &BuildContext<Self::Buildpack>,
        layer_path: &Path,
    ) -> anyhow::Result<LayerResult<Self::Metadata>> {
        println!("---> Installing gems");

        let bundle_exit_code = Command::new("bundle")
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

        if !bundle_exit_code.success() {
            return Err(anyhow::anyhow!("Could not bundle install"));
        }

        LayerResultBuilder::new(BundlerLayerMetadata {
            gemfile_lock_checksum: sha256_checksum(context.app_dir.join("Gemfile.lock"))?,
        })
        .build()
    }

    fn should_be_updated(
        &self,
        context: &BuildContext<Self::Buildpack>,
        layer: &LayerData<Self::Metadata>,
    ) -> anyhow::Result<bool> {
        sha256_checksum(context.app_dir.join("Gemfile.lock"))
            .map(|checksum| checksum != layer.content_metadata.metadata.gemfile_lock_checksum)
    }

    fn update(
        &self,
        context: &BuildContext<Self::Buildpack>,
        layer: &LayerData<Self::Metadata>,
    ) -> anyhow::Result<LayerResult<Self::Metadata>> {
        println!("---> Reusing gems");

        Command::new("bundle")
            .args(&["config", "--local", "path", layer.path.to_str().unwrap()])
            .envs(&self.ruby_env)
            .spawn()?
            .wait()?;

        Command::new("bundle")
            .args(&[
                "config",
                "--local",
                "bin",
                layer.path.join("bin").as_path().to_str().unwrap(),
            ])
            .envs(&self.ruby_env)
            .spawn()?
            .wait()?;

        LayerResultBuilder::new(BundlerLayerMetadata {
            gemfile_lock_checksum: sha256_checksum(context.app_dir.join("Gemfile.lock"))?,
        })
        .build()
    }
}

fn sha256_checksum(path: impl AsRef<Path>) -> anyhow::Result<String> {
    Ok(fs::read(path)
        .map(|bytes| sha2::Sha256::digest(bytes.as_ref()))
        .map(|bytes| format!("{:x}", bytes))?)
}
