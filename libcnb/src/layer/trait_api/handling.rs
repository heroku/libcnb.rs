// This lint triggers when both layer_dir and layers_dir are present which are quite common.
#![allow(clippy::similar_names)]

use super::Layer;
use crate::Buildpack;
use crate::build::BuildContext;
use crate::data::layer::LayerName;
use crate::data::layer_content_metadata::LayerContentMetadata;
use crate::generic::GenericMetadata;
use crate::layer::shared::{
    ReadLayerError, WriteLayerError, delete_layer, replace_layer_exec_d_programs,
    replace_layer_sboms,
};
use crate::layer::{ExistingLayerStrategy, LayerData, LayerError, MetadataMigration};
use crate::layer_env::LayerEnv;
use crate::sbom::Sbom;
use serde::Serialize;
use serde::de::DeserializeOwned;
use std::collections::HashMap;
use std::fs;
use std::path::{Path, PathBuf};

pub(crate) fn handle_layer<B: Buildpack + ?Sized, L: Layer<Buildpack = B>>(
    context: &BuildContext<B>,
    layer_name: LayerName,
    mut layer: L,
) -> Result<LayerData<L::Metadata>, LayerErrorOrBuildpackError<B::Error>> {
    match read_layer(&context.layers_dir, &layer_name) {
        Ok(None) => handle_create_layer(context, &layer_name, &mut layer),
        Ok(Some(layer_data)) => {
            let existing_layer_strategy = layer
                .existing_layer_strategy(context, &layer_data)
                .map_err(LayerErrorOrBuildpackError::BuildpackError)?;

            match existing_layer_strategy {
                ExistingLayerStrategy::Recreate => {
                    delete_layer(&context.layers_dir, &layer_name).map_err(|error| {
                        LayerErrorOrBuildpackError::LayerError(LayerError::DeleteLayerError(error))
                    })?;

                    handle_create_layer(context, &layer_name, &mut layer)
                }
                ExistingLayerStrategy::Update => {
                    handle_update_layer(context, &layer_data, &mut layer)
                }
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
                    )
                    .map_err(|error| {
                        LayerErrorOrBuildpackError::LayerError(LayerError::WriteLayerError(error))
                    })?;

                    // Reread the layer from disk to ensure the returned layer data accurately reflects
                    // the state on disk after we messed with it.
                    read_layer(&context.layers_dir, &layer_name)
                        .map_err(|error| {
                            LayerErrorOrBuildpackError::LayerError(LayerError::ReadLayerError(
                                error,
                            ))
                        })?
                        .ok_or(LayerError::UnexpectedMissingLayer)
                        .map_err(LayerErrorOrBuildpackError::LayerError)
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
                        .map_err(LayerErrorOrBuildpackError::BuildpackError)?;

                    match metadata_migration_strategy {
                        MetadataMigration::RecreateLayer => {
                            delete_layer(&context.layers_dir, &layer_name).map_err(|error| {
                                LayerErrorOrBuildpackError::LayerError(
                                    LayerError::DeleteLayerError(error),
                                )
                            })?;
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
                            )
                            .map_err(|error| {
                                LayerErrorOrBuildpackError::LayerError(LayerError::WriteLayerError(
                                    error,
                                ))
                            })?;
                        }
                    }

                    handle_layer(context, layer_name, layer)
                }
                Ok(None) => Err(LayerErrorOrBuildpackError::LayerError(
                    LayerError::UnexpectedMissingLayer,
                )),
                Err(read_layer_error) => Err(LayerErrorOrBuildpackError::LayerError(
                    LayerError::ReadLayerError(read_layer_error),
                )),
            }
        }
        Err(read_layer_error) => Err(LayerErrorOrBuildpackError::LayerError(
            LayerError::ReadLayerError(read_layer_error),
        )),
    }
}

fn handle_create_layer<B: Buildpack + ?Sized, L: Layer<Buildpack = B>>(
    context: &BuildContext<B>,
    layer_name: &LayerName,
    layer: &mut L,
) -> Result<LayerData<L::Metadata>, LayerErrorOrBuildpackError<B::Error>> {
    let layer_dir = context.layers_dir.join(layer_name.as_str());

    fs::create_dir_all(&layer_dir)
        .map_err(LayerError::IoError)
        .map_err(LayerErrorOrBuildpackError::LayerError)?;

    let layer_result = layer
        .create(context, &layer_dir)
        .map_err(LayerErrorOrBuildpackError::BuildpackError)?;

    write_layer(
        &context.layers_dir,
        layer_name,
        &layer_result.env.unwrap_or_default(),
        &LayerContentMetadata {
            types: Some(layer.types()),
            metadata: layer_result.metadata,
        },
        ExecDPrograms::Replace(layer_result.exec_d_programs),
        Sboms::Replace(layer_result.sboms),
    )
    .map_err(|error| LayerErrorOrBuildpackError::LayerError(LayerError::WriteLayerError(error)))?;

    read_layer(&context.layers_dir, layer_name)
        .map_err(|error| LayerErrorOrBuildpackError::LayerError(LayerError::ReadLayerError(error)))?
        .ok_or(LayerError::UnexpectedMissingLayer)
        .map_err(LayerErrorOrBuildpackError::LayerError)
}

