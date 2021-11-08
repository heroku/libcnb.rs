use std::collections::HashMap;
use std::fs;
use std::io;
use std::path::Path;

use anyhow::Error;
use flate2::read::GzDecoder;
use libcnb::data::layer_content_metadata::LayerContentMetadata;
use libcnb::layer_lifecycle::LayerLifecycle;
use libcnb::{BuildContext, GenericMetadata, GenericPlatform};

use std::env;
use tar::Archive;
use tempfile::NamedTempFile;

use crate::RubyBuildpackMetadata;

pub struct RubyLayerLifecycle;

impl
    LayerLifecycle<
        GenericPlatform,
        RubyBuildpackMetadata,
        GenericMetadata,
        HashMap<String, String>,
        anyhow::Error,
    > for RubyLayerLifecycle
{
    fn create(
        &self,
        layer_path: &Path,
        build_context: &BuildContext<GenericPlatform, RubyBuildpackMetadata>,
    ) -> Result<LayerContentMetadata<GenericMetadata>, anyhow::Error> {
        let ruby_tgz = NamedTempFile::new()?;
        download(
            &build_context.buildpack_descriptor.metadata.ruby_url,
            ruby_tgz.path(),
        )?;
        untar(ruby_tgz.path(), &layer_path)?;

        Ok(LayerContentMetadata::default()
            .metadata(GenericMetadata::default())
            .launch(true))
    }

    fn layer_lifecycle_data(
        &self,
        layer_path: &Path,
        _layer_content_metadata: LayerContentMetadata<GenericMetadata>,
    ) -> Result<HashMap<String, String>, Error> {
        let mut ruby_env: HashMap<String, String> = HashMap::new();
        let ruby_bin_path = format!(
            "{}/.gem/ruby/2.6.6/bin",
            env::var("HOME").unwrap_or_default()
        );

        ruby_env.insert(
            String::from("PATH"),
            format!(
                "{}:{}:{}",
                layer_path.join("bin").as_path().to_str().unwrap(),
                ruby_bin_path,
                env::var("PATH").unwrap_or_default(),
            ),
        );

        ruby_env.insert(
            String::from("LD_LIBRARY_PATH"),
            format!(
                "{}:{}",
                env::var("LD_LIBRARY_PATH").unwrap_or_default(),
                layer_path.join("layer").as_path().to_str().unwrap()
            ),
        );

        Ok(ruby_env)
    }
}

fn download(uri: impl AsRef<str>, dst: impl AsRef<Path>) -> anyhow::Result<()> {
    let response = reqwest::blocking::get(uri.as_ref())?;
    let mut content = io::Cursor::new(response.bytes()?);
    let mut file = fs::File::create(dst.as_ref())?;
    io::copy(&mut content, &mut file)?;

    Ok(())
}

fn untar(file: impl AsRef<Path>, dst: impl AsRef<Path>) -> anyhow::Result<()> {
    let tar_gz = fs::File::open(file.as_ref())?;
    let tar = GzDecoder::new(tar_gz);
    let mut archive = Archive::new(tar);
    archive.unpack(dst.as_ref())?;

    Ok(())
}
