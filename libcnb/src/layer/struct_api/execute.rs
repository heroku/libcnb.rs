use crate::layer::handling::{ExecDPrograms, Sboms};
use crate::layer::{
    DeleteLayerError, EmptyReason, InspectExistingAction, IntoAction, InvalidMetadataAction,
    LayerContents, LayerRef, ReadLayerError, WriteLayerError,
};
use crate::layer_env::LayerEnv;
use crate::Buildpack;
use libcnb_common::toml_file::{read_toml_file, TomlFileError};
use libcnb_data::generic::GenericMetadata;
use libcnb_data::layer::LayerName;
use libcnb_data::layer_content_metadata::{LayerContentMetadata, LayerTypes};
use serde::de::DeserializeOwned;
use serde::Serialize;
use std::marker::PhantomData;
use std::path::{Path, PathBuf};

#[derive(thiserror::Error, Debug)]
#[allow(clippy::enum_variant_names)]
pub enum ExecuteLayerDefinitionError {
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
}

#[allow(clippy::too_many_lines)]
pub(crate) fn execute<B, M, MA, IA, MC, IC>(
    layer_types: LayerTypes,
    invalid_metadata: &dyn Fn(&GenericMetadata) -> MA,
    inspect_existing: &dyn Fn(&M, &Path) -> IA,
    layer_name: LayerName,
    layers_dir: &Path,
) -> crate::Result<LayerRef<B, MC, IC>, B::Error>
where
    B: Buildpack + ?Sized,
    M: Serialize + DeserializeOwned,
    MA: IntoAction<InvalidMetadataAction<M>, MC, B::Error>,
    IA: IntoAction<InspectExistingAction, IC, B::Error>,
{
    match crate::layer::handling::read_layer::<M, _>(layers_dir, &layer_name) {
        Ok(None) => create_layer(layer_types, &layer_name, layers_dir, EmptyReason::Uncached),
        Ok(Some(layer_data)) => {
            let inspect_action =
                inspect_existing(&layer_data.content_metadata.metadata, &layer_data.path)
                    .into_action()
                    .map_err(crate::Error::BuildpackError)?;

            match inspect_action {
                (InspectExistingAction::Delete, cause) => {
                    crate::layer::handling::delete_layer(layers_dir, &layer_name)
                        .map_err(ExecuteLayerDefinitionError::DeleteLayerError)?;

                    create_layer(
                        layer_types,
                        &layer_name,
                        layers_dir,
                        EmptyReason::Inspect(cause),
                    )
                }
                (InspectExistingAction::Keep, cause) => {
                    // Always write the layer types as:
                    // a) they might be different from what is currently on disk
                    // b) the cache field will be removed by CNB lifecycle on cache restore
                    crate::layer::replace_layer_types(layers_dir, &layer_name, layer_types)
                        .map_err(|error| {
                            ExecuteLayerDefinitionError::WriteLayerError(
                                WriteLayerError::WriteLayerMetadataError(error),
                            )
                        })?;

                    Ok(LayerRef {
                        name: layer_data.name,
                        layers_dir: PathBuf::from(layers_dir),
                        buildpack: PhantomData,
                        contents: LayerContents::Cached(cause),
                    })
                }
            }
        }
        Err(ReadLayerError::LayerContentMetadataParseError(_)) => {
            let layer_content_metadata = read_toml_file::<LayerContentMetadata>(
                layers_dir.join(format!("{}.toml", &layer_name)),
            )
            .map_err(ExecuteLayerDefinitionError::CouldNotReadGenericLayerMetadata)?;

            let invalid_metadata_action = invalid_metadata(&layer_content_metadata.metadata)
                .into_action()
                .map_err(crate::Error::BuildpackError)?;

            match invalid_metadata_action {
                (InvalidMetadataAction::DeleteLayer, cause) => {
                    crate::layer::handling::delete_layer(layers_dir, &layer_name)
                        .map_err(ExecuteLayerDefinitionError::DeleteLayerError)?;

                    create_layer(
                        layer_types,
                        &layer_name,
                        layers_dir,
                        EmptyReason::MetadataInvalid(cause),
                    )
                }
                (InvalidMetadataAction::ReplaceMetadata(metadata), _) => {
                    crate::layer::replace_layer_metadata(layers_dir, &layer_name, metadata)
                        .map_err(|error| {
                            ExecuteLayerDefinitionError::WriteLayerError(
                                WriteLayerError::WriteLayerMetadataError(error),
                            )
                        })?;

                    execute(
                        layer_types,
                        invalid_metadata,
                        inspect_existing,
                        layer_name,
                        layers_dir,
                    )
                }
            }
        }
        Err(read_layer_error) => Err(ExecuteLayerDefinitionError::ReadLayerError(
            read_layer_error,
        ))?,
    }
}

fn create_layer<X, Y, B>(
    layer_types: LayerTypes,
    layer_name: &LayerName,
    layers_dir: &Path,
    empty_reason: EmptyReason<X, Y>,
) -> Result<LayerRef<B, X, Y>, crate::Error<B::Error>>
where
    B: Buildpack + ?Sized,
{
    crate::layer::handling::write_layer(
        layers_dir,
        layer_name,
        &LayerEnv::new(),
        &LayerContentMetadata {
            types: Some(layer_types),
            metadata: GenericMetadata::default(),
        },
        ExecDPrograms::Keep,
        Sboms::Keep,
    )
    .map_err(ExecuteLayerDefinitionError::WriteLayerError)?;

    let layer_data =
        crate::layer::handling::read_layer::<GenericMetadata, _>(layers_dir, layer_name)
            .map_err(ExecuteLayerDefinitionError::ReadLayerError)?
            .ok_or(ExecuteLayerDefinitionError::CouldNotReadLayerAfterCreate(
                layer_name.clone(),
            ))?;

    Ok(LayerRef {
        name: layer_data.name,
        layers_dir: PathBuf::from(layers_dir),
        buildpack: PhantomData,
        contents: LayerContents::Empty(empty_reason),
    })
}
