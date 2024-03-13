// This lint triggers when both layer_dir and layers_dir are present which are quite common.
#![allow(clippy::similar_names)]

use crate::util::{default_on_not_found, remove_dir_recursively};
use libcnb_common::toml_file::{read_toml_file, write_toml_file, TomlFileError};
use libcnb_data::layer::LayerName;
use libcnb_data::layer_content_metadata::{LayerContentMetadata, LayerTypes};
use serde::de::DeserializeOwned;
use serde::Serialize;
use std::fs;
use std::path::{Path, PathBuf};

pub(in crate::layer) fn read_layer<M: DeserializeOwned, P: AsRef<Path>>(
    layers_dir: P,
    layer_name: &LayerName,
) -> Result<Option<ReadLayer<M>>, ReadLayerError> {
    let layer_dir_path = layers_dir.as_ref().join(layer_name.as_str());
    let layer_toml_path = layers_dir.as_ref().join(format!("{layer_name}.toml"));

    if !layer_dir_path.exists() && !layer_toml_path.exists() {
        return Ok(None);
    } else if !layer_dir_path.exists() && layer_toml_path.exists() {
        // This is a valid case according to the spec:
        // https://github.com/buildpacks/spec/blob/7b20dfa070ed428c013e61a3cefea29030af1732/buildpack.md#layer-types
        //
        // When launch = true, build = false, cache = false, the layer metadata will be restored but
        // not the layer itself. However, we choose to not support this case as of now. It would
        // complicate the API we need to expose to the user of libcnb as this case is very different
        // compared to all other combinations of launch, build and cache. It's the only case where
        // a cache = false layer restores some of its data between builds.
        //
        // To normalize, we remove the layer TOML file and treat the layer as non-existent.
        fs::remove_file(&layer_toml_path)?;
        return Ok(None);
    }

    // An empty layer content metadata file is valid and the CNB spec is not clear if the lifecycle
    // has to restore them if they're empty. This is especially important since the layer types
    // are removed from the file if it's restored. To normalize, we write an empty file if the layer
    // directory exists without the metadata file.
    if !layer_toml_path.exists() {
        fs::write(&layer_toml_path, "")?;
    }

    let layer_toml_contents = fs::read_to_string(&layer_toml_path)?;

    let layer_content_metadata = toml::from_str::<LayerContentMetadata<M>>(&layer_toml_contents)
        .map_err(ReadLayerError::LayerContentMetadataParseError)?;

    Ok(Some(ReadLayer {
        name: layer_name.clone(),
        path: layer_dir_path,
        metadata: layer_content_metadata,
    }))
}

pub(in crate::layer) struct ReadLayer<M> {
    pub(in crate::layer) name: LayerName,
    pub(in crate::layer) path: PathBuf,
    pub(in crate::layer) metadata: LayerContentMetadata<M>,
}

#[derive(thiserror::Error, Debug)]
pub enum ReadLayerError {
    #[error("Layer content metadata couldn't be parsed!")]
    LayerContentMetadataParseError(toml::de::Error),

    #[error("Unexpected I/O error while reading layer: {0}")]
    IoError(#[from] std::io::Error),
}

/// Updates layer metadata on disk
pub(in crate::layer) fn write_layer<M: Serialize, P: AsRef<Path>>(
    layers_dir: P,
    layer_name: &LayerName,
    layer_content_metadata: &LayerContentMetadata<M>,
) -> Result<(), WriteLayerError> {
    let layers_dir = layers_dir.as_ref();
    fs::create_dir_all(layers_dir.join(layer_name.as_str()))?;
    replace_layer_metadata(layers_dir, layer_name, layer_content_metadata)?;

    Ok(())
}

#[derive(thiserror::Error, Debug)]
pub enum WriteLayerError {
    #[error("Layer content metadata couldn't be parsed!")]
    WriteLayerMetadataError(WriteLayerMetadataError),

    #[error("Unexpected I/O error while writing layer: {0}")]
    IoError(#[from] std::io::Error),
}

/// Does not error if the layer doesn't exist.
pub(in crate::layer) fn delete_layer<P: AsRef<Path>>(
    layers_dir: P,
    layer_name: &LayerName,
) -> Result<(), DeleteLayerError> {
    let layer_dir = layers_dir.as_ref().join(layer_name.as_str());
    let layer_toml = layers_dir.as_ref().join(format!("{layer_name}.toml"));

    default_on_not_found(remove_dir_recursively(&layer_dir))?;
    default_on_not_found(fs::remove_file(layer_toml))?;

    Ok(())
}

#[derive(thiserror::Error, Debug)]
pub enum DeleteLayerError {
    #[error("I/O error while deleting existing layer: {0}")]
    IoError(#[from] std::io::Error),
}

pub(in crate::layer) fn replace_layer_metadata<M: Serialize, P: AsRef<Path>>(
    layers_dir: P,
    layer_name: &LayerName,
    metadata: M,
) -> Result<(), WriteLayerMetadataError> {
    let layer_content_metadata_path = layers_dir.as_ref().join(format!("{layer_name}.toml"));

    let content_metadata = read_toml_file::<LayerContentMetadata>(&layer_content_metadata_path)?;

    write_toml_file(
        &LayerContentMetadata {
            types: content_metadata.types,
            metadata,
        },
        &layer_content_metadata_path,
    )
    .map_err(WriteLayerMetadataError::TomlFileError)
}

pub(crate) fn replace_layer_types<P: AsRef<Path>>(
    layers_dir: P,
    layer_name: &LayerName,
    layer_types: LayerTypes,
) -> Result<(), WriteLayerMetadataError> {
    let layer_content_metadata_path = layers_dir.as_ref().join(format!("{layer_name}.toml"));

    let mut content_metadata =
        read_toml_file::<LayerContentMetadata>(&layer_content_metadata_path)?;
    content_metadata.types = Some(layer_types);

    write_toml_file(&content_metadata, &layer_content_metadata_path)
        .map_err(WriteLayerMetadataError::TomlFileError)
}

#[derive(thiserror::Error, Debug)]
pub enum WriteLayerMetadataError {
    #[error("Unexpected I/O error while writing layer metadata: {0}")]
    IoError(#[from] std::io::Error),

    #[error("Error while writing layer content metadata TOML: {0}")]
    TomlFileError(#[from] TomlFileError),
}
