use crate::Buildpack;
use crate::layer::shared::{
    ReadLayer, ReadLayerError, WriteLayerError, delete_layer, read_layer, replace_layer_metadata,
    replace_layer_types,
};
use crate::layer::{
    EmptyLayerCause, IntoAction, InvalidMetadataAction, LayerError, LayerRef, LayerState,
    RestoredLayerAction,
};
use libcnb_common::toml_file::read_toml_file;
use libcnb_data::generic::GenericMetadata;
use libcnb_data::layer::LayerName;
use libcnb_data::layer_content_metadata::{LayerContentMetadata, LayerTypes};
use serde::Serialize;
use serde::de::DeserializeOwned;
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
    // Errors aren't traced here. They're traced by the `libcnb-build-error` event instead.
    let (layer_ref, metadata_migrated) = resolve_layer(
        layer_types,
        invalid_metadata_action_fn,
        restored_layer_action_fn,
        layer_name,
        layers_dir,
    )?;

    trace_layer(layer_name, layer_types, &layer_ref.state, metadata_migrated);

    Ok(layer_ref)
}

/// A layer, and whether its metadata was replaced via [`InvalidMetadataAction::ReplaceMetadata`].
type ResolvedLayer<B, MAC, RAC> = (LayerRef<B, MAC, RAC>, bool);

fn resolve_layer<B, M, MA, RA, MAC, RAC>(
    layer_types: LayerTypes,
    invalid_metadata_action_fn: &dyn Fn(&GenericMetadata) -> MA,
    restored_layer_action_fn: &dyn Fn(&M, &Path) -> RA,
    layer_name: &LayerName,
    layers_dir: &Path,
) -> crate::Result<ResolvedLayer<B, MAC, RAC>, B::Error>
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
            EmptyLayerCause::NewlyCreated,
        )
        .map(|layer_ref| (layer_ref, false)),
        Ok(Some(layer_data)) => handle_restored_layer(
            layer_types,
            restored_layer_action_fn,
            layer_data,
            layer_name,
            layers_dir,
        )
        .map(|layer_ref| (layer_ref, false)),
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
                        EmptyLayerCause::InvalidMetadataAction { cause },
                    )
                    .map(|layer_ref| (layer_ref, false))
                }
                (InvalidMetadataAction::ReplaceMetadata(metadata), _) => {
                    replace_layer_metadata(layers_dir, layer_name, metadata).map_err(|error| {
                        LayerError::WriteLayerError(WriteLayerError::WriteLayerMetadataError(error))
                    })?;

                    let layer_data = read_layer::<M, _>(layers_dir, layer_name)
                        .map_err(LayerError::ReplacementMetadataReadError)?
                        .ok_or(LayerError::UnexpectedMissingLayer)?;

                    handle_restored_layer(
                        layer_types,
                        restored_layer_action_fn,
                        layer_data,
                        layer_name,
                        layers_dir,
                    )
                    .map(|layer_ref| (layer_ref, true))
                }
            }
        }
        Err(read_layer_error) => Err(LayerError::ReadLayerError(read_layer_error))?,
    }
}

