// This lint triggers when both layer_dir and layers_dir are present which are quite common.
#![allow(clippy::similar_names)]

use crate::sbom::{cnb_sbom_path, Sbom};
use crate::util::{default_on_not_found, remove_dir_recursively};
use libcnb_common::toml_file::{read_toml_file, write_toml_file, TomlFileError};
use libcnb_data::layer::LayerName;
use libcnb_data::layer_content_metadata::{LayerContentMetadata, LayerTypes};
use libcnb_data::sbom::SBOM_FORMATS;
use serde::de::DeserializeOwned;
use serde::Serialize;
use std::collections::HashMap;
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

pub(in crate::layer) fn write_layer<M: Serialize, P: AsRef<Path>>(
    layers_dir: P,
    layer_name: &LayerName,
    layer_content_metadata: &LayerContentMetadata<M>,
) -> Result<(), WriteLayerError> {
    let layer_dir = layers_dir.as_ref().join(layer_name.as_str());
    fs::create_dir_all(layer_dir)?;

    let layer_content_metadata_path = layers_dir.as_ref().join(format!("{layer_name}.toml"));

    write_toml_file(&layer_content_metadata, layer_content_metadata_path)
        .map_err(WriteLayerMetadataError::TomlFileError)
        .map_err(WriteLayerError::WriteLayerMetadataError)?;

    Ok(())
}

#[derive(thiserror::Error, Debug)]
#[allow(clippy::enum_variant_names)]
pub enum WriteLayerError {
    #[error("Layer content metadata couldn't be parsed!")]
    WriteLayerMetadataError(WriteLayerMetadataError),

    #[error("TODO")]
    ReplaceLayerSbomsError(ReplaceLayerSbomsError),

