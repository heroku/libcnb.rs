use crate::build::BuildContext;
use crate::data::layer_content_metadata::LayerContentMetadata;
use crate::generic::GenericMetadata;
use crate::layer::{Layer, LayerData};
use crate::layer_env::LayerEnv;
use crate::util::default_on_not_found;
use crate::{write_toml_file, TomlFileError};
use crate::{Buildpack, MetadataMigration};
use serde::de::DeserializeOwned;
use serde::Serialize;
use std::fs;
use std::path::Path;

pub(crate) fn handle_layer<B: Buildpack + ?Sized, L: Layer<Buildpack = B>>(
    context: &BuildContext<B>,
    layer_name: impl AsRef<str>,
    layer: L,
) -> Result<LayerData<L::Metadata>, HandleLayerErrorOrBuildpackError<B::Error>> {
    match read_layer(&context.layers_dir, layer_name.as_ref()) {
        Ok(None) => handle_create_layer(context, layer_name, layer),
        Ok(Some(layer_data)) => {
            let layer_should_be_recreated = layer
                .should_be_recreated(context, &layer_data)
                .map_err(HandleLayerErrorOrBuildpackError::BuildpackError)?;

            let layer_should_be_updated = layer
                .should_be_updated(context, &layer_data)
                .map_err(HandleLayerErrorOrBuildpackError::BuildpackError)?;

            if layer_should_be_recreated {
                delete_layer(&context.layers_dir, layer_name.as_ref())?;
                handle_create_layer(context, layer_name, layer)
            } else if layer_should_be_updated {
                handle_update_layer(context, layer_data, layer)
            } else {
                // We need to rewrite the metadata even if we just want to keep the layer around
                // since cached layers are restored without their types, causing the layer to be
                // discarded.
                write_layer(
                    &context.layers_dir,
                    &layer_data.name,
                    layer_data.env,
                    LayerContentMetadata {
                        // We cannot copy the types from layer_data due to an issue with the current
                        // libcnb implementation. The types will be missing in the TOML file on disk
                        // but if they're not there, their default values will be used when
                        // deserializing. Issue: https://github.com/Malax/libcnb.rs/issues/146
                        //
                        // Even if the deserialization of LayerContentMetadata is fixed, it would
                        // not contain the layer types as they're not restored by the CNB lifecycle.
                        // We must call layer.types here to get the correct types for the layer.
                        types: layer.types(),
                        metadata: layer_data.content_metadata.metadata,
                    },
                )?;

                // Reread the layer from disk to ensure the returned layer data accurately reflects
                // the state on disk after we messed with it.
                read_layer(&context.layers_dir, layer_name.as_ref())?
                    .ok_or(HandleLayerError::UnexpectedMissingLayer)
                    .map_err(HandleLayerErrorOrBuildpackError::HandleLayerError)
            }
        }
        Err(ReadLayerError::LayerContentMetadataParseError(_)) => {
            match read_layer::<GenericMetadata, _, _>(&context.layers_dir, layer_name.as_ref()) {
                Ok(Some(generic_layer_data)) => {
                    let metadata_migration_strategy = layer
                        .migrate_incompatible_metadata(
                            context,
                            &generic_layer_data.content_metadata.metadata,
                        )
                        .map_err(HandleLayerErrorOrBuildpackError::BuildpackError)?;

                    match metadata_migration_strategy {
                        MetadataMigration::RecreateLayer => {
                            delete_layer(&context.layers_dir, layer_name.as_ref())?;
                        }
                        MetadataMigration::ReplaceMetadata(migrated_metadata) => {
                            write_layer(
                                &context.layers_dir,
                                layer_name.as_ref(),
                                generic_layer_data.env,
                                LayerContentMetadata {
                                    types: generic_layer_data.content_metadata.types,
                                    metadata: migrated_metadata,
                                },
                            )?;
                        }
                    }

                    handle_layer(context, layer_name, layer)
                }
                Ok(None) => Err(HandleLayerError::UnexpectedMissingLayer.into()),
                Err(read_layer_error) => {
                    Err(HandleLayerError::ReadLayerError(read_layer_error).into())
                }
            }
        }
        Err(read_layer_error) => Err(HandleLayerError::ReadLayerError(read_layer_error).into()),
    }
}

