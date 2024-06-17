use crate::layer::shared::{
    delete_layer, read_layer, replace_layer_metadata, replace_layer_types, ReadLayerError,
    WriteLayerError,
};
use crate::layer::{
    EmptyLayerCause, IntoAction, InvalidMetadataAction, LayerError, LayerRef, LayerState,
    RestoredLayerAction,
};
use crate::Buildpack;
use libcnb_common::toml_file::read_toml_file;
use libcnb_data::generic::GenericMetadata;
use libcnb_data::layer::LayerName;
use libcnb_data::layer_content_metadata::{LayerContentMetadata, LayerTypes};
use serde::de::DeserializeOwned;
use serde::Serialize;
use std::marker::PhantomData;
use std::path::{Path, PathBuf};

pub(crate) fn handle_layer<B, M, MA, RA, MAC, RAC>(
    layer_types: LayerTypes,
    invalid_metadata_action_fn: &dyn Fn(&GenericMetadata) -> MA,
    restored_layer_action_fn: &dyn Fn(&M, &Path) -> RA,
    layer_name: &LayerName,
    layers_dir: &Path,
) -> crate::Result<LayerRef<B, MAC, RAC>, B::Error>
where
    B: Buildpack + ?Sized,
    M: Serialize + DeserializeOwned,
    MA: IntoAction<InvalidMetadataAction<M>, MAC, B::Error>,
    RA: IntoAction<RestoredLayerAction, RAC, B::Error>,
{
    match read_layer::<M, _>(layers_dir, layer_name) {
        Ok(None) => create_layer(
            layer_types,
            layer_name,
            layers_dir,
            EmptyLayerCause::Uncached,
        ),
        Ok(Some(layer_data)) => {
            let inspect_action =
                restored_layer_action_fn(&layer_data.metadata.metadata, &layer_data.path)
                    .into_action()
                    .map_err(crate::Error::BuildpackError)?;

            match inspect_action {
                (RestoredLayerAction::DeleteLayer, cause) => {
                    delete_layer(layers_dir, layer_name).map_err(LayerError::DeleteLayerError)?;

                    create_layer(
                        layer_types,
                        layer_name,
                        layers_dir,
                        EmptyLayerCause::Inspect { cause },
                    )
                }
                (RestoredLayerAction::KeepLayer, cause) => {
                    // Always write the layer types as:
                    // a) they might be different from what is currently on disk
                    // b) the cache field will be removed by CNB lifecycle on cache restore
                    replace_layer_types(layers_dir, layer_name, layer_types).map_err(|error| {
                        LayerError::WriteLayerError(WriteLayerError::WriteLayerMetadataError(error))
                    })?;

                    Ok(LayerRef {
                        name: layer_data.name,
                        layers_dir: PathBuf::from(layers_dir),
                        buildpack: PhantomData,
                        state: LayerState::Restored { cause },
                    })
                }
            }
        }
        Err(ReadLayerError::LayerContentMetadataParseError(_)) => {
            let layer_content_metadata = read_toml_file::<LayerContentMetadata>(
                layers_dir.join(format!("{layer_name}.toml")),
            )
            .map_err(LayerError::CouldNotReadGenericLayerMetadata)?;

            let invalid_metadata_action =
                invalid_metadata_action_fn(&layer_content_metadata.metadata)
                    .into_action()
                    .map_err(crate::Error::BuildpackError)?;

            match invalid_metadata_action {
                (InvalidMetadataAction::DeleteLayer, cause) => {
                    delete_layer(layers_dir, layer_name).map_err(LayerError::DeleteLayerError)?;

                    create_layer(
                        layer_types,
                        layer_name,
                        layers_dir,
                        EmptyLayerCause::MetadataInvalid { cause },
                    )
                }
                (InvalidMetadataAction::ReplaceMetadata(metadata), _) => {
                    replace_layer_metadata(layers_dir, layer_name, metadata).map_err(|error| {
                        LayerError::WriteLayerError(WriteLayerError::WriteLayerMetadataError(error))
                    })?;

                    handle_layer(
                        layer_types,
                        invalid_metadata_action_fn,
                        restored_layer_action_fn,
                        layer_name,
                        layers_dir,
                    )
                }
            }
        }
        Err(read_layer_error) => Err(LayerError::ReadLayerError(read_layer_error))?,
    }
}

