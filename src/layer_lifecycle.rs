use std::error::Error;
use std::fmt::Debug;
use std::path::{Path, PathBuf};

use serde::de::DeserializeOwned;
use serde::Serialize;

use crate::build::BuildContext;
use crate::data::layer::LayerContentMetadata;

use crate::error::LibCnbError;
use crate::platform::Platform;
use crate::shared::TomlFileError;

/// A lifecycle of a Cloud Native Buildpack layer
///
/// See [`execute_layer_lifecycle`]
pub trait LayerLifecycle<P: Platform, BM, LM, O: Default, E: Error> {
    /// Creates the layer from scratch
    ///
    /// When used with [`execute_layer_lifecycle`], `path` will be created and empty. The
    /// returned [`LayerContentMetadata`] will be automatically written to disk. Implementations
    /// only need to care about putting files into `path`.
    fn create(
        &self,
        layer_path: &Path,
        build_context: &BuildContext<P, BM>,
    ) -> Result<LayerContentMetadata<LM>, E>;

    /// Tries to recover from invalid layer metadata
    ///
    /// When the metadata of the existing layer cannot be deserialized into `LM`, this function will
    /// be called by [`execute_layer_lifecycle`] with the actual layer metadata as TOML. This allows
    /// implementors to recover from this case by, for example, implementing migrations from older
    /// versions of the metadata to the current one.
    ///
    /// The default implementation returns [`InvalidMetadataRecoveryResult::DeleteLayer`] to signal
    /// that the existing layer should be deleted in its entirety.
    fn recover_from_invalid_metadata(
        &self,
        #[allow(unused_variables)] layer_metadata: &toml::value::Table,
        #[allow(unused_variables)] build_context: &BuildContext<P, BM>,
    ) -> Result<MetadataRecoveryStrategy<LM>, E> {
        // Default implementation is to delete the layer if the metadata is invalid
        Ok(MetadataRecoveryStrategy::DeleteLayer)
    }

    /// Based on the current state of the layer, determines how the layer will be processed
    ///
    /// This will be called by [`execute_layer_lifecycle`] when the layer for this lifecycle already
    /// exists. Based on the layer contents and its metadata, implementors can decide how the layer
    /// will be processed.
    ///
    /// A common case is the comparison of the layer metadata to some expected value to determine
    /// if the contents are still valid for the current build. If the contents are no longer valid,
    /// an implementation might return [`ValidateResult::RecreateLayer`] which will result in the
    /// layer being deleted and then recreated via [`LayerLifecycle::create`]. If the layer contents
    /// are still valid, [`ValidateResult::KeepLayer`] could be returned to keep the layer as-is
    /// without doing any modifications to it.
    ///
    /// The common case above is just one conceivable implementation of this function, take a look
    /// at [`ValidateResult`] for additional validation results and their meanings.
    fn validate(
        &self,
        #[allow(unused_variables)] layer_path: &Path,
        #[allow(unused_variables)] layer_content_metadata: &LayerContentMetadata<LM>,
        #[allow(unused_variables)] build_context: &BuildContext<P, BM>,
    ) -> ValidateResult {
        // Default implementation is to always recreate the layer
        ValidateResult::RecreateLayer
    }

    /// Updates an existing layer
    fn update(
        &self,
        #[allow(unused_variables)] layer_path: &Path,
        #[allow(unused_variables)] layer_content_metadata: LayerContentMetadata<LM>,
        #[allow(unused_variables)] build_context: &BuildContext<P, BM>,
    ) -> Result<LayerContentMetadata<LM>, E> {
        // Default implementation is a no-op
        Ok(layer_content_metadata)
    }

    fn layer_lifecycle_data(
        &self,
        #[allow(unused_variables)] layer_path: &Path,
        #[allow(unused_variables)] layer_content_metadata: LayerContentMetadata<LM>,
    ) -> Result<O, E> {
        Ok(O::default())
    }

    fn on_lifecycle_start(&self) {}
    fn on_keep(&self) {}
    fn on_update(&self) {}
    fn on_create(&self) {}
    fn on_lifecycle_end(&self) {}
}

/// The result of the recovery process for invalid layer metadata
///
/// See [`LayerLifecycle::invalid_metadata_recovery`]
pub enum MetadataRecoveryStrategy<M> {
    /// Delete the layer entirely
    DeleteLayer,
    /// Replace the metadata
    ReplaceMetadata(M),
}

/// The result of a layer validation
///
/// See [`LayerLifecycle::validate`]
pub enum ValidateResult {
    /// Keep the layer just as it is
    ///
    /// No [`LayerLifecycle`] functions will be called for this layer
    KeepLayer,

    /// Delete the layer and create a new one
    ///
    /// Only [`create`](LayerLifecycle::create) will be called
    RecreateLayer,

