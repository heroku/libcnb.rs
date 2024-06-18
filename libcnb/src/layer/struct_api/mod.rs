pub(crate) mod handling;

// BuildContext is only used in RustDoc (https://github.com/rust-lang/rust/issues/79542)
#[allow(unused)]
use crate::build::BuildContext;
use crate::layer::shared::{replace_layer_exec_d_programs, replace_layer_sboms, WriteLayerError};
use crate::layer::{LayerError, ReadLayerError};
use crate::layer_env::LayerEnv;
use crate::sbom::Sbom;
use crate::Buildpack;
use libcnb_data::generic::GenericMetadata;
use libcnb_data::layer::LayerName;
use serde::Serialize;
use std::borrow::Borrow;
use std::collections::HashMap;
use std::marker::PhantomData;
use std::path::{Path, PathBuf};

/// A definition for a cached layer.
///
/// Refer to the docs of [`BuildContext::cached_layer`] for usage examples.
pub struct CachedLayerDefinition<'a, M, MA, RA> {
    /// Whether the layer is intended for build.
    pub build: bool,
    /// Whether the layer is intended for launch.
    pub launch: bool,
    /// Callback for when the metadata of a restored layer cannot be parsed as `M`.
    ///
    /// Allows replacing the metadata before continuing (i.e. migration to a newer version) or
    /// deleting the layer.
    pub invalid_metadata_action: &'a dyn Fn(&GenericMetadata) -> MA,
    /// Callback when the layer was restored from cache to validate the contents and metadata.
    /// Can be used to delete existing cached layers.
    pub restored_layer_action: &'a dyn Fn(&M, &Path) -> RA,
}

/// A definition for an uncached layer.
///
/// Refer to the docs of [`BuildContext::uncached_layer`] for usage examples.
pub struct UncachedLayerDefinition {
    /// Whether the layer is intended for build.
    pub build: bool,
    /// Whether the layer is intended for launch.
    pub launch: bool,
}

/// The action to take when the layer metadata is invalid.
#[derive(Copy, Clone, Debug)]
pub enum InvalidMetadataAction<M> {
    /// Delete the existing layer.
    DeleteLayer,
    /// Keep the layer, but replace the metadata. Commonly used to migrate to a newer
    /// metadata format.
    ReplaceMetadata(M),
}

/// The action to take when a previously cached layer was restored.
#[derive(Copy, Clone, Debug)]
pub enum RestoredLayerAction {
    /// Delete the restored layer.
    DeleteLayer,
    /// Keep the restored layer. It can then be used as-is or updated if required.
    KeepLayer,
}

/// Framework metadata about the layer state.
///
/// See: [`BuildContext::cached_layer`] and [`BuildContext::uncached_layer`]
#[derive(Copy, Clone, Debug, Eq, PartialEq)]
pub enum LayerState<MAC, RAC> {
    /// The layer contains validated cached contents from a previous buildpack run.
    ///
    /// See: `restored_layer_action` in [`CachedLayerDefinition`].
    Restored { cause: RAC },
    /// The layer is empty. Inspect the contained [`EmptyLayerCause`] for the cause.
    Empty { cause: EmptyLayerCause<MAC, RAC> },
}

/// The cause of a layer being empty.
#[derive(Copy, Clone, Debug, Eq, PartialEq)]
pub enum EmptyLayerCause<MAC, RAC> {
    /// The layer wasn't cached in a previous buildpack run and was newly created.
    NewlyCreated,
    /// The layer was cached in a previous buildpack run, but the metadata was invalid and couldn't
    /// be converted into a valid form. Subsequently, the layer was deleted entirely.
    ///
    /// See: `invalid_metadata_action` in [`CachedLayerDefinition`].
    InvalidMetadataAction { cause: MAC },
    /// The layer was cached in a previous buildpack run, but the `restored_layer_action` function
    /// rejected the contents and/or metadata.
    ///
    /// See: `restored_layer_action` in [`CachedLayerDefinition`].
    RestoredLayerAction { cause: RAC },
}

/// A value-to-value conversion for layer actions.
///
/// Similar to [`Into`], but specialized. Allowing it to also be implemented for
/// values in the standard library such as [`Result`].
///
/// Implement this trait if you want to use your own types as actions.
///
/// libcnb ships with generic implementations for the majority of the use-cases:
/// - Using [`RestoredLayerAction`] or [`InvalidMetadataAction`] directly.
/// - Using [`RestoredLayerAction`] or [`InvalidMetadataAction`] directly, wrapped in a Result.
/// - Using [`RestoredLayerAction`] or [`InvalidMetadataAction`] with a cause value in a tuple.
/// - Using [`RestoredLayerAction`] or [`InvalidMetadataAction`] with a cause value in a tuple, wrapped in a Result.
pub trait IntoAction<T, C, E> {
    fn into_action(self) -> Result<(T, C), E>;
}