fn create_layer<B, MAC, RAC>(
    layer_types: LayerTypes,
    layer_name: &LayerName,
    layers_dir: &Path,
    empty_layer_cause: EmptyLayerCause<MAC, RAC>,
) -> Result<LayerRef<B, MAC, RAC>, crate::Error<B::Error>>
where
    B: Buildpack + ?Sized,
{
    crate::layer::shared::write_layer(
        layers_dir,
        layer_name,
        &LayerContentMetadata {
            types: Some(layer_types),
            metadata: GenericMetadata::default(),
        },
    )
    .map_err(LayerError::WriteLayerError)?;

    let layer_data = read_layer::<GenericMetadata, _>(layers_dir, layer_name)
        .map_err(LayerError::ReadLayerError)?
        .ok_or(LayerError::CouldNotReadLayerAfterCreate(layer_name.clone()))?;

    Ok(LayerRef {
        name: layer_data.name,
        layers_dir: PathBuf::from(layers_dir),
        buildpack: PhantomData,
        state: LayerState::Empty {
            cause: empty_layer_cause,
        },
    })
}

#[cfg(test)]
mod tests {
    use super::handle_layer;
    use crate::build::{BuildContext, BuildResult};
    use crate::detect::{DetectContext, DetectResult};
    use crate::generic::{GenericError, GenericPlatform};
    use crate::layer::{EmptyLayerCause, InvalidMetadataAction, LayerState, RestoredLayerAction};
    use crate::Buildpack;
    use libcnb_common::toml_file::read_toml_file;
    use libcnb_data::generic::GenericMetadata;
    use libcnb_data::layer_content_metadata::{LayerContentMetadata, LayerTypes};
    use libcnb_data::layer_name;
    use serde::{Deserialize, Serialize};
    use tempfile::tempdir;
    use toml::toml;

    #[test]
    fn create_layer() {
        let temp_dir = tempdir().unwrap();

        let cause = EmptyLayerCause::Inspect { cause: () };
        let layer_name = layer_name!("test_layer");
        let layer_ref = super::create_layer::<TestBuildpack, (), ()>(
            LayerTypes {
                launch: true,
                build: true,
                cache: false,
            },
            &layer_name,
            temp_dir.path(),
            cause,
        )
        .unwrap();

        assert_eq!(layer_ref.layers_dir, temp_dir.path());
        assert_eq!(layer_ref.state, LayerState::Empty { cause });
        assert!(temp_dir.path().join(&*layer_name).is_dir());
        assert_eq!(
            read_toml_file::<LayerContentMetadata<GenericMetadata>>(
                temp_dir.path().join(format!("{layer_name}.toml"))
            )
            .unwrap(),
            LayerContentMetadata {
                types: Some(LayerTypes {
                    launch: true,
                    build: true,
                    cache: false,
                }),
                metadata: GenericMetadata::default()
            }
        );
    }

    #[test]
    fn handle_layer_uncached() {
        let temp_dir = tempdir().unwrap();

        let layer_name = layer_name!("test_layer");
        let layer_ref = handle_layer::<
            TestBuildpack,
            GenericMetadata,
            InvalidMetadataAction<GenericMetadata>,
            RestoredLayerAction,
            (),
            (),
        >(
            LayerTypes {
                build: true,
                launch: true,
                cache: true,
            },
            &|_| panic!("invalid_metadata_action callback should not be called!"),
            &|_, _| panic!("restored_layer_action callback should not be called!"),
            &layer_name,
            temp_dir.path(),
        )
        .unwrap();

        assert_eq!(layer_ref.path(), temp_dir.path().join(&*layer_name));
        assert!(layer_ref.path().is_dir());
        assert_eq!(
            read_toml_file::<LayerContentMetadata<GenericMetadata>>(
                temp_dir.path().join(format!("{layer_name}.toml"))
            )
            .unwrap(),
            LayerContentMetadata {
                types: Some(LayerTypes {
                    build: true,
                    launch: true,
                    cache: true,
                }),
                metadata: GenericMetadata::default()
            }
        );
        assert_eq!(
            layer_ref.state,
            LayerState::Empty {
                cause: EmptyLayerCause::Uncached
            }
        );
    }

