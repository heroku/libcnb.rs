pub(crate) mod handling;

// BuildContext is only used in RustDoc (https://github.com/rust-lang/rust/issues/79542)
#[allow(unused)]
use crate::build::BuildContext;
use crate::layer::shared::{replace_layer_exec_d_programs, replace_layer_sboms, WriteLayerError};
use crate::layer::LayerError;
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

/// The action to take after inspecting existing layer data.
#[derive(Copy, Clone, Debug)]
pub enum InspectExistingAction {
    /// Delete the existing layer.
    DeleteLayer,
    /// Keep the layer as-is.
    KeepLayer,
}

/// Framework metadata about the layer contents.
///
/// See: [`BuildContext::cached_layer`] and [`BuildContext::uncached_layer`]
#[derive(Copy, Clone, Debug)]
pub enum LayerContents<MAC, EAC> {
    /// The layer contains validated cached contents from a previous buildpack run.
    ///
    /// See: `inspect_existing` in [`CachedLayerDefinition`].
    Cached { cause: EAC },
    /// The layer is empty. Inspect the contained [`EmptyLayerCause`] for the cause.
    Empty { cause: EmptyLayerCause<MAC, EAC> },
}

/// The cause of a layer being empty.
#[derive(Copy, Clone, Debug)]
pub enum EmptyLayerCause<MAC, EAC> {
    /// The layer wasn't cached in a previous buildpack run and was freshly created.
    Uncached,
    /// The layer was cached in a previous buildpack run, but the metadata was invalid and couldn't
    /// be converted into a valid form. Subsequently, the layer was deleted entirely.
    ///
    /// See: `invalid_metadata` in [`CachedLayerDefinition`].
    MetadataInvalid { cause: MAC },
    /// The layer was cached in a previous buildpack run, but the `inspect_existing` function
    /// rejected the contents and/or metadata.
    ///
    /// See: `inspect_existing` in [`CachedLayerDefinition`].
    Inspect { cause: EAC },
}

/// A value-to-value conversion for layer actions.
///
/// Similar to [`Into`], but specialized. Allowing it to also be implemented for
/// values in the standard library such as [`Result`].
///
/// Implement this trait if you want to use your own types as actions.
///
/// libcnb ships with generic implementations for the majority of the use-cases:
/// - Using [`InspectExistingAction`] or [`InvalidMetadataAction`] directly.
/// - Using [`InspectExistingAction`] or [`InvalidMetadataAction`] directly, wrapped in a Result.
/// - Using [`InspectExistingAction`] or [`InvalidMetadataAction`] with a cause value in a tuple.
/// - Using [`InspectExistingAction`] or [`InvalidMetadataAction`] with a cause value in a tuple, wrapped in a Result.
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
pub struct LayerRef<B, MAC, EAC>
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
    pub contents: LayerContents<MAC, EAC>,
}

impl<B, MAC, EAC> LayerRef<B, MAC, EAC>
where
    B: Buildpack,
{
    /// Returns the path to the layer on disk.
    pub fn path(&self) -> PathBuf {
        self.layers_dir.join(self.name.as_str())
    }

    /// Replaces the existing layer metadata with a new value.
    ///
    /// The new value does not have to be of the same type as the existing metadata.
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

    /// Replaces the existing layer environment with a new one.
    pub fn replace_env(&self, env: impl Borrow<LayerEnv>) -> crate::Result<(), B::Error> {
        env.borrow()
            .write_to_layer_dir(self.path())
            .map_err(|error| {
                crate::Error::LayerError(LayerError::WriteLayerError(WriteLayerError::IoError(
                    error,
                )))
            })
    }

    /// Replace all existing layer SBOMs with new ones.
    pub fn replace_sboms(&self, sboms: &[Sbom]) -> crate::Result<(), B::Error> {
        replace_layer_sboms(&self.layers_dir, &self.name, sboms).map_err(|error| {
            crate::Error::LayerError(LayerError::WriteLayerError(
                WriteLayerError::ReplaceLayerSbomsError(error),
            ))
        })
    }

    /// Replace all existing layer exec.d programs with new ones.
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