    /// Update the existing layer
    ///
    /// Only [`update`](LayerLifecycle::update) will be called
    UpdateLayer,
}

#[derive(thiserror::Error, Debug)]
pub enum LayerLifecycleError {
    #[error("Could not replace metadata after invalid_metadata_recovery: {0}")]
    MetadataReplaceFailed(TomlFileError),

    #[error("Could not read untyped metadata for invalid_metadata_recovery: {0}")]
    CouldNotReadUntypedLayerMetadata(TomlFileError),

    #[error("Could not write layer content metadata after creating new layer: {0}")]
    CouldNotWriteLayerContentMetadataAfterCreate(TomlFileError),

    #[error("Could create layer directory before layer lifecycle create: {0}")]
    CannotCreateLayerDirectoryBeforeCreate(std::io::Error),

    #[error("XXX {0}")]
    CannotDeleteLayerAfterValidate(std::io::Error),

    #[error("XXX {0}")]
    CannotDeleteLayerAfterMetadataRecovery(std::io::Error),

    #[error("XXX {0}")]
    CannotWriteLayerContentMetadataAfterUpdate(TomlFileError),

    #[error("XXX {0}")]
    CannotWriteLayerContentMetadataAfterCreateTriggredByRecreate(TomlFileError),

    #[error("XXX {0}")]
    CannotCreateLayerDirectoryBeforeCreateTriggredByRecreate(std::io::Error),

    #[error("XXX {0}")]
    CannotDeleteLayerBeforeCreateTriggredByRecreate(std::io::Error),

    #[error("XXX")]
    LayerContentMetadataMissingAfterLifecycle(),

    #[error("XXX {0}")]
    LayerContentMetadataIssue(TomlFileError),
}

pub fn execute_layer_lifecycle<
    P: Platform,
    BM,
    LM: Serialize + DeserializeOwned,
    O: Default,
    E: Error,
>(
    layer_name: impl AsRef<str>,
    layer_lifecycle: impl LayerLifecycle<P, BM, LM, O, E>,
    context: &BuildContext<P, BM>,
) -> Result<O, LibCnbError<E>> {
    layer_lifecycle.on_lifecycle_start();

    let layer_path = context.layer_path(&layer_name);
    let layer_content_metadata = match context.read_layer_content_metadata(&layer_name) {
        Ok(value) => value,
        Err(_) => {
            // If we cannot read the metadata due to a TOML file error, it's very likely that the
            // metadata could not be parsed into `LM` due to field/type mismatch(es). Regardless
            // of the actual error, we run the metadata recovery process here.
            metadata_recovery(&layer_name, &layer_lifecycle, &context)?
        }
    };

    match layer_content_metadata {
        Some(layer_content_metadata) => {
            let handler =
                match layer_lifecycle.validate(&layer_path, &layer_content_metadata, &context) {
                    ValidateResult::KeepLayer => handle_layer_keep,
                    ValidateResult::RecreateLayer => handle_layer_recreate,
                    ValidateResult::UpdateLayer => handle_layer_update,
                };

            handler(
                &layer_name,
                &layer_path,
                layer_content_metadata,
                &layer_lifecycle,
                &context,
            )?;
        }
        None => handle_layer_create(&layer_name, &layer_path, &layer_lifecycle, &context)?,
    };

    layer_lifecycle.on_lifecycle_end();

    match context.read_layer_content_metadata(&layer_name) {
        Err(toml_file_error) => Err(LibCnbError::LayerLifecycleError(
            LayerLifecycleError::LayerContentMetadataIssue(toml_file_error),
        )),
        Ok(None) => Err(LibCnbError::LayerLifecycleError(
            LayerLifecycleError::LayerContentMetadataMissingAfterLifecycle(),
        )),
        Ok(Some(metadata)) => layer_lifecycle
            .layer_lifecycle_data(&layer_path, metadata)
            .map_err(LibCnbError::BuildpackError),
    }
}

fn handle_layer_keep<P: Platform, BM, LM: Serialize + DeserializeOwned, O: Default, E: Error>(
    _layer_name: impl AsRef<str>,
    _layer_path: &PathBuf,
    _layer_content_metadata: LayerContentMetadata<LM>,
    layer_lifecycle: &impl LayerLifecycle<P, BM, LM, O, E>,
    _context: &BuildContext<P, BM>,
) -> Result<(), LibCnbError<E>> {
    layer_lifecycle.on_keep();
    Ok(())
}