    #[test]
    fn handle_layer_cached_keep() {
        const KEEP_CAUSE: &str = "cause";

        let temp_dir = tempdir().unwrap();
        let layer_name = layer_name!("test_layer");

        // Create a layer as if it was restored by the CNB lifecycle, most notably WITHOUT layer
        // types but WITH metadata.
        std::fs::create_dir_all(temp_dir.path().join(&*layer_name)).unwrap();
        std::fs::write(
            temp_dir.path().join(format!("{layer_name}.toml")),
            "[metadata]\nanswer=42",
        )
        .unwrap();

        let layer_ref =
            handle_layer::<TestBuildpack, _, InvalidMetadataAction<GenericMetadata>, _, (), _>(
                LayerTypes {
                    build: true,
                    launch: true,
                    cache: true,
                },
                &|_| panic!("invalid_metadata_action callback should not be called!"),
                &|metadata, path| {
                    assert_eq!(metadata, &Some(toml! { answer = 42 }));
                    assert_eq!(path, temp_dir.path().join(&*layer_name.clone()));
                    (RestoredLayerAction::KeepLayer, KEEP_CAUSE)
                },
                &layer_name,
                temp_dir.path(),
            )
            .unwrap();

        assert_eq!(layer_ref.path(), temp_dir.path().join(&*layer_name));
        assert!(layer_ref.path().is_dir());
        assert_eq!(
            read_toml_file::<LayerContentMetadata<_>>(
                temp_dir.path().join(format!("{layer_name}.toml"))
            )
            .unwrap(),
            LayerContentMetadata {
                types: Some(LayerTypes {
                    build: true,
                    launch: true,
                    cache: true,
                }),
                metadata: Some(toml! { answer = 42 })
            }
        );
        assert_eq!(layer_ref.state, LayerState::Restored { cause: KEEP_CAUSE });
    }

    #[test]
    fn handle_layer_cached_delete() {
        const DELETE_CAUSE: &str = "cause";

        let temp_dir = tempdir().unwrap();
        let layer_name = layer_name!("test_layer");

        // Create a layer as if it was restored by the CNB lifecycle, most notably WITHOUT layer
        // types but WITH metadata.
        std::fs::create_dir_all(temp_dir.path().join(&*layer_name)).unwrap();
        std::fs::write(
            temp_dir.path().join(format!("{layer_name}.toml")),
            "[metadata]\nanswer=42",
        )
        .unwrap();

        let layer_ref =
            handle_layer::<TestBuildpack, _, InvalidMetadataAction<GenericMetadata>, _, (), _>(
                LayerTypes {
                    build: true,
                    launch: true,
                    cache: true,
                },
                &|_| panic!("invalid_metadata_action callback should not be called!"),
                &|metadata, path| {
                    assert_eq!(metadata, &Some(toml! { answer = 42 }));
                    assert_eq!(path, temp_dir.path().join(&*layer_name.clone()));
                    (RestoredLayerAction::DeleteLayer, DELETE_CAUSE)
                },
                &layer_name,
                temp_dir.path(),
            )
            .unwrap();

        assert_eq!(layer_ref.path(), temp_dir.path().join(&*layer_name));
        assert!(layer_ref.path().is_dir());
        assert_eq!(
            read_toml_file::<LayerContentMetadata<_>>(
                temp_dir.path().join(format!("{layer_name}.toml"))
            )
            .unwrap(),
            LayerContentMetadata {
                types: Some(LayerTypes {
                    build: true,
                    launch: true,
                    cache: true,
                }),
                metadata: GenericMetadata::default()
            }
        );
        assert_eq!(
            layer_ref.state,
            LayerState::Empty {
                cause: EmptyLayerCause::Inspect {
                    cause: DELETE_CAUSE
                }
            }
        );
    }