fn handle_create_layer<B: Buildpack + ?Sized, L: Layer<Buildpack = B>>(
    context: &BuildContext<B>,
    layer_name: impl AsRef<str>,
    layer: L,
) -> Result<LayerData<L::Metadata>, HandleLayerErrorOrBuildpackError<B::Error>> {
    let layer_dir = context.layers_dir.join(layer_name.as_ref());

    let layer_result = layer
        .create(context, &layer_dir)
        .map_err(HandleLayerErrorOrBuildpackError::BuildpackError)?;

    write_layer(
        &context.layers_dir,
        layer_name.as_ref(),
        layer_result.env.unwrap_or_default(),
        LayerContentMetadata {
            types: layer.types(),
            metadata: layer_result.metadata,
        },
    )?;

    read_layer(&context.layers_dir, layer_name.as_ref())?
        .ok_or(HandleLayerError::UnexpectedMissingLayer)
        .map_err(HandleLayerErrorOrBuildpackError::HandleLayerError)
}

fn handle_update_layer<B: Buildpack + ?Sized, L: Layer<Buildpack = B>>(
    context: &BuildContext<B>,
    layer_data: LayerData<L::Metadata>,
    layer: L,
) -> Result<LayerData<L::Metadata>, HandleLayerErrorOrBuildpackError<B::Error>> {
    let layer_result = layer
        .update(context, &layer_data)
        .map_err(HandleLayerErrorOrBuildpackError::BuildpackError)?;

    write_layer(
        &context.layers_dir,
        &layer_data.name,
        layer_result.env.unwrap_or_default(),
        LayerContentMetadata {
            types: layer.types(),
            metadata: layer_result.metadata,
        },
    )?;

    read_layer(&context.layers_dir, &layer_data.name)?
        .ok_or(HandleLayerError::UnexpectedMissingLayer)
        .map_err(HandleLayerErrorOrBuildpackError::HandleLayerError)
}

#[derive(Debug)]
pub(crate) enum HandleLayerErrorOrBuildpackError<E> {
    HandleLayerError(HandleLayerError),
    BuildpackError(E),
}

impl<E> From<HandleLayerError> for HandleLayerErrorOrBuildpackError<E> {
    fn from(e: HandleLayerError) -> Self {
        HandleLayerErrorOrBuildpackError::HandleLayerError(e)
    }
}

impl<E> From<ReadLayerError> for HandleLayerErrorOrBuildpackError<E> {
    fn from(e: ReadLayerError) -> Self {
        HandleLayerErrorOrBuildpackError::HandleLayerError(HandleLayerError::ReadLayerError(e))
    }
}

impl<E> From<WriteLayerError> for HandleLayerErrorOrBuildpackError<E> {
    fn from(e: WriteLayerError) -> Self {
        HandleLayerErrorOrBuildpackError::HandleLayerError(HandleLayerError::WriteLayerError(e))
    }
}

impl<E> From<std::io::Error> for HandleLayerErrorOrBuildpackError<E> {
    fn from(e: std::io::Error) -> Self {
        HandleLayerErrorOrBuildpackError::HandleLayerError(HandleLayerError::IoError(e))
    }
}