fn handle_layer_create<P: Platform, BM, LM: Serialize + DeserializeOwned, O: Default, E: Error>(
    layer_name: impl AsRef<str>,
    layer_path: &PathBuf,
    layer_lifecycle: &impl LayerLifecycle<P, BM, LM, O, E>,
    context: &BuildContext<P, BM>,
) -> Result<(), LibCnbError<E>> {
    std::fs::create_dir_all(&layer_path)
        .map_err(LayerLifecycleError::CannotCreateLayerDirectoryBeforeCreate)?;

    layer_lifecycle.on_create();

    let layer_content_metadata = layer_lifecycle
        .create(&layer_path, &context)
        .map_err(LibCnbError::BuildpackError)?;

    context
        .write_layer_content_metadata(&layer_name, &layer_content_metadata)
        .map_err(LayerLifecycleError::CouldNotWriteLayerContentMetadataAfterCreate)?;
    Ok(())
}

fn handle_layer_recreate<
    P: Platform,
    BM,
    LM: Serialize + DeserializeOwned,
    O: Default,
    E: Error,
>(
    layer_name: impl AsRef<str>,
    layer_path: &PathBuf,
    _layer_content_metadata: LayerContentMetadata<LM>,
    layer_lifecycle: &impl LayerLifecycle<P, BM, LM, O, E>,
    context: &BuildContext<P, BM>,
) -> Result<(), LibCnbError<E>> {
    context
        .delete_layer(&layer_name)
        .map_err(LayerLifecycleError::CannotDeleteLayerBeforeCreateTriggredByRecreate)?;

    std::fs::create_dir_all(&layer_path)
        .map_err(LayerLifecycleError::CannotCreateLayerDirectoryBeforeCreateTriggredByRecreate)?;

    layer_lifecycle.on_create();

    let content_metadata = layer_lifecycle
        .create(&layer_path, &context)
        .map_err(LibCnbError::BuildpackError)?;

    context
        .write_layer_content_metadata(&layer_name, &content_metadata)
        .map_err(|toml_error| {
            LibCnbError::LayerLifecycleError(
                LayerLifecycleError::CannotWriteLayerContentMetadataAfterCreateTriggredByRecreate(
                    toml_error,
                ),
            )
        })
}

fn handle_layer_update<P: Platform, BM, LM: Serialize + DeserializeOwned, O: Default, E: Error>(
    layer_name: impl AsRef<str>,
    layer_path: &PathBuf,
    layer_content_metadata: LayerContentMetadata<LM>,
    layer_lifecycle: &impl LayerLifecycle<P, BM, LM, O, E>,
    context: &BuildContext<P, BM>,
) -> Result<(), LibCnbError<E>> {
    layer_lifecycle.on_update();

    let content_metadata = layer_lifecycle
        .update(&layer_path, layer_content_metadata, &context)
        .map_err(LibCnbError::BuildpackError)?;

    context
        .write_layer_content_metadata(&layer_name, &content_metadata)
        .map_err(|toml_error| {
            LibCnbError::LayerLifecycleError(
                LayerLifecycleError::CannotWriteLayerContentMetadataAfterUpdate(toml_error),
            )
        })
}

fn metadata_recovery<P: Platform, BM, LM: Serialize + DeserializeOwned, O: Default, E: Error>(
    layer_name: impl AsRef<str>,
    layer_lifecycle: &impl LayerLifecycle<P, BM, LM, O, E>,
    context: &BuildContext<P, BM>,
) -> Result<Option<LayerContentMetadata<LM>>, LibCnbError<E>> {
    // Read existing layer content metadata as TOML table, handling potential errors and
    // non-existent metadata so subsequent steps don't have to deal with either.
    let mut layer_content_metadata = {
        let maybe_layer_content_metadata = context
            .read_layer_content_metadata(&layer_name)
            .map_err(|toml_file_error| {
                LibCnbError::LayerLifecycleError(
                    LayerLifecycleError::CouldNotReadUntypedLayerMetadata(toml_file_error),
                )
            })?;

        match maybe_layer_content_metadata {
            None => return Ok(None),
            Some(value) => value,
        }
    };

    let metadata_recovery_strategy = layer_lifecycle
        .recover_from_invalid_metadata(&layer_content_metadata.metadata, &context)
        .map_err(LibCnbError::BuildpackError)?;

    match metadata_recovery_strategy {
        MetadataRecoveryStrategy::DeleteLayer => {
            context
                .delete_layer(&layer_name)
                .map_err(LayerLifecycleError::CannotDeleteLayerAfterMetadataRecovery)?;

            Ok(None)
        }
        MetadataRecoveryStrategy::ReplaceMetadata(replacement_metadata) => {
            let updated_metadata = layer_content_metadata.metadata(replacement_metadata);

            context
                .write_layer_content_metadata(&layer_name, &updated_metadata)
                .map_err(LayerLifecycleError::MetadataReplaceFailed)?;

            Ok(Some(updated_metadata))
        }
    }
}
