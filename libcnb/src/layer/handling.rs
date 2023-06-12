// This lint triggers when both layer_dir and layers_dir are present which are quite common.
#![allow(clippy::similar_names)]

use crate::build::BuildContext;
use crate::data::layer::LayerName;
use crate::data::layer_content_metadata::LayerContentMetadata;
use crate::generic::GenericMetadata;
use crate::layer::{ExistingLayerStrategy, Layer, LayerData, MetadataMigration};
use crate::layer_env::LayerEnv;
use crate::sbom::{cnb_sbom_path, Sbom};
use crate::util::{default_on_not_found, remove_dir_recursively};
use crate::Buildpack;
use crate::{write_toml_file, TomlFileError};
use libcnb_data::sbom::SBOM_FORMATS;
use serde::de::DeserializeOwned;
use serde::Serialize;
use std::collections::HashMap;
use std::fs;
use std::path::{Path, PathBuf};

pub(crate) fn handle_layer<B: Buildpack + ?Sized, L: Layer<Buildpack = B>>(
    context: &BuildContext<B>,
    layer_name: LayerName,
    layer: L,
) -> Result<LayerData<L::Metadata>, HandleLayerErrorOrBuildpackError<B::Error>> {
    match read_layer(&context.layers_dir, &layer_name) {
        Ok(None) => handle_create_layer(context, &layer_name, &layer),
        Ok(Some(layer_data)) => {
            let existing_layer_strategy = layer
                .existing_layer_strategy(context, &layer_data)
                .map_err(HandleLayerErrorOrBuildpackError::BuildpackError)?;

            match existing_layer_strategy {
                ExistingLayerStrategy::Recreate => {
                    delete_layer(&context.layers_dir, &layer_name)?;
                    handle_create_layer(context, &layer_name, &layer)
                }
                ExistingLayerStrategy::Update => handle_update_layer(context, &layer_data, &layer),
                ExistingLayerStrategy::Keep => {
                    // We need to rewrite the metadata even if we just want to keep the layer around
                    // since cached layers are restored without their types, causing the layer to be
                    // discarded.
                    write_layer(
                        &context.layers_dir,
                        &layer_data.name,
                        &layer_data.env,
                        &LayerContentMetadata {
                            // We cannot copy the types from layer_data since they're not restored by the CNB lifecycle.
                            // We must call layer.types here to get the correct types for the layer.
                            types: Some(layer.types()),
                            metadata: layer_data.content_metadata.metadata,
                        },
                        ExecDPrograms::Keep,
                        Sboms::Keep,
                    )?;

                    // Reread the layer from disk to ensure the returned layer data accurately reflects
                    // the state on disk after we messed with it.
                    read_layer(&context.layers_dir, &layer_name)?
                        .ok_or(HandleLayerError::UnexpectedMissingLayer)
                        .map_err(HandleLayerErrorOrBuildpackError::HandleLayerError)
                }
            }
        }
        Err(ReadLayerError::LayerContentMetadataParseError(_)) => {
            match read_layer::<GenericMetadata, _>(&context.layers_dir, &layer_name) {
                Ok(Some(generic_layer_data)) => {
                    let metadata_migration_strategy = layer
                        .migrate_incompatible_metadata(
                            context,
                            &generic_layer_data.content_metadata.metadata,
                        )
                        .map_err(HandleLayerErrorOrBuildpackError::BuildpackError)?;

                    match metadata_migration_strategy {
                        MetadataMigration::RecreateLayer => {
                            delete_layer(&context.layers_dir, &layer_name)?;
                        }
                        MetadataMigration::ReplaceMetadata(migrated_metadata) => {
                            write_layer(
                                &context.layers_dir,
                                &layer_name,
                                &generic_layer_data.env,
                                &LayerContentMetadata {
                                    types: generic_layer_data.content_metadata.types,
                                    metadata: migrated_metadata,
                                },
                                ExecDPrograms::Keep,
                                Sboms::Keep,
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
    layer_name: &LayerName,
    layer: &L,
) -> Result<LayerData<L::Metadata>, HandleLayerErrorOrBuildpackError<B::Error>> {
    let layer_dir = context.layers_dir.join(layer_name.as_str());

    fs::create_dir_all(&layer_dir)
        .map_err(HandleLayerError::IoError)
        .map_err(HandleLayerErrorOrBuildpackError::HandleLayerError)?;

    let layer_result = layer
        .create(context, &layer_dir)
        .map_err(HandleLayerErrorOrBuildpackError::BuildpackError)?;

    write_layer(
        &context.layers_dir,
        layer_name,
        &layer_result.env.unwrap_or_default(),
        &LayerContentMetadata {
            types: Some(layer.types()),
            metadata: layer_result.metadata,
        },
        ExecDPrograms::Overwrite(layer_result.exec_d_programs),
        Sboms::Overwrite(layer_result.sboms),
    )?;

    read_layer(&context.layers_dir, layer_name)?
        .ok_or(HandleLayerError::UnexpectedMissingLayer)
        .map_err(HandleLayerErrorOrBuildpackError::HandleLayerError)
}

fn handle_update_layer<B: Buildpack + ?Sized, L: Layer<Buildpack = B>>(
    context: &BuildContext<B>,
    layer_data: &LayerData<L::Metadata>,
    layer: &L,
) -> Result<LayerData<L::Metadata>, HandleLayerErrorOrBuildpackError<B::Error>> {
    let layer_result = layer
        .update(context, layer_data)
        .map_err(HandleLayerErrorOrBuildpackError::BuildpackError)?;

    write_layer(
        &context.layers_dir,
        &layer_data.name,
        &layer_result.env.unwrap_or_default(),
        &LayerContentMetadata {
            types: Some(layer.types()),
            metadata: layer_result.metadata,
        },
        ExecDPrograms::Overwrite(layer_result.exec_d_programs),
        Sboms::Overwrite(layer_result.sboms),
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
        Self::HandleLayerError(e)
    }
}

impl<E> From<DeleteLayerError> for HandleLayerErrorOrBuildpackError<E> {
    fn from(e: DeleteLayerError) -> Self {
        Self::HandleLayerError(HandleLayerError::DeleteLayerError(e))
    }
}

impl<E> From<ReadLayerError> for HandleLayerErrorOrBuildpackError<E> {
    fn from(e: ReadLayerError) -> Self {
        Self::HandleLayerError(HandleLayerError::ReadLayerError(e))
    }
}

impl<E> From<WriteLayerError> for HandleLayerErrorOrBuildpackError<E> {
    fn from(e: WriteLayerError) -> Self {
        Self::HandleLayerError(HandleLayerError::WriteLayerError(e))
    }
}

impl<E> From<std::io::Error> for HandleLayerErrorOrBuildpackError<E> {
    fn from(e: std::io::Error) -> Self {
        Self::HandleLayerError(HandleLayerError::IoError(e))
    }
}

#[derive(thiserror::Error, Debug)]
pub enum HandleLayerError {
    #[error("Unexpected IoError while handling layer: {0}")]
    IoError(#[from] std::io::Error),

    #[error("Unexpected DeleteLayerError while handling layer: {0}")]
    DeleteLayerError(#[from] DeleteLayerError),

    #[error("Unexpected ReadLayerError while handling layer: {0}")]
    ReadLayerError(#[from] ReadLayerError),

    #[error("Unexpected WriteLayerError while handling layer: {0}")]
    WriteLayerError(#[from] WriteLayerError),

    #[error("Expected layer to be present, but it was missing")]
    UnexpectedMissingLayer,
}

#[derive(thiserror::Error, Debug)]
pub enum DeleteLayerError {
    #[error("IOError while deleting existing layer: {0}")]
    IoError(#[from] std::io::Error),
}

#[derive(thiserror::Error, Debug)]
pub enum ReadLayerError {
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

    #[error("Cannot find exec.d file for copying: {0}")]
    MissingExecDFile(PathBuf),
}

#[derive(Debug)]
enum ExecDPrograms {
    Keep,
    Overwrite(HashMap<String, PathBuf>),
}

#[derive(Debug)]
enum Sboms {
    Keep,
    Overwrite(Vec<Sbom>),
}

/// Does not error if the layer doesn't exist.
fn delete_layer<P: AsRef<Path>>(
    layers_dir: P,
    layer_name: &LayerName,
) -> Result<(), DeleteLayerError> {
    let layer_dir = layers_dir.as_ref().join(layer_name.as_str());
    let layer_toml = layers_dir.as_ref().join(format!("{layer_name}.toml"));

    default_on_not_found(remove_dir_recursively(&layer_dir))?;
    default_on_not_found(fs::remove_file(layer_toml))?;

    Ok(())
}

/// Updates layer metadata on disk
fn write_layer<M: Serialize, P: AsRef<Path>>(
    layers_dir: P,
    layer_name: &LayerName,
    layer_env: &LayerEnv,
    layer_content_metadata: &LayerContentMetadata<M>,
    layer_exec_d_programs: ExecDPrograms,
    layer_sboms: Sboms,
) -> Result<(), WriteLayerError> {
    let layer_dir = layers_dir.as_ref().join(layer_name.as_str());
    let layer_content_metadata_path = layers_dir.as_ref().join(format!("{layer_name}.toml"));

    fs::create_dir_all(&layer_dir)?;
    layer_env.write_to_layer_dir(&layer_dir)?;
    write_toml_file(&layer_content_metadata, layer_content_metadata_path)?;

    match layer_sboms {
        Sboms::Overwrite(layer_sboms) => {
            for format in SBOM_FORMATS {
                default_on_not_found(fs::remove_file(cnb_sbom_path(
                    format,
                    &layers_dir,
                    layer_name,
                )))?;
            }

            for layer_sbom in layer_sboms {
                fs::write(
                    cnb_sbom_path(&layer_sbom.format, &layers_dir, layer_name),
                    &layer_sbom.data,
                )?;
            }
        }
        Sboms::Keep => {}
    }

    match layer_exec_d_programs {
        ExecDPrograms::Overwrite(exec_d_programs) => {
            let exec_d_dir = layer_dir.join("exec.d");

            if exec_d_dir.is_dir() {
                fs::remove_dir_all(&exec_d_dir)?;
            }

            if !exec_d_programs.is_empty() {
                fs::create_dir_all(&exec_d_dir)?;

                for (name, path) in exec_d_programs {
                    // We could just try to copy the file here and let the call-site deal with the
                    // IO errors when the path does not exist. We're using an explicit error variant
                    // for a missing exec.d binary makes it easier to debug issues with packaging
                    // since the usage of exec.d binaries often relies on implicit packaging the
                    // buildpack author might not be aware of.
                    Some(&path)
                        .filter(|path| path.exists())
                        .ok_or_else(|| WriteLayerError::MissingExecDFile(path.clone()))
                        .and_then(|path| {
                            fs::copy(path, exec_d_dir.join(name)).map_err(WriteLayerError::IoError)
                        })?;
                }
            }
        }
        ExecDPrograms::Keep => {}
    }

    Ok(())
}

fn read_layer<M: DeserializeOwned, P: AsRef<Path>>(
    layers_dir: P,
    layer_name: &LayerName,
) -> Result<Option<LayerData<M>>, ReadLayerError> {
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

    let layer_env = LayerEnv::read_from_layer_dir(&layer_dir_path)?;

    Ok(Some(LayerData {
        name: layer_name.clone(),
        path: layer_dir_path,
        env: layer_env,
        content_metadata: layer_content_metadata,
    }))
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::data::layer_content_metadata::{LayerContentMetadata, LayerTypes};
    use crate::data::layer_name;
    use crate::generic::GenericMetadata;
    use crate::layer_env::{ModificationBehavior, Scope};
    use crate::read_toml_file;
    use serde::Deserialize;
    use std::ffi::OsString;
    use std::fs;

    use tempfile::tempdir;

    #[test]
    fn delete_layer() {
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
            "#,
        )
        .unwrap();

        super::delete_layer(layers_dir, &layer_name).unwrap();

        assert!(!layer_dir.exists());
        assert!(!layers_dir.join(format!("{layer_name}.toml")).exists());
    }

    #[test]
    fn delete_disjointed_layer() {
        let layer_name = layer_name!("foo");
        let temp_dir = tempdir().unwrap();
        let layers_dir = temp_dir.path();
        let layer_dir = layers_dir.join(layer_name.as_str());

        fs::write(
            layers_dir.join(format!("{layer_name}.toml")),
            r#"
            [types]
            launch = true
            build = false
            cache = true
            "#,
        )
        .unwrap();

        super::delete_layer(layers_dir, &layer_name).unwrap();

        assert!(!layer_dir.exists());
        assert!(!layers_dir.join(format!("{layer_name}.toml")).exists());
    }

    #[test]
    fn delete_nonexisting_layer() {
        let layer_name = layer_name!("foo");
        let temp_dir = tempdir().unwrap();
        let layers_dir = temp_dir.path();

        super::delete_layer(layers_dir, &layer_name).unwrap();
    }

    #[test]
    fn write_nonexisting_layer() {
        let layer_name = layer_name!("foo");
        let temp_dir = tempdir().unwrap();
        let layers_dir = temp_dir.path();
        let layer_dir = layers_dir.join(layer_name.as_str());

        let execd_source_temp_dir = tempdir().unwrap();
        let foo_execd_file = execd_source_temp_dir.path().join("foo");
        fs::write(&foo_execd_file, "foo-contents").unwrap();

        super::write_layer(
            layers_dir,
            &layer_name,
            &LayerEnv::new().chainable_insert(
                Scope::All,
                ModificationBehavior::Default,
                "ENV_VAR",
                "ENV_VAR_VALUE",
            ),
            &LayerContentMetadata {
                types: Some(LayerTypes {
                    launch: true,
                    build: true,
                    cache: false,
                }),
                metadata: GenericMetadata::default(),
            },
            ExecDPrograms::Overwrite(HashMap::from([(String::from("foo"), foo_execd_file)])),
            Sboms::Keep,
        )
        .unwrap();

        assert!(layer_dir.exists());

        assert_eq!(
            fs::read_to_string(layer_dir.join("env/ENV_VAR.default")).unwrap(),
            "ENV_VAR_VALUE"
        );

        assert_eq!(
            fs::read_to_string(layer_dir.join("exec.d/foo")).unwrap(),
            "foo-contents"
        );

        let layer_content_metadata: LayerContentMetadata<GenericMetadata> =
            read_toml_file(layers_dir.join(format!("{layer_name}.toml"))).unwrap();

        assert_eq!(
            layer_content_metadata.types,
            Some(LayerTypes {
                launch: true,
                build: true,
                cache: false
            })
        );
    }

    #[test]
    fn write_nonexisting_layer_with_nonexisting_exec_d_path() {
        let layer_name = layer_name!("foo");
        let temp_dir = tempdir().unwrap();
        let layers_dir = temp_dir.path();

        let execd_file = PathBuf::from("/this/path/should/not/exist/exec_d_binary");
        let write_layer_error = super::write_layer(
            layers_dir,
            &layer_name,
            &LayerEnv::new(),
            &LayerContentMetadata {
                types: Some(LayerTypes {
                    launch: true,
                    build: true,
                    cache: false,
                }),
                metadata: GenericMetadata::default(),
            },
            ExecDPrograms::Overwrite(HashMap::from([(String::from("foo"), execd_file.clone())])),
            Sboms::Keep,
        )
        .unwrap_err();

        match write_layer_error {
            WriteLayerError::MissingExecDFile(path) => {
                assert_eq!(path, execd_file);
            }
            other => {
                panic!("Expected WriteLayerError::MissingExecDFile, but got {other:?}");
            }
        };
    }

    #[test]
    fn write_existing_layer() {
        let layer_name = layer_name!("foo");
        let temp_dir = tempdir().unwrap();
        let layers_dir = temp_dir.path();
        let layer_dir = layers_dir.join(layer_name.as_str());

        let execd_source_temp_dir = tempdir().unwrap();
        let foo_execd_file = execd_source_temp_dir.path().join("foo");
        let bar_execd_file = execd_source_temp_dir.path().join("bar");
        let baz_execd_file = execd_source_temp_dir.path().join("baz");
        fs::write(&foo_execd_file, "foo-contents").unwrap();
        fs::write(&bar_execd_file, "bar-contents").unwrap();
        fs::write(&baz_execd_file, "baz-contents").unwrap();

        super::write_layer(
            layers_dir,
            &layer_name,
            &LayerEnv::new()
                .chainable_insert(
                    Scope::All,
                    ModificationBehavior::Default,
                    "ENV_VAR",
                    "INITIAL_ENV_VAR_VALUE",
                )
                .chainable_insert(
                    Scope::All,
                    ModificationBehavior::Default,
                    "SOME_OTHER_ENV_VAR",
                    "SOME_OTHER_ENV_VAR_VALUE",
                ),
            &LayerContentMetadata {
                types: Some(LayerTypes {
                    launch: false,
                    build: false,
                    cache: true,
                }),
                metadata: GenericMetadata::default(),
            },
            ExecDPrograms::Overwrite(HashMap::from([(String::from("foo"), foo_execd_file)])),
            Sboms::Keep,
        )
        .unwrap();

        fs::write(layer_dir.join("content.txt"), "Hello World!").unwrap();

        super::write_layer(
            layers_dir,
            &layer_name,
            &LayerEnv::new().chainable_insert(
                Scope::All,
                ModificationBehavior::Default,
                "ENV_VAR",
                "NEW_ENV_VAR_VALUE",
            ),
            &LayerContentMetadata {
                types: Some(LayerTypes {
                    launch: false,
                    build: false,
                    cache: true,
                }),
                metadata: GenericMetadata::default(),
            },
            ExecDPrograms::Overwrite(HashMap::from([
                (String::from("bar"), bar_execd_file),
                (String::from("baz"), baz_execd_file),
            ])),
            Sboms::Keep,
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

        assert!(!layer_dir.join("exec.d/foo").exists());

        assert_eq!(
            fs::read_to_string(layer_dir.join("exec.d/bar")).unwrap(),
            "bar-contents"
        );

        assert_eq!(
            fs::read_to_string(layer_dir.join("exec.d/baz")).unwrap(),
            "baz-contents"
        );

        let layer_content_metadata: LayerContentMetadata<GenericMetadata> =
            read_toml_file(layers_dir.join(format!("{layer_name}.toml"))).unwrap();

        assert_eq!(
            layer_content_metadata.types,
            Some(LayerTypes {
                launch: false,
                build: false,
                cache: true
            })
        );
    }

    #[test]
    fn write_layer_keep_execd() {
        let layer_name = layer_name!("foo");
        let temp_dir = tempdir().unwrap();
        let layers_dir = temp_dir.path();
        let layer_dir = layers_dir.join(layer_name.as_str());

        super::write_layer(
            layers_dir,
            &layer_name,
            &LayerEnv::new(),
            &LayerContentMetadata {
                types: Some(LayerTypes {
                    launch: false,
                    build: false,
                    cache: true,
                }),
                metadata: GenericMetadata::default(),
            },
            ExecDPrograms::Keep,
            Sboms::Keep,
        )
        .unwrap();

        assert!(!layer_dir.join("exec.d").exists());
    }

    #[test]
    fn write_existing_layer_keep_execd() {
        let layer_name = layer_name!("foo");
        let temp_dir = tempdir().unwrap();
        let layers_dir = temp_dir.path();
        let layer_dir = layers_dir.join(layer_name.as_str());

        let execd_source_temp_dir = tempdir().unwrap();
        let foo_execd_file = execd_source_temp_dir.path().join("foo");
        fs::write(&foo_execd_file, "foo-contents").unwrap();

        super::write_layer(
            layers_dir,
            &layer_name,
            &LayerEnv::new(),
            &LayerContentMetadata {
                types: Some(LayerTypes {
                    launch: false,
                    build: false,
                    cache: true,
                }),
                metadata: GenericMetadata::default(),
            },
            ExecDPrograms::Overwrite(HashMap::from([(String::from("foo"), foo_execd_file)])),
            Sboms::Keep,
        )
        .unwrap();

        assert_eq!(
            fs::read_to_string(layer_dir.join("exec.d/foo")).unwrap(),
            "foo-contents"
        );

        super::write_layer(
            layers_dir,
            &layer_name,
            &LayerEnv::new(),
            &LayerContentMetadata {
                types: Some(LayerTypes {
                    launch: false,
                    build: false,
                    cache: true,
                }),
                metadata: GenericMetadata::default(),
            },
            ExecDPrograms::Keep,
            Sboms::Keep,
        )
        .unwrap();

        assert_eq!(
            fs::read_to_string(layer_dir.join("exec.d/foo")).unwrap(),
            "foo-contents"
        );
    }

    #[test]
    fn write_existing_layer_overwrite_with_empty_execd() {
        let layer_name = layer_name!("foo");
        let temp_dir = tempdir().unwrap();
        let layers_dir = temp_dir.path();
        let layer_dir = layers_dir.join(layer_name.as_str());

        let execd_source_temp_dir = tempdir().unwrap();
        let foo_execd_file = execd_source_temp_dir.path().join("foo");
        fs::write(&foo_execd_file, "foo-contents").unwrap();

        super::write_layer(
            layers_dir,
            &layer_name,
            &LayerEnv::new(),
            &LayerContentMetadata {
                types: Some(LayerTypes {
                    launch: false,
                    build: false,
                    cache: true,
                }),
                metadata: GenericMetadata::default(),
            },
            ExecDPrograms::Overwrite(HashMap::from([(String::from("foo"), foo_execd_file)])),
            Sboms::Keep,
        )
        .unwrap();

        assert_eq!(
            fs::read_to_string(layer_dir.join("exec.d/foo")).unwrap(),
            "foo-contents"
        );

        super::write_layer(
            layers_dir,
            &layer_name,
            &LayerEnv::new(),
            &LayerContentMetadata {
                types: Some(LayerTypes {
                    launch: false,
                    build: false,
                    cache: true,
                }),
                metadata: GenericMetadata::default(),
            },
            ExecDPrograms::Overwrite(HashMap::new()),
            Sboms::Keep,
        )
        .unwrap();

        assert!(!layer_dir.join("exec.d").exists());
    }

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

        // Add a bin directory to test if implicit entries are added to the LayerEnv
        fs::create_dir_all(layer_dir.join("bin")).unwrap();

        // Add a file to the env directory to test if explicit entries are added to the LayerEnv
        fs::create_dir_all(layer_dir.join("env")).unwrap();
        fs::write(layer_dir.join("env/CUSTOM_ENV"), "CUSTOM_ENV_VALUE").unwrap();

        let layer_data = super::read_layer::<TestLayerMetadata, _>(layers_dir, &layer_name)
            .unwrap()
            .unwrap();

        assert_eq!(layer_data.path, layer_dir);

        assert_eq!(layer_data.name, layer_name);

        assert_eq!(
            layer_data.content_metadata.types,
            Some(LayerTypes {
                launch: true,
                build: false,
                cache: true
            })
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

        let applied_layer_env = layer_data.env.apply_to_empty(Scope::Build);
        assert_eq!(
            applied_layer_env.get("PATH").cloned(),
            Some(layer_dir.join("bin").into())
        );

        assert_eq!(
            applied_layer_env.get("CUSTOM_ENV"),
            Some(&OsString::from("CUSTOM_ENV_VALUE"))
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
            r#"
            [types
            build = true
            launch = true
            cache = true
            "#,
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
                    layer_data.content_metadata,
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
