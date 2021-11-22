use std::fs;
use std::io;
use std::path::Path;

use flate2::read::GzDecoder;
use tar::Archive;
use tempfile::NamedTempFile;

use crate::RubyBuildpack;
use libcnb::build::BuildContext;
use libcnb::data::layer_content_metadata::LayerTypes;
use libcnb::generic::GenericMetadata;
use libcnb::layer::{Layer, LayerResult, LayerResultBuilder};
use libcnb::layer_env::{LayerEnv, ModificationBehavior, TargetLifecycle};

pub struct RubyLayer;

impl Layer for RubyLayer {
    type Buildpack = RubyBuildpack;
    type Metadata = GenericMetadata;

    fn types(&self) -> LayerTypes {
        LayerTypes {
            build: true,
            launch: true,
            cache: false,
        }
    }

    fn create(
        &self,
        context: &BuildContext<Self::Buildpack>,
        layer_path: &Path,
    ) -> anyhow::Result<LayerResult<Self::Metadata>> {
        println!("---> Download and extracting Ruby");

        let ruby_tgz = NamedTempFile::new()?;

        download(
            &context.buildpack_descriptor.metadata.ruby_url,
            ruby_tgz.path(),
        )?;

        untar(ruby_tgz.path(), &layer_path)?;

        LayerResultBuilder::new(GenericMetadata::default())
            .env(
                LayerEnv::new()
                    .chainable_insert(
                        TargetLifecycle::All,
                        ModificationBehavior::Prepend,
                        "PATH",
                        context.app_dir.join(".gem/ruby/2.6.6/bin"),
                    )
                    .chainable_insert(
                        TargetLifecycle::All,
                        ModificationBehavior::Prepend,
                        "LD_LIBRARY_PATH",
                        layer_path,
                    ),
            )
            .build()
    }
}

fn download(uri: impl AsRef<str>, dst: impl AsRef<Path>) -> anyhow::Result<()> {
    let response = ureq::get(uri.as_ref()).call()?;
    let mut reader = response.into_reader();
    let mut file = fs::File::create(dst.as_ref())?;
    io::copy(&mut reader, &mut file)?;

    Ok(())
}

fn untar(file: impl AsRef<Path>, dst: impl AsRef<Path>) -> anyhow::Result<()> {
    let tar_gz = fs::File::open(file.as_ref())?;
    let tar = GzDecoder::new(tar_gz);
    let mut archive = Archive::new(tar);
    archive.unpack(dst.as_ref())?;

    Ok(())
}