// Allows to use the layer actions directly.
impl<T, E> IntoAction<T, (), E> for T {
    fn into_action(self) -> Result<(T, ()), E> {
        Ok((self, ()))
    }
}

//  Allows to use the layer actions directly wrapped in a Result.
impl<T, E> IntoAction<T, (), E> for Result<T, E> {
    fn into_action(self) -> Result<(T, ()), E> {
        self.map(|value| (value, ()))
    }
}

// Allows to use the layer actions directly with a cause as a tuple.
impl<T, C, E> IntoAction<T, C, E> for (T, C) {
    fn into_action(self) -> Result<(T, C), E> {
        Ok(self)
    }
}

// Allows to use the layer actions directly with a cause as a tuple wrapped in a Result.
impl<T, C, E> IntoAction<T, C, E> for Result<(T, C), E> {
    fn into_action(self) -> Result<(T, C), E> {
        self
    }
}

/// A reference to an existing layer on disk.
///
/// Provides functions to modify the layer such as replacing its metadata, environment, SBOMs or
/// exec.d programs.
///
/// To obtain a such a reference, use [`BuildContext::cached_layer`] or [`BuildContext::uncached_layer`].
pub struct LayerRef<B, MAC, RAC>
where
    B: Buildpack + ?Sized,
{
    name: LayerName,
    // Technically not part of the layer itself. However, the functions that modify the layer
    // will need a reference to the layers directory as they will also modify files outside the
    // actual layer directory. To make LayerRef nice to use, we bite the bullet and include
    // the layers_dir here.
    layers_dir: PathBuf,
    buildpack: PhantomData<B>,
    pub state: LayerState<MAC, RAC>,
}

impl<B, MAC, RAC> LayerRef<B, MAC, RAC>
where
    B: Buildpack,
{
    /// Returns the path to the layer on disk.
    pub fn path(&self) -> PathBuf {
        self.layers_dir.join(self.name.as_str())
    }

    /// Writes the given layer metadata to disk.
    ///
    /// Any existing layer metadata will be overwritten. The new value does not have to be of the
    /// same type as the existing metadata.
    pub fn write_metadata<M>(&self, metadata: M) -> crate::Result<(), B::Error>
    where
        M: Serialize,
    {
        crate::layer::shared::replace_layer_metadata(&self.layers_dir, &self.name, metadata)
            .map_err(|error| {
                crate::Error::LayerError(LayerError::WriteLayerError(
                    WriteLayerError::WriteLayerMetadataError(error),
                ))
            })
    }

    /// Writes the given layer environment to disk.
    ///
    /// Any existing layer environment will be overwritten.
    pub fn write_env(&self, env: impl Borrow<LayerEnv>) -> crate::Result<(), B::Error> {
        env.borrow()
            .write_to_layer_dir(self.path())
            .map_err(|error| {
                crate::Error::LayerError(LayerError::WriteLayerError(WriteLayerError::IoError(
                    error,
                )))
            })
    }

    /// Reads the current layer environment from disk.
    ///
    /// Note that this includes implicit entries such as adding `bin/` to `PATH`. See [`LayerEnv`]
    /// docs for details on implicit entries.
    pub fn read_env(&self) -> crate::Result<LayerEnv, B::Error> {
        LayerEnv::read_from_layer_dir(self.path()).map_err(|error| {
            crate::Error::LayerError(LayerError::ReadLayerError(ReadLayerError::IoError(error)))
        })
    }

    /// Writes the given SBOMs to disk.
    ///
    /// Any existing SBOMs will be overwritten.
    pub fn write_sboms(&self, sboms: &[Sbom]) -> crate::Result<(), B::Error> {
        replace_layer_sboms(&self.layers_dir, &self.name, sboms).map_err(|error| {
            crate::Error::LayerError(LayerError::WriteLayerError(
                WriteLayerError::ReplaceLayerSbomsError(error),
            ))
        })
    }

    /// Writes the given exec.d programs to disk.
    ///
    /// Any existing exec.d programs will be overwritten.
    pub fn write_exec_d_programs<P, S>(&self, programs: P) -> crate::Result<(), B::Error>
    where
        S: Into<String>,
        P: IntoIterator<Item = (S, PathBuf)>,
    {
        let programs = programs
            .into_iter()
            .map(|(k, v)| (k.into(), v))
            .collect::<HashMap<_, _>>();

        replace_layer_exec_d_programs(&self.layers_dir, &self.name, &programs).map_err(|error| {
            crate::Error::LayerError(LayerError::WriteLayerError(
                WriteLayerError::ReplaceLayerExecdProgramsError(error),
            ))
        })
    }
}