#[derive(thiserror::Error, Debug)]
pub enum HandleLayerError {
    #[error("Unexpected IoError while handling layer: {0}")]
    IoError(#[from] std::io::Error),

    #[error("Unexpected ReadLayerError while handling layer: {0}")]
    ReadLayerError(#[from] ReadLayerError),

    #[error("Unexpected WriteMetadataError while handling layer: {0}")]
    WriteLayerError(#[from] WriteLayerError),

    #[error("Expected layer to be present, but it was missing")]
    UnexpectedMissingLayer,
}

#[derive(thiserror::Error, Debug)]
pub enum ReadLayerError {
    #[error("Found either layer metadata TOML or layer path, but not both!")]
    DisjointedLayer,

    #[error("Layer content metadata could not be parsed!")]
    LayerContentMetadataParseError(toml::de::Error),

    #[error("Unexpected IoError while reading layer: {0}")]
    IoError(#[from] std::io::Error),
}

#[derive(thiserror::Error, Debug)]
pub enum WriteLayerError {
    #[error("Unexpected IoError while writing layer metadata: {0}")]
    IoError(#[from] std::io::Error),

    #[error("Error while writing layer content metadata TOML: {0}")]
    TomlFileError(#[from] TomlFileError),
}

/// Does not error if the layer doesn't exist.
fn delete_layer<P: AsRef<Path>, S: AsRef<str>>(
    layers_dir: P,
    layer_name: S,
) -> Result<(), std::io::Error> {
    let layer_dir = layers_dir.as_ref().join(layer_name.as_ref());
    let layer_toml = layers_dir
        .as_ref()
        .join(format!("{}.toml", layer_name.as_ref()));

    default_on_not_found(fs::remove_dir_all(&layer_dir))?;
    default_on_not_found(fs::remove_file(&layer_toml))?;

    Ok(())
}

/// Updates layer metadata on disk
fn write_layer<M: Serialize, P: AsRef<Path>, S: AsRef<str>>(
    layers_dir: P,
    layer_name: S,
    layer_env: LayerEnv,
    layer_content_metadata: LayerContentMetadata<M>,
) -> Result<(), WriteLayerError> {
    let layer_dir = layers_dir.as_ref().join(layer_name.as_ref());
    let layer_content_metadata_path = layers_dir
        .as_ref()
        .join(format!("{}.toml", layer_name.as_ref()));

    fs::create_dir_all(&layer_dir)?;
    layer_env.write_to_layer_dir(&layer_dir)?;
    write_toml_file(&layer_content_metadata, layer_content_metadata_path)?;

    Ok(())
}

fn read_layer<M: DeserializeOwned, P: AsRef<Path>, S: AsRef<str>>(
    layers_dir: P,
    layer_name: S,
) -> Result<Option<LayerData<M>>, ReadLayerError> {
    let layer_dir_path = layers_dir.as_ref().join(layer_name.as_ref());
    let layer_toml_path = layers_dir
        .as_ref()
        .join(format!("{}.toml", layer_name.as_ref()));

    if !layer_dir_path.exists() && !layer_toml_path.exists() {
        return Ok(None);
    } else if layer_dir_path.exists() != layer_toml_path.exists() {
        return Err(ReadLayerError::DisjointedLayer);
    }

    let layer_toml_contents = fs::read_to_string(&layer_toml_path)?;
    let layer_content_metadata = toml::from_str::<LayerContentMetadata<M>>(&layer_toml_contents)
        .map_err(ReadLayerError::LayerContentMetadataParseError)?;

    let layer_env = LayerEnv::read_from_layer_dir(&layer_dir_path)?;

    Ok(Some(LayerData {
        name: String::from(layer_name.as_ref()),
        path: layer_dir_path,
        env: layer_env,
        content_metadata: layer_content_metadata,
    }))
}

#[cfg(test)]
mod test {
    use super::*;
    use crate::data::layer_content_metadata::{LayerContentMetadata, LayerTypes};

    use crate::generic::GenericMetadata;
    use crate::layer_env::{LayerEnvBuilder, ModificationBehavior, TargetLifecycle};
    use crate::{read_toml_file, Env};
    use serde::Deserialize;
    use std::ffi::OsString;
    use std::fs;

    use tempfile::tempdir;

    #[test]
    fn delete_layer() {
        let layer_name = "foo";
        let temp_dir = tempdir().unwrap();
        let layers_dir = temp_dir.path();
        let layer_dir = layers_dir.join(&layer_name);

        fs::create_dir_all(&layer_dir).unwrap();
        fs::write(
            layers_dir.join(format!("{}.toml", &layer_name)),
            r#"
            [types]
            launch = true
            build = false
            cache = true
            "#,
        )
        .unwrap();

        super::delete_layer(&layers_dir, &layer_name).unwrap();

        assert!(!layer_dir.exists());
        assert!(!layers_dir.join(format!("{}.toml", &layer_name)).exists());
    }

    #[test]
    fn delete_disjointed_layer() {
        let layer_name = "foo";
        let temp_dir = tempdir().unwrap();
        let layers_dir = temp_dir.path();
        let layer_dir = layers_dir.join(&layer_name);

        fs::write(
            layers_dir.join(format!("{}.toml", &layer_name)),
            r#"
            [types]
            launch = true
            build = false
            cache = true
            "#,
        )
        .unwrap();

        super::delete_layer(&layers_dir, &layer_name).unwrap();

        assert!(!layer_dir.exists());
        assert!(!layers_dir.join(format!("{}.toml", &layer_name)).exists());
    }

    #[test]
    fn delete_nonexisting_layer() {
        let layer_name = "foo";
        let temp_dir = tempdir().unwrap();
        let layers_dir = temp_dir.path();

        super::delete_layer(&layers_dir, &layer_name).unwrap();
    }

    #[test]
    fn write_nonexisting_layer() {
        let layer_name = "foo";
        let temp_dir = tempdir().unwrap();
        let layers_dir = temp_dir.path();
        let layer_dir = layers_dir.join(&layer_name);

        super::write_layer(
            &layers_dir,
            &layer_name,
            LayerEnvBuilder::new()
                .with(
                    TargetLifecycle::All,
                    ModificationBehavior::Default,
                    "ENV_VAR",
                    "ENV_VAR_VALUE",
                )
                .build(),
            LayerContentMetadata {
                types: LayerTypes {
                    launch: true,
                    build: true,
                    cache: false,
                },
                metadata: GenericMetadata::default(),
            },
        )
        .unwrap();

        assert!(layer_dir.exists());

        assert_eq!(
            fs::read_to_string(layer_dir.join("env/ENV_VAR.default")).unwrap(),
            "ENV_VAR_VALUE"
        );

        let layer_content_metadata: LayerContentMetadata<GenericMetadata> =
            read_toml_file(layers_dir.join(format!("{}.toml", layer_name))).unwrap();

        assert_eq!(
            layer_content_metadata.types,
            LayerTypes {
                launch: true,
                build: true,
                cache: false
            }
        );
    }

    #[test]
    fn write_existing_layer() {
        let layer_name = "foo";
        let temp_dir = tempdir().unwrap();
        let layers_dir = temp_dir.path();
        let layer_dir = layers_dir.join(&layer_name);

        super::write_layer(
            &layers_dir,
            &layer_name,
            LayerEnvBuilder::new()
                .with(
                    TargetLifecycle::All,
                    ModificationBehavior::Default,
                    "ENV_VAR",
                    "INITIAL_ENV_VAR_VALUE",
                )
                .with(
                    TargetLifecycle::All,
                    ModificationBehavior::Default,
                    "SOME_OTHER_ENV_VAR",
                    "SOME_OTHER_ENV_VAR_VALUE",
                )
                .build(),
            LayerContentMetadata {
                types: LayerTypes {
                    launch: false,
                    build: false,
                    cache: true,
                },
                metadata: GenericMetadata::default(),
            },
        )
        .unwrap();

        fs::write(layer_dir.join("content.txt"), "Hello World!").unwrap();

        super::write_layer(
            &layers_dir,
            &layer_name,
            LayerEnvBuilder::new()
                .with(
                    TargetLifecycle::All,
                    ModificationBehavior::Default,
                    "ENV_VAR",
                    "NEW_ENV_VAR_VALUE",
                )
                .build(),
            LayerContentMetadata {
                types: LayerTypes {
                    launch: false,
                    build: false,
                    cache: true,
                },
                metadata: GenericMetadata::default(),
            },
        )
        .unwrap();

        assert!(layer_dir.exists());

        assert_eq!(
            fs::read_to_string(layer_dir.join("content.txt")).unwrap(),
            "Hello World!"
        );

        assert_eq!(
            fs::read_to_string(layer_dir.join("env/ENV_VAR.default")).unwrap(),
            "NEW_ENV_VAR_VALUE"
        );

        assert!(!layer_dir.join("env/SOME_OTHER_ENV_VAR.default").exists());

        let layer_content_metadata: LayerContentMetadata<GenericMetadata> =
            read_toml_file(layers_dir.join(format!("{}.toml", layer_name))).unwrap();

        assert_eq!(
            layer_content_metadata.types,
            LayerTypes {
                launch: false,
                build: false,
                cache: true
            }
        );
    }

    #[test]
    fn read_layer() {
        #[derive(Deserialize, Debug, Eq, PartialEq)]
        struct TestLayerMetadata {
            version: String,
            sha: String,
        }

        let layer_name = "foo";
        let temp_dir = tempdir().unwrap();
        let layers_dir = temp_dir.path();
        let layer_dir = layers_dir.join(&layer_name);

        fs::create_dir_all(&layer_dir).unwrap();
        fs::write(
            layers_dir.join(format!("{}.toml", &layer_name)),
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

        // Add a bin directory to test if implicit entries are added to the LayerEnv
        fs::create_dir_all(layer_dir.join("bin")).unwrap();

        // Add a file to the env directory to test if explicit entries are added to the LayerEnv
        fs::create_dir_all(layer_dir.join("env")).unwrap();
        fs::write(layer_dir.join("env/CUSTOM_ENV"), "CUSTOM_ENV_VALUE").unwrap();

        let layer_data = super::read_layer::<TestLayerMetadata, _, _>(&layers_dir, &layer_name)
            .unwrap()
            .unwrap();

        assert_eq!(layer_data.path, layer_dir);

        assert_eq!(layer_data.name, String::from(layer_name));

        assert_eq!(
            layer_data.content_metadata.types,
            LayerTypes {
                launch: true,
                build: false,
                cache: true
            }
        );

        assert_eq!(
            layer_data.content_metadata.metadata,
            TestLayerMetadata {
                version: String::from("1.0"),
                sha: String::from(
                    "2608a36467a6fec50be1672bfbf88b04b9ec8efaafa58c71d9edf73519ed8e2c"
                )
            }
        );

        let applied_layer_env = layer_data.env.apply(TargetLifecycle::Build, &Env::new());
        assert_eq!(
            applied_layer_env.get("PATH"),
            Some(layer_dir.join("bin").into())
        );

        assert_eq!(
            applied_layer_env.get("CUSTOM_ENV"),
            Some(OsString::from("CUSTOM_ENV_VALUE"))
        );
    }

    #[test]
    fn read_malformed_toml_layer() {
        let layer_name = "foo";
        let temp_dir = tempdir().unwrap();
        let layers_dir = temp_dir.path();
        let layer_dir = layers_dir.join(&layer_name);

        fs::create_dir_all(&layer_dir).unwrap();
        fs::write(
            layers_dir.join(format!("{}.toml", &layer_name)),
            r#"
            [types
            build = true
            launch = true
            cache = true
            "#,
        )
        .unwrap();

        match super::read_layer::<GenericMetadata, _, _>(&layers_dir, &layer_name) {
            Err(ReadLayerError::LayerContentMetadataParseError(toml_error)) => {
                assert_eq!(toml_error.line_col(), Some((1, 18)));
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

        let layer_name = "foo";
        let temp_dir = tempdir().unwrap();
        let layers_dir = temp_dir.path();
        let layer_dir = layers_dir.join(&layer_name);

        fs::create_dir_all(&layer_dir).unwrap();
        fs::write(
            layers_dir.join(format!("{}.toml", &layer_name)),
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

        match super::read_layer::<TestLayerMetadata, _, _>(&layers_dir, &layer_name) {
            Err(ReadLayerError::LayerContentMetadataParseError(toml_error)) => {
                assert_eq!(toml_error.line_col(), Some((6, 12)));
            }
            _ => panic!("Expected ReadLayerError::LayerContentMetadataParseError!"),
        }
    }

    #[test]
    fn read_disjointed_layer_1() {
        let layer_name = "foo";
        let temp_dir = tempdir().unwrap();
        let layers_dir = temp_dir.path();

        fs::create_dir_all(layers_dir.join(&layer_name)).unwrap();

        match super::read_layer::<GenericMetadata, _, _>(&layers_dir, &layer_name) {
            Err(ReadLayerError::DisjointedLayer) => {}
            _ => panic!("Expected ReadLayerError::DisjointedLayer!"),
        }
    }

    #[test]
    fn read_disjointed_layer_2() {
        let layer_name = "foo";
        let temp_dir = tempdir().unwrap();
        let layers_dir = temp_dir.path();

        fs::write(layers_dir.join(format!("{}.toml", &layer_name)), "").unwrap();

        match super::read_layer::<GenericMetadata, _, _>(&layers_dir, &layer_name) {
            Err(ReadLayerError::DisjointedLayer) => {}
            _ => panic!("Expected ReadLayerError::DisjointedLayer!"),
        }
    }

    #[test]
    fn read_nonexistent_layer() {
        let layer_name = "foo";
        let temp_dir = tempdir().unwrap();
        let layers_dir = temp_dir.path();

        match super::read_layer::<GenericMetadata, _, _>(&layers_dir, &layer_name) {
            Ok(None) => {}
            _ => panic!("Expected Ok(None)!"),
        }
    }
}
