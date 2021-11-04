use std::{fs, path::PathBuf};

use serde::de::DeserializeOwned;
use serde::Serialize;

use crate::data::stack_id::StackId;
use crate::{
    data::{
        buildpack::BuildpackToml, buildpack_plan::BuildpackPlan, launch::Launch,
        layer_content_metadata::LayerContentMetadata,
    },
    platform::Platform,
    toml_file::{read_toml_file, write_toml_file, TomlFileError},
};

/// Context for a buildpack's build phase execution.
pub struct BuildContext<P: Platform, BM> {
    pub layers_dir: PathBuf,
    pub app_dir: PathBuf,
    pub buildpack_dir: PathBuf,
    pub stack_id: StackId,
    pub platform: P,
    pub buildpack_plan: BuildpackPlan,
    pub buildpack_descriptor: BuildpackToml<BM>,
}

impl<P: Platform, BM> BuildContext<P, BM> {
    pub fn layer_path(&self, layer_name: impl AsRef<str>) -> PathBuf {
        self.layers_dir.join(layer_name.as_ref())
    }

    pub fn layer_content_metadata_path(&self, layer_name: impl AsRef<str>) -> PathBuf {
        self.layers_dir
            .join(format!("{}.toml", layer_name.as_ref()))
    }

    pub fn read_layer_content_metadata<M: DeserializeOwned>(
        &self,
        layer_name: impl AsRef<str>,
    ) -> Result<Option<LayerContentMetadata<M>>, TomlFileError> {
        let path = self.layer_content_metadata_path(layer_name);

        if path.exists() {
            read_toml_file(path).map(Some)
        } else {
            Ok(None)
        }
    }

    pub fn write_layer_content_metadata<M: Serialize>(
        &self,
        layer_name: impl AsRef<str>,
        layer_content_metadata: &LayerContentMetadata<M>,
    ) -> Result<(), TomlFileError> {
        write_toml_file(
            layer_content_metadata,
            self.layer_content_metadata_path(layer_name),
        )
    }

    pub fn delete_layer(&self, layer_name: impl AsRef<str>) -> Result<(), std::io::Error> {
        // Do not fail if the metadata file does not exist
        match fs::remove_file(self.layer_content_metadata_path(&layer_name)) {
            Err(io_error) => match io_error.kind() {
                std::io::ErrorKind::NotFound => Ok(()),
                _ => Err(io_error),
            },
            Ok(_) => Ok(()),
        }?;

        match fs::remove_dir_all(self.layer_path(&layer_name)) {
            Err(io_error) => match io_error.kind() {
                std::io::ErrorKind::NotFound => Ok(()),
                _ => Err(io_error),
            },
            Ok(_) => Ok(()),
        }?;

        Ok(())
    }

    pub fn read_layer<M: DeserializeOwned>(
        &self,
        layer_name: impl AsRef<str>,
    ) -> Result<Option<(PathBuf, LayerContentMetadata<M>)>, TomlFileError> {
        let layer_path = self.layer_path(&layer_name);

        self.read_layer_content_metadata(&layer_name)
            .map(|maybe_content_layer_metadata| {
                maybe_content_layer_metadata.and_then(
                    |layer_content_metadata: LayerContentMetadata<M>| {
                        if layer_path.exists() {
                            Some((layer_path, layer_content_metadata))
                        } else {
                            None
                        }
                    },
                )
            })
    }

    pub fn layer_exists(&self, layer_name: impl AsRef<str>) -> bool {
        let layer_path = self.layer_path(&layer_name);
        let content_metadata_path = self.layer_content_metadata_path(&layer_name);
        layer_path.exists() && content_metadata_path.exists()
    }

    pub fn write_launch(&self, data: Launch) -> Result<(), TomlFileError> {
        write_toml_file(&data, self.layers_dir.join("launch.toml"))
    }
}