fn handle_update_layer<B: Buildpack + ?Sized, L: Layer<Buildpack = B>>(
    context: &BuildContext<B>,
    layer_data: &LayerData<L::Metadata>,
    layer: &mut L,
) -> Result<LayerData<L::Metadata>, LayerErrorOrBuildpackError<B::Error>> {
    let layer_result = layer
        .update(context, layer_data)
        .map_err(LayerErrorOrBuildpackError::BuildpackError)?;

    write_layer(
        &context.layers_dir,
        &layer_data.name,
        &layer_result.env.unwrap_or_default(),
        &LayerContentMetadata {
            types: Some(layer.types()),
            metadata: layer_result.metadata,
        },
        ExecDPrograms::Replace(layer_result.exec_d_programs),
        Sboms::Replace(layer_result.sboms),
    )
    .map_err(|error| LayerErrorOrBuildpackError::LayerError(LayerError::WriteLayerError(error)))?;

    read_layer(&context.layers_dir, &layer_data.name)
        .map_err(|error| LayerErrorOrBuildpackError::LayerError(LayerError::ReadLayerError(error)))?
        .ok_or(LayerError::UnexpectedMissingLayer)
        .map_err(LayerErrorOrBuildpackError::LayerError)
}

#[derive(Debug)]
pub(crate) enum LayerErrorOrBuildpackError<E> {
    LayerError(LayerError),
    BuildpackError(E),
}

#[derive(Debug)]
pub(in crate::layer) enum ExecDPrograms {
    Keep,
    Replace(HashMap<String, PathBuf>),
}

#[derive(Debug)]
pub(in crate::layer) enum Sboms {
    Keep,
    Replace(Vec<Sbom>),
}

/// Updates layer metadata on disk
pub(in crate::layer) fn write_layer<M: Serialize, P: AsRef<Path>>(
    layers_dir: P,
    layer_name: &LayerName,
    layer_env: &LayerEnv,
    layer_content_metadata: &LayerContentMetadata<M>,
    layer_exec_d_programs: ExecDPrograms,
    layer_sboms: Sboms,
) -> Result<(), WriteLayerError> {
    let layers_dir = layers_dir.as_ref();

    crate::layer::shared::write_layer(layers_dir, layer_name, layer_content_metadata)?;

    let layer_dir = layers_dir.join(layer_name.as_str());
    layer_env.write_to_layer_dir(layer_dir)?;

    if let Sboms::Replace(sboms) = layer_sboms {
        replace_layer_sboms(layers_dir, layer_name, &sboms)
            .map_err(WriteLayerError::ReplaceLayerSbomsError)?;
    }

    if let ExecDPrograms::Replace(exec_d_programs) = layer_exec_d_programs {
        replace_layer_exec_d_programs(layers_dir, layer_name, &exec_d_programs)
            .map_err(WriteLayerError::ReplaceLayerExecdProgramsError)?;
    }

    Ok(())
}

pub(crate) fn read_layer<M: DeserializeOwned, P: AsRef<Path>>(
    layers_dir: P,
    layer_name: &LayerName,
) -> Result<Option<LayerData<M>>, ReadLayerError> {
    crate::layer::shared::read_layer(layers_dir, layer_name).and_then(|layer| {
        layer
            .map(|layer| {
                LayerEnv::read_from_layer_dir(&layer.path)
                    .map_err(ReadLayerError::IoError)
                    .map(|env| LayerData {
                        name: layer.name,
                        path: layer.path,
                        env,
                        content_metadata: layer.metadata,
                    })
            })
            .transpose()
    })
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::data::layer_content_metadata::{LayerContentMetadata, LayerTypes};
    use crate::data::layer_name;
    use crate::generic::GenericMetadata;
    use crate::layer::shared::ReplaceLayerExecdProgramsError;
    use crate::layer_env::{ModificationBehavior, Scope};
    use crate::read_toml_file;
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
            r"
            [types]
            launch = true
            build = false
            cache = true
            ",
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
            r"
            [types]
            launch = true
            build = false
            cache = true
            ",
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
            ExecDPrograms::Replace(HashMap::from([(String::from("foo"), foo_execd_file)])),
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
            ExecDPrograms::Replace(HashMap::from([(String::from("foo"), execd_file.clone())])),
            Sboms::Keep,
        )
        .unwrap_err();

        match write_layer_error {
            WriteLayerError::ReplaceLayerExecdProgramsError(
                ReplaceLayerExecdProgramsError::MissingExecDFile(path),
            ) => {
                assert_eq!(path, execd_file);
            }
            other => {
                panic!("Expected WriteLayerError::MissingExecDFile, but got {other:?}");
            }
        }
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
            ExecDPrograms::Replace(HashMap::from([(String::from("foo"), foo_execd_file)])),
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
            ExecDPrograms::Replace(HashMap::from([
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
            ExecDPrograms::Replace(HashMap::from([(String::from("foo"), foo_execd_file)])),
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
            ExecDPrograms::Replace(HashMap::from([(String::from("foo"), foo_execd_file)])),
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
            ExecDPrograms::Replace(HashMap::new()),
            Sboms::Keep,
        )
        .unwrap();

        assert!(!layer_dir.join("exec.d").exists());
    }
}
