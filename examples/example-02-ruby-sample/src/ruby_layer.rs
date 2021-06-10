use std::path::Path;

use libcnb::build::BuildContext;
use libcnb::data::layer::LayerContentMetadata;
use libcnb::generic::{GenericLayerLifecycle, GenericMetadata, GenericPlatform};
use libcnb::layer_lifecycle::LayerLifecycle;
use tempfile::NamedTempFile;

pub struct RubyLayer {}

impl LayerLifecycle<GenericPlatform, GenericMetadata, GenericMetadata, std::io::Error>
    for RubyLayer
{
    fn create(
        &self,
        path: &Path,
        context: &BuildContext<GenericPlatform, GenericMetadata>,
    ) -> Result<LayerContentMetadata<GenericMetadata>, Error> {
        let ruby_tgz = NamedTempFile::new()?;
        download(RUBY_URL, ruby_tgz.path())?;
        untar(ruby_tgz.path(), &ruby_layer)?;

        Ok(LayerContentMetadata::default().launch(true))
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
