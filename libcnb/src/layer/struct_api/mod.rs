pub(crate) mod handling;

use crate::layer::shared::{replace_layer_exec_d_programs, replace_layer_sboms, WriteLayerError};
use crate::layer::LayerError;
use crate::layer_env::LayerEnv;
use crate::sbom::Sbom;
use crate::Buildpack;
use libcnb_data::generic::GenericMetadata;
use libcnb_data::layer::LayerName;
use serde::Serialize;
use std::collections::HashMap;
use std::marker::PhantomData;
use std::path::{Path, PathBuf};

/// A definition for a cached layer.
pub struct CachedLayerDefinition<'a, M, MA, EA> {
    /// Whether the layer is intended for build.
    pub build: bool,
    /// Whether the layer is intended for launch.
    pub launch: bool,
    /// Callback for when the metadata of an existing layer cannot be parsed as `M`.
    ///
    /// Allows replacing the metadata before continuing (i.e. migration to a newer version) or
    /// deleting the layer.
    pub invalid_metadata: &'a dyn Fn(&GenericMetadata) -> MA,
    /// Callback when the layer already exists to validate the contents and metadata. Can be used
    /// to delete existing cached layers.
    pub inspect_existing: &'a dyn Fn(&M, &Path) -> EA,
}

/// A definition for an uncached layer.
pub struct UncachedLayerDefinition {
    /// Whether the layer is intended for build.
    pub build: bool,
    /// Whether the layer is intended for launch.
    pub launch: bool,
}

/// The action to take when the layer metadata is invalid.
#[derive(Copy, Clone)]
pub enum InvalidMetadataAction<M> {
    DeleteLayer,
    ReplaceMetadata(M),
}

/// The action to take after inspecting existing layer data.
#[derive(Copy, Clone)]
pub enum InspectExistingAction {
    Delete,
    Keep,
}

pub enum LayerContents<X, Y> {
    /// The layer contains validated cached contents from a previous buildpack run.
    ///
    /// See: `inspect_existing` in [`CachedLayerDefinition`].
    Cached(Y),
    /// The layer is empty. Inspect the contained [`EmptyReason`] for details why.
    Empty(EmptyReason<X, Y>),
}

pub enum EmptyReason<X, Y> {
    /// The layer wasn't cached in a previous buildpack run.
    Uncached,
    /// The layer was cached in a previous buildpack run, but the metadata was invalid and couldn't
    /// be converted into a valid form. Subsequently, the layer was deleted entirely.
    ///
    /// See: `invalid_metadata` in [`CachedLayerDefinition`].
    MetadataInvalid(X),
    /// The layer was cached in a previous buildpack run, but the `inspect_existing` function
    /// rejected the contents.
    ///
    /// See: `inspect_existing` in [`CachedLayerDefinition`].
    Inspect(Y),
}

/// A value-to-value conversion for layer actions.
///
/// Similar to [`Into`], but specialized. Allowing it to also be implemented for
/// values in the standard library such as [`Result`].
pub trait IntoAction<T, C, E> {
    fn into_action(self) -> Result<(T, C), E>;
}

impl<T, E> IntoAction<T, (), E> for T {
    fn into_action(self) -> Result<(T, ()), E> {
        Ok((self, ()))
    }
}

impl<T, C, E> IntoAction<T, C, E> for (T, C) {
    fn into_action(self) -> Result<(T, C), E> {
        Ok(self)
    }
}

impl<T, C, E> IntoAction<T, C, E> for Result<(T, C), E> {
    fn into_action(self) -> Result<(T, C), E> {
        self
    }
}

impl<T, E> IntoAction<T, (), E> for Result<T, E> {
    fn into_action(self) -> Result<(T, ()), E> {
        self.map(|value| (value, ()))
    }
}

pub struct LayerRef<B, X, Y>
where
    B: Buildpack + ?Sized,
{
    name: LayerName,
    layers_dir: PathBuf,
    buildpack: PhantomData<B>,
    pub contents: LayerContents<X, Y>,
}

impl<B, X, Y> LayerRef<B, X, Y>
where
    B: Buildpack,
{
    pub fn path(&self) -> PathBuf {
        self.layers_dir.join(self.name.as_str())
    }

    pub fn replace_metadata<M>(&self, metadata: M) -> crate::Result<(), B::Error>
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

    pub fn replace_env(&self, env: &LayerEnv) -> crate::Result<(), B::Error> {
        env.write_to_layer_dir(self.path()).map_err(|error| {
            crate::Error::LayerError(LayerError::WriteLayerError(WriteLayerError::IoError(error)))
        })
    }

    pub fn replace_sboms(&self, sboms: &[Sbom]) -> crate::Result<(), B::Error> {
        replace_layer_sboms(&self.layers_dir, &self.name, sboms).map_err(|error| {
            crate::Error::LayerError(LayerError::WriteLayerError(
                WriteLayerError::ReplaceLayerSbomsError(error),
            ))
        })
    }

    pub fn replace_exec_d_programs<P, S>(&self, programs: P) -> crate::Result<(), B::Error>
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
