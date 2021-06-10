use core::num::dec2flt::parse::ParseResult::Valid;
use libcnb::build::BuildContext;
use libcnb::data::layer::LayerContentMetadata;
use libcnb::generic::{GenericLayerLifecycle, GenericMetadata, GenericPlatform};
use libcnb::layer_lifecycle::{LayerLifecycle, ValidateResult};
use std::path::Path;

struct BundlerLayer {}

struct BundlerLayerMetadata {
    gemfile_checksum: String,
}

impl LayerLifecycle<GenericPlatform, GenericMetadata, BundlerLayerMetadata, std::io::Error>
    for BundlerLayer
{
    fn validate(
        &self,
        path: &Path,
        layer_content_metadata: &LayerContentMetadata<BundlerLayerMetadata>,
        build_context: &BuildContext<GenericPlatform, GenericMetadata>,
    ) -> ValidateResult {
        let local_gemfile_checksum = sha256_checksum(context.app_dir.join("Gemfile.lock"))?;
        if local_gemfile_checksum == layer_content_metadata.metadata.gemfile_checksum {
            ValidateResult::UpdateLayer
        } else {
            ValidateResult::UpdateLayer
        }
    }

    fn update(
        &self,
        path: &Path,
        layer_content_metadata: LayerContentMetadata<BundlerLayerMetadata>,
        build_context: &BuildContext<GenericPlatform, GenericMetadata>,
    ) -> Result<LayerContentMetadata<GenericMetadata>, Error> {
        todo!()
    }

    fn create(
        &self,
        path: &Path,
        context: &BuildContext<GenericPlatform, BundlerLayerMetadata>,
    ) -> Result<LayerContentMetadata<GenericMetadata>, Error> {
        Ok(LayerContentMetadata::default().launch(true).cache(true))
    }
}

fn install_bundler() {
    println!("---> Installing bundler");
    {
        let cmd = Command::new("gem")
            .args(&["install", "bundler", "--no-ri", "--no-rdoc"])
            .envs(&ruby_env)
            .spawn()?
            .wait()?;

        if !cmd.success() {
            anyhow::anyhow!("Could not install bundler");
        }
    }
}

fn sha256_checksum(path: impl AsRef<Path>) -> io::Result<String> {
    fs::read(path)
        .map(|bytes| sha2::Sha256::digest(&bytes))
        .map(|bytes| format!("{:x}", bytes))
}