fn handle_restored_layer<B, M, RA, MAC, RAC>(
    layer_types: LayerTypes,
    restored_layer_action_fn: &dyn Fn(&M, &Path) -> RA,
    layer_data: ReadLayer<M>,
    layer_name: &LayerName,
    layers_dir: &Path,
) -> crate::Result<LayerRef<B, MAC, RAC>, B::Error>
where
    B: Buildpack + ?Sized,
    RA: IntoAction<RestoredLayerAction, RAC, B::Error>,
{
    let restored_layer_action =
        restored_layer_action_fn(&layer_data.metadata.metadata, &layer_data.path)
            .into_action()
            .map_err(crate::Error::BuildpackError)?;

    match restored_layer_action {
        (RestoredLayerAction::DeleteLayer, cause) => {
            delete_layer(layers_dir, layer_name).map_err(LayerError::DeleteLayerError)?;

            create_layer(
                layer_types,
                layer_name,
                layers_dir,
                EmptyLayerCause::RestoredLayerAction { cause },
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

/// Emits an event describing the caching outcome of a layer.
#[cfg(feature = "trace")]
fn trace_layer<MAC, RAC>(
    layer_name: &LayerName,
    layer_types: LayerTypes,
    state: &LayerState<MAC, RAC>,
    metadata_migrated: bool,
) {
    let (layer_state, empty_cause) = match state {
        LayerState::Restored { .. } => ("restored", None),
        LayerState::Empty { cause } => (
            "empty",
            Some(match cause {
                EmptyLayerCause::NewlyCreated => "newly_created",
                EmptyLayerCause::InvalidMetadataAction { .. } => "invalid_metadata_action",
                EmptyLayerCause::RestoredLayerAction { .. } => "restored_layer_action",
            }),
        ),
    };

    tracing::info!(
        layer_name = layer_name.as_str(),
        layer_build = layer_types.build,
        layer_launch = layer_types.launch,
        layer_cache = layer_types.cache,
        layer_state,
        layer_empty_cause = empty_cause,
        layer_metadata_migrated = metadata_migrated,
        "libcnb-layer"
    );
}

#[cfg(not(feature = "trace"))]
fn trace_layer<MAC, RAC>(_: &LayerName, _: LayerTypes, _: &LayerState<MAC, RAC>, _: bool) {}

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
    use crate::Buildpack;
    use crate::build::{BuildContext, BuildResult};
    use crate::detect::{DetectContext, DetectResult};
    use crate::generic::{GenericError, GenericPlatform};
    use crate::layer::{
        EmptyLayerCause, InvalidMetadataAction, LayerError, LayerState, ReadLayerError,
        RestoredLayerAction,
    };
    use libcnb_common::toml_file::read_toml_file;
    use libcnb_data::generic::GenericMetadata;
    use libcnb_data::layer_content_metadata::{LayerContentMetadata, LayerTypes};
    use libcnb_data::layer_name;
    use serde::{Deserialize, Serialize};
    #[cfg(feature = "trace")]
    use std::collections::BTreeMap;
    use tempfile::tempdir;
    use toml::toml;

    #[test]
    fn create_layer() {
        let temp_dir = tempdir().unwrap();

        let cause = EmptyLayerCause::RestoredLayerAction { cause: () };
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
                cause: EmptyLayerCause::NewlyCreated
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
                cause: EmptyLayerCause::RestoredLayerAction {
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
                cause: EmptyLayerCause::InvalidMetadataAction {
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

    #[test]
    fn handle_layer_cached_invalid_metadata_replace_with_unreadable_metadata() {
        // Serializes to TOML that it can't deserialize from, so replacement metadata stays invalid.
        #[derive(Serialize, Deserialize)]
        struct TestLayerMetadata {
            #[serde(rename(serialize = "written"))]
            read: String,
        }

        let temp_dir = tempdir().unwrap();
        let layer_name = layer_name!("test_layer");

        std::fs::create_dir_all(temp_dir.path().join(&*layer_name)).unwrap();
        std::fs::write(
            temp_dir.path().join(format!("{layer_name}.toml")),
            "[metadata]\nanswer=42",
        )
        .unwrap();

        let invalid_metadata_action_calls = std::cell::Cell::new(0);
        let result = handle_layer::<TestBuildpack, TestLayerMetadata, _, RestoredLayerAction, (), ()>(
            LayerTypes {
                build: true,
                launch: true,
                cache: true,
            },
            &|_| {
                invalid_metadata_action_calls.set(invalid_metadata_action_calls.get() + 1);
                InvalidMetadataAction::ReplaceMetadata(TestLayerMetadata {
                    read: String::from("LV-246"),
                })
            },
            &|_, _| panic!("restored_layer_action callback should not be called!"),
            &layer_name,
            temp_dir.path(),
        );

        assert!(matches!(
            result,
            Err(crate::Error::LayerError(
                LayerError::ReplacementMetadataReadError(
                    ReadLayerError::LayerContentMetadataParseError(_)
                )
            ))
        ));
        assert_eq!(invalid_metadata_action_calls.get(), 1);
    }

    #[cfg(feature = "trace")]
    const TRACED_LAYER_TYPES: LayerTypes = LayerTypes {
        build: true,
        launch: false,
        cache: true,
    };

    #[cfg(feature = "trace")]
    #[derive(Serialize, Deserialize)]
    struct NotAnswerMetadata {
        planet: String,
    }

    #[cfg(feature = "trace")]
    fn write_restored_layer(layers_dir: &std::path::Path, metadata: &str) {
        std::fs::create_dir_all(layers_dir.join("test_layer")).unwrap();
        std::fs::write(layers_dir.join("test_layer.toml"), metadata).unwrap();
    }

    #[cfg(feature = "trace")]
    fn record_layer_events(f: impl FnOnce()) -> Vec<BTreeMap<&'static str, String>> {
        use std::sync::{Arc, Mutex};
        use tracing::field::{Field, Visit};
        use tracing_subscriber::layer::{Context, SubscriberExt};

        #[derive(Default)]
        struct Fields(BTreeMap<&'static str, String>);

        impl Visit for Fields {
            fn record_bool(&mut self, field: &Field, value: bool) {
                self.0.insert(field.name(), value.to_string());
            }

            fn record_str(&mut self, field: &Field, value: &str) {
                self.0.insert(field.name(), value.to_string());
            }

            fn record_debug(&mut self, field: &Field, value: &dyn std::fmt::Debug) {
                self.0.insert(field.name(), format!("{value:?}"));
            }
        }

        #[derive(Clone, Default)]
        struct Recorder(Arc<Mutex<Vec<BTreeMap<&'static str, String>>>>);

        impl<S: tracing::Subscriber> tracing_subscriber::Layer<S> for Recorder {
            fn on_event(&self, event: &tracing::Event<'_>, _: Context<'_, S>) {
                let mut fields = Fields::default();
                event.record(&mut fields);
                if fields.0.remove("message").as_deref() == Some("libcnb-layer") {
                    self.0.lock().unwrap().push(fields.0);
                }
            }
        }

        // Keeps parallel tests from hiding events from this recorder: <https://github.com/tokio-rs/tracing/issues/2874>
        let _second_dispatcher =
            tracing::Dispatch::new(tracing::subscriber::NoSubscriber::default());

        let recorder = Recorder::default();
        tracing::subscriber::with_default(tracing_subscriber::registry().with(recorder.clone()), f);
        recorder.0.lock().unwrap().clone()
    }

    #[cfg(feature = "trace")]
    fn layer_event(
        layer_state: &str,
        layer_empty_cause: Option<&str>,
        layer_metadata_migrated: bool,
    ) -> BTreeMap<&'static str, String> {
        let mut fields = BTreeMap::from([
            ("layer_name", "test_layer".to_string()),
            ("layer_build", "true".to_string()),
            ("layer_launch", "false".to_string()),
            ("layer_cache", "true".to_string()),
            ("layer_state", layer_state.to_string()),
            (
                "layer_metadata_migrated",
                layer_metadata_migrated.to_string(),
            ),
        ]);
        if let Some(layer_empty_cause) = layer_empty_cause {
            fields.insert("layer_empty_cause", layer_empty_cause.to_string());
        }
        fields
    }

    #[cfg(feature = "trace")]
    #[test]
    fn handle_layer_traces_newly_created_layer() {
        let temp_dir = tempdir().unwrap();

        let events = record_layer_events(|| {
            handle_layer::<
                TestBuildpack,
                GenericMetadata,
                InvalidMetadataAction<GenericMetadata>,
                RestoredLayerAction,
                (),
                (),
            >(
                TRACED_LAYER_TYPES,
                &|_| panic!("invalid_metadata_action callback should not be called!"),
                &|_, _| panic!("restored_layer_action callback should not be called!"),
                &layer_name!("test_layer"),
                temp_dir.path(),
            )
            .unwrap();
        });

        assert_eq!(events, [layer_event("empty", Some("newly_created"), false)]);
    }

    #[cfg(feature = "trace")]
    #[test]
    fn handle_layer_traces_restored_layer_kept() {
        let temp_dir = tempdir().unwrap();
        write_restored_layer(temp_dir.path(), "[metadata]\nanswer=42");

        let events = record_layer_events(|| {
            handle_layer::<TestBuildpack, GenericMetadata, InvalidMetadataAction<_>, _, (), ()>(
                TRACED_LAYER_TYPES,
                &|_| panic!("invalid_metadata_action callback should not be called!"),
                &|_, _| RestoredLayerAction::KeepLayer,
                &layer_name!("test_layer"),
                temp_dir.path(),
            )
            .unwrap();
        });

        assert_eq!(events, [layer_event("restored", None, false)]);
    }

    #[cfg(feature = "trace")]
    #[test]
    fn handle_layer_traces_restored_layer_deleted() {
        let temp_dir = tempdir().unwrap();
        write_restored_layer(temp_dir.path(), "[metadata]\nanswer=42");

        let events = record_layer_events(|| {
            handle_layer::<TestBuildpack, GenericMetadata, InvalidMetadataAction<_>, _, (), ()>(
                TRACED_LAYER_TYPES,
                &|_| panic!("invalid_metadata_action callback should not be called!"),
                &|_, _| RestoredLayerAction::DeleteLayer,
                &layer_name!("test_layer"),
                temp_dir.path(),
            )
            .unwrap();
        });

        assert_eq!(
            events,
            [layer_event("empty", Some("restored_layer_action"), false)]
        );
    }

    #[cfg(feature = "trace")]
    #[test]
    fn handle_layer_traces_invalid_metadata_deleted() {
        let temp_dir = tempdir().unwrap();
        write_restored_layer(temp_dir.path(), "[metadata]\nanswer=42");

        let events = record_layer_events(|| {
            handle_layer::<TestBuildpack, NotAnswerMetadata, _, RestoredLayerAction, (), ()>(
                TRACED_LAYER_TYPES,
                &|_| InvalidMetadataAction::DeleteLayer,
                &|_, _| panic!("restored_layer_action callback should not be called!"),
                &layer_name!("test_layer"),
                temp_dir.path(),
            )
            .unwrap();
        });

        assert_eq!(
            events,
            [layer_event("empty", Some("invalid_metadata_action"), false)]
        );
    }

    #[cfg(feature = "trace")]
    #[test]
    fn handle_layer_traces_invalid_metadata_replaced_and_kept() {
        let temp_dir = tempdir().unwrap();
        write_restored_layer(temp_dir.path(), "[metadata]\nanswer=42");

        let events = record_layer_events(|| {
            handle_layer::<TestBuildpack, NotAnswerMetadata, _, _, (), ()>(
                TRACED_LAYER_TYPES,
                &|_| {
                    InvalidMetadataAction::ReplaceMetadata(NotAnswerMetadata {
                        planet: String::from("LV-246"),
                    })
                },
                &|_, _| RestoredLayerAction::KeepLayer,
                &layer_name!("test_layer"),
                temp_dir.path(),
            )
            .unwrap();
        });

        assert_eq!(events, [layer_event("restored", None, true)]);
    }

    #[cfg(feature = "trace")]
    #[test]
    fn handle_layer_traces_invalid_metadata_replaced_and_deleted() {
        let temp_dir = tempdir().unwrap();
        write_restored_layer(temp_dir.path(), "[metadata]\nanswer=42");

        let events = record_layer_events(|| {
            handle_layer::<TestBuildpack, NotAnswerMetadata, _, _, (), ()>(
                TRACED_LAYER_TYPES,
                &|_| {
                    InvalidMetadataAction::ReplaceMetadata(NotAnswerMetadata {
                        planet: String::from("LV-246"),
                    })
                },
                &|_, _| RestoredLayerAction::DeleteLayer,
                &layer_name!("test_layer"),
                temp_dir.path(),
            )
            .unwrap();
        });

        assert_eq!(
            events,
            [layer_event("empty", Some("restored_layer_action"), true)]
        );
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