    #[error("TODO")]
    ReplaceLayerExecdProgramsError(ReplaceLayerExecdProgramsError),

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

pub(in crate::layer) fn replace_layer_types<P: AsRef<Path>>(
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

pub(crate) fn replace_layer_sboms<P: AsRef<Path>>(
    layers_dir: P,
    layer_name: &LayerName,
    sboms: &[Sbom],
) -> Result<(), ReplaceLayerSbomsError> {
    let layers_dir = layers_dir.as_ref();

    if !layers_dir.join(layer_name.as_str()).is_dir() {
        return Err(ReplaceLayerSbomsError::MissingLayer(layer_name.clone()));
    }

    for format in SBOM_FORMATS {
        default_on_not_found(fs::remove_file(cnb_sbom_path(
            format, layers_dir, layer_name,
        )))?;
    }

    for sbom in sboms {
        fs::write(
            cnb_sbom_path(&sbom.format, layers_dir, layer_name),
            &sbom.data,
        )?;
    }

    Ok(())
}

#[derive(thiserror::Error, Debug)]
pub enum ReplaceLayerSbomsError {
    #[error("Layer doesn't exist: {0}")]
    MissingLayer(LayerName),

    #[error("Unexpected I/O error while replacing layer SBOMs: {0}")]
    IoError(#[from] std::io::Error),
}

pub(in crate::layer) fn replace_layer_exec_d_programs<P: AsRef<Path>>(
    layers_dir: P,
    layer_name: &LayerName,
    exec_d_programs: &HashMap<String, PathBuf>,
) -> Result<(), ReplaceLayerExecdProgramsError> {
    let layer_dir = layers_dir.as_ref().join(layer_name.as_str());

    if !layer_dir.is_dir() {
        return Err(ReplaceLayerExecdProgramsError::MissingLayer(
            layer_name.clone(),
        ));
    }

    let exec_d_dir = layer_dir.join("exec.d");

    if exec_d_dir.is_dir() {
        fs::remove_dir_all(&exec_d_dir)?;
    }

    if !exec_d_programs.is_empty() {
        fs::create_dir_all(&exec_d_dir)?;

        for (name, path) in exec_d_programs {
            // We could just try to copy the file here and let the call-site deal with the
            // I/O errors when the path does not exist. We're using an explicit error variant
            // for a missing exec.d binary makes it easier to debug issues with packaging
            // since the usage of exec.d binaries often relies on implicit packaging the
            // buildpack author might not be aware of.
            Some(&path)
                .filter(|path| path.exists())
                .ok_or_else(|| ReplaceLayerExecdProgramsError::MissingExecDFile(path.clone()))
                .and_then(|path| {
                    fs::copy(path, exec_d_dir.join(name))
                        .map_err(ReplaceLayerExecdProgramsError::IoError)
                })?;
        }
    }

    Ok(())
}

#[derive(thiserror::Error, Debug)]
pub enum ReplaceLayerExecdProgramsError {
    #[error("Unexpected I/O error while replacing layer execd programs: {0}")]
    IoError(#[from] std::io::Error),

    #[error("Couldn't find exec.d file for copying: {0}")]
    MissingExecDFile(PathBuf),

    #[error("Layer doesn't exist: {0}")]
    MissingLayer(LayerName),
}

#[derive(thiserror::Error, Debug)]
pub enum WriteLayerMetadataError {
    #[error("Unexpected I/O error while writing layer metadata: {0}")]
    IoError(#[from] std::io::Error),

    #[error("Error while writing layer content metadata TOML: {0}")]
    TomlFileError(#[from] TomlFileError),
}

#[derive(thiserror::Error, Debug)]
pub enum LayerError {
    #[error("{0}")]
    ReadLayerError(#[from] ReadLayerError),
    #[error("{0}")]
    WriteLayerError(#[from] WriteLayerError),
    #[error("{0}")]
    DeleteLayerError(#[from] DeleteLayerError),
    #[error("Cannot read generic layer metadata: {0}")]
    CouldNotReadGenericLayerMetadata(TomlFileError),
    #[error("Cannot read layer {0} after creating it")]
    CouldNotReadLayerAfterCreate(LayerName),
    #[error("Unexpected I/O error: {0}")]
    IoError(#[from] std::io::Error),
    #[error("Unexpected missing layer")]
    UnexpectedMissingLayer,
}

#[cfg(test)]
mod test {
    use crate::layer::ReadLayerError;
    use libcnb_data::generic::GenericMetadata;
    use libcnb_data::layer_content_metadata::{LayerContentMetadata, LayerTypes};
    use libcnb_data::layer_name;
    use serde::Deserialize;
    use std::fs;
    use tempfile::tempdir;

    #[test]
    fn read_layer() {
        #[derive(Deserialize, Debug, Eq, PartialEq)]
        struct TestLayerMetadata {
            version: String,
            sha: String,
        }

        let layer_name = layer_name!("foo");
        let temp_dir = tempdir().unwrap();
        let layers_dir = temp_dir.path();
        let layer_dir = layers_dir.join(layer_name.as_str());

        fs::create_dir_all(&layer_dir).unwrap();
        fs::write(
            layers_dir.join(format!("{layer_name}.toml")),
            r#"
            [types]
            launch = true
            build = false
            cache = true

            [metadata]
            version = "1.0"
            sha = "2608a36467a6fec50be1672bfbf88b04b9ec8efaafa58c71d9edf73519ed8e2c"
            "#,
        )
        .unwrap();

        let layer_data = super::read_layer::<TestLayerMetadata, _>(layers_dir, &layer_name)
            .unwrap()
            .unwrap();

        assert_eq!(layer_data.path, layer_dir);

        assert_eq!(layer_data.name, layer_name);

        assert_eq!(
            layer_data.metadata.types,
            Some(LayerTypes {
                launch: true,
                build: false,
                cache: true
            })
        );

        assert_eq!(
            layer_data.metadata.metadata,
            TestLayerMetadata {
                version: String::from("1.0"),
                sha: String::from(
                    "2608a36467a6fec50be1672bfbf88b04b9ec8efaafa58c71d9edf73519ed8e2c"
                )
            }
        );
    }

    #[test]
    fn read_malformed_toml_layer() {
        let layer_name = layer_name!("foo");
        let temp_dir = tempdir().unwrap();
        let layers_dir = temp_dir.path();
        let layer_dir = layers_dir.join(layer_name.as_str());

        fs::create_dir_all(layer_dir).unwrap();
        fs::write(
            layers_dir.join(format!("{layer_name}.toml")),
            r"
            [types
            build = true
            launch = true
            cache = true
            ",
        )
        .unwrap();

        match super::read_layer::<GenericMetadata, _>(layers_dir, &layer_name) {
            Err(ReadLayerError::LayerContentMetadataParseError(toml_error)) => {
                assert_eq!(toml_error.span(), Some(19..20));
            }
            _ => panic!("Expected ReadLayerError::LayerContentMetadataParseError!"),
        }
    }

    #[test]
    fn read_incompatible_metadata_layer() {
        #[derive(Deserialize, Debug, Eq, PartialEq)]
        struct TestLayerMetadata {
            version: String,
            sha: String,
        }

        let layer_name = layer_name!("foo");
        let temp_dir = tempdir().unwrap();
        let layers_dir = temp_dir.path();
        let layer_dir = layers_dir.join(layer_name.as_str());

        fs::create_dir_all(layer_dir).unwrap();
        fs::write(
            layers_dir.join(format!("{layer_name}.toml")),
            r#"
            [types]
            build = true
            launch = true
            cache = true

            [metadata]
            version = "1.0"
            "#,
        )
        .unwrap();

        match super::read_layer::<TestLayerMetadata, _>(layers_dir, &layer_name) {
            Err(ReadLayerError::LayerContentMetadataParseError(toml_error)) => {
                assert_eq!(toml_error.span(), Some(110..148));
            }
            _ => panic!("Expected ReadLayerError::LayerContentMetadataParseError!"),
        }
    }

    #[test]
    fn read_layer_without_layer_directory() {
        let layer_name = layer_name!("foo");
        let temp_dir = tempdir().unwrap();
        let layers_dir = temp_dir.path();
        let layer_dir = layers_dir.join(layer_name.as_str());

        fs::create_dir_all(layer_dir).unwrap();

        match super::read_layer::<GenericMetadata, _>(layers_dir, &layer_name) {
            Ok(Some(layer_data)) => {
                assert_eq!(
                    layer_data.metadata,
                    LayerContentMetadata {
                        types: None,
                        metadata: None
                    }
                );
            }
            _ => panic!("Expected Ok(Some(_)!"),
        }
    }

    #[test]
    fn read_layer_without_layer_content_metadata() {
        let layer_name = layer_name!("foo");
        let temp_dir = tempdir().unwrap();
        let layers_dir = temp_dir.path();

        fs::write(layers_dir.join(format!("{layer_name}.toml")), "").unwrap();

        match super::read_layer::<GenericMetadata, _>(layers_dir, &layer_name) {
            Ok(None) => {}
            _ => panic!("Expected Ok(None)!"),
        }
    }

    #[test]
    fn read_nonexistent_layer() {
        let layer_name = layer_name!("foo");
        let temp_dir = tempdir().unwrap();
        let layers_dir = temp_dir.path();

        match super::read_layer::<GenericMetadata, _>(layers_dir, &layer_name) {
            Ok(None) => {}
            _ => panic!("Expected Ok(None)!"),
        }
    }
}