    #[test]
    fn handle_layer_cached_invalid_metadata_delete() {
        const DELETE_CAUSE: &str = "cause";

        #[derive(Serialize, Deserialize)]
        struct TestLayerMetadata {
            planet: String,
        }

        let temp_dir = tempdir().unwrap();
        let layer_name = layer_name!("test_layer");

        // Create a layer as if it was restored by the CNB lifecycle, most notably WITHOUT layer
        // types but WITH metadata.
        std::fs::create_dir_all(temp_dir.path().join(&*layer_name)).unwrap();
        std::fs::write(
            temp_dir.path().join(format!("{layer_name}.toml")),
            "[metadata]\nanswer=42",
        )
        .unwrap();

        let layer_ref = handle_layer::<
            TestBuildpack,
            TestLayerMetadata,
            _,
            (RestoredLayerAction, &str),
            &str,
            _,
        >(
            LayerTypes {
                build: true,
                launch: true,
                cache: true,
            },
            &|metadata| {
                assert_eq!(metadata, &Some(toml! { answer = 42 }));
                (InvalidMetadataAction::DeleteLayer, DELETE_CAUSE)
            },
            &|_, _| panic!("restored_layer_action callback should not be called!"),
            &layer_name,
            temp_dir.path(),
        )
        .unwrap();

        assert_eq!(layer_ref.path(), temp_dir.path().join(&*layer_name));
        assert!(layer_ref.path().is_dir());
        assert_eq!(
            read_toml_file::<LayerContentMetadata<_>>(
                temp_dir.path().join(format!("{layer_name}.toml"))
            )
            .unwrap(),
            LayerContentMetadata {
                types: Some(LayerTypes {
                    build: true,
                    launch: true,
                    cache: true,
                }),
                metadata: GenericMetadata::default()
            }
        );
        assert_eq!(
            layer_ref.state,
            LayerState::Empty {
                cause: EmptyLayerCause::MetadataInvalid {
                    cause: DELETE_CAUSE
                }
            }
        );
    }

    #[test]
    fn handle_layer_cached_invalid_metadata_replace() {
        const KEEP_CAUSE: &str = "cause";

        #[derive(Serialize, Deserialize, Debug, Eq, PartialEq)]
        struct TestLayerMetadata {
            planet: String,
        }

        let temp_dir = tempdir().unwrap();
        let layer_name = layer_name!("test_layer");

        // Create a layer as if it was restored by the CNB lifecycle, most notably WITHOUT layer
        // types but WITH metadata.
        std::fs::create_dir_all(temp_dir.path().join(&*layer_name)).unwrap();
        std::fs::write(
            temp_dir.path().join(&*layer_name).join("data.txt"),
            "some_data",
        )
        .unwrap();
        std::fs::write(
            temp_dir.path().join(format!("{layer_name}.toml")),
            "[metadata]\nanswer=42",
        )
        .unwrap();

        let layer_ref = handle_layer::<TestBuildpack, TestLayerMetadata, _, _, _, _>(
            LayerTypes {
                build: true,
                launch: true,
                cache: true,
            },
            &|metadata| {
                assert_eq!(metadata, &Some(toml! { answer = 42 }));

                InvalidMetadataAction::ReplaceMetadata(TestLayerMetadata {
                    planet: String::from("LV-246"),
                })
            },
            &|metadata, _| {
                assert_eq!(
                    metadata,
                    &TestLayerMetadata {
                        planet: String::from("LV-246"),
                    }
                );

                (RestoredLayerAction::KeepLayer, KEEP_CAUSE)
            },
            &layer_name,
            temp_dir.path(),
        )
        .unwrap();

        assert_eq!(layer_ref.path(), temp_dir.path().join(&*layer_name));
        assert!(layer_ref.path().is_dir());
        assert_eq!(
            read_toml_file::<LayerContentMetadata<_>>(
                temp_dir.path().join(format!("{layer_name}.toml"))
            )
            .unwrap(),
            LayerContentMetadata {
                types: Some(LayerTypes {
                    build: true,
                    launch: true,
                    cache: true,
                }),
                metadata: TestLayerMetadata {
                    planet: String::from("LV-246")
                }
            }
        );

        assert_eq!(layer_ref.state, LayerState::Restored { cause: KEEP_CAUSE });
    }

    struct TestBuildpack;
    impl Buildpack for TestBuildpack {
        type Platform = GenericPlatform;
        type Metadata = GenericMetadata;
        type Error = GenericError;

        fn detect(&self, _: DetectContext<Self>) -> crate::Result<DetectResult, Self::Error> {
            unimplemented!()
        }

        fn build(&self, _: BuildContext<Self>) -> crate::Result<BuildResult, Self::Error> {
            unimplemented!()
        }
    }
}
