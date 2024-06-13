use crate::layer::shared::{
    delete_layer, read_layer, replace_layer_metadata, replace_layer_types, ReadLayerError,
    WriteLayerError,
};
use crate::layer::{
    EmptyLayerCause, InspectExistingAction, IntoAction, InvalidMetadataAction, LayerContents,
    LayerError, LayerRef,
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

pub(crate) fn handle_layer<B, M, MA, EA, MAC, EAC>(
    layer_types: LayerTypes,
    invalid_metadata: &dyn Fn(&GenericMetadata) -> MA,
    inspect_existing: &dyn Fn(&M, &Path) -> EA,
    layer_name: LayerName,
    layers_dir: &Path,
) -> crate::Result<LayerRef<B, MAC, EAC>, B::Error>
where
    B: Buildpack + ?Sized,
    M: Serialize + DeserializeOwned,
    MA: IntoAction<InvalidMetadataAction<M>, MAC, B::Error>,
    EA: IntoAction<InspectExistingAction, EAC, B::Error>,
{
    match read_layer::<M, _>(layers_dir, &layer_name) {
        Ok(None) => create_layer(
            layer_types,
            &layer_name,
            layers_dir,
            EmptyLayerCause::Uncached,
        ),
        Ok(Some(layer_data)) => {
            let inspect_action = inspect_existing(&layer_data.metadata.metadata, &layer_data.path)
                .into_action()
                .map_err(crate::Error::BuildpackError)?;

            match inspect_action {
                (InspectExistingAction::DeleteLayer, cause) => {
                    delete_layer(layers_dir, &layer_name).map_err(LayerError::DeleteLayerError)?;

                    create_layer(
                        layer_types,
                        &layer_name,
                        layers_dir,
                        EmptyLayerCause::Inspect { cause },
                    )
                }
                (InspectExistingAction::KeepLayer, cause) => {
                    // Always write the layer types as:
                    // a) they might be different from what is currently on disk
                    // b) the cache field will be removed by CNB lifecycle on cache restore
                    replace_layer_types(layers_dir, &layer_name, layer_types).map_err(|error| {
                        LayerError::WriteLayerError(WriteLayerError::WriteLayerMetadataError(error))
                    })?;

                    Ok(LayerRef {
                        name: layer_data.name,
                        layers_dir: PathBuf::from(layers_dir),
                        buildpack: PhantomData,
                        contents: LayerContents::Cached { cause },
                    })
                }
            }
        }
        Err(ReadLayerError::LayerContentMetadataParseError(_)) => {
            let layer_content_metadata = read_toml_file::<LayerContentMetadata>(
                layers_dir.join(format!("{}.toml", &layer_name)),
            )
            .map_err(LayerError::CouldNotReadGenericLayerMetadata)?;

            let invalid_metadata_action = invalid_metadata(&layer_content_metadata.metadata)
                .into_action()
                .map_err(crate::Error::BuildpackError)?;

            match invalid_metadata_action {
                (InvalidMetadataAction::DeleteLayer, cause) => {
                    delete_layer(layers_dir, &layer_name).map_err(LayerError::DeleteLayerError)?;

                    create_layer(
                        layer_types,
                        &layer_name,
                        layers_dir,
                        EmptyLayerCause::MetadataInvalid { cause },
                    )
                }
                (InvalidMetadataAction::ReplaceMetadata(metadata), _) => {
                    replace_layer_metadata(layers_dir, &layer_name, metadata).map_err(|error| {
                        LayerError::WriteLayerError(WriteLayerError::WriteLayerMetadataError(error))
                    })?;

                    handle_layer(
                        layer_types,
                        invalid_metadata,
                        inspect_existing,
                        layer_name,
                        layers_dir,
                    )
                }
            }
        }
        Err(read_layer_error) => Err(LayerError::ReadLayerError(read_layer_error))?,
    }
}

fn create_layer<B, MAC, EAC>(
    layer_types: LayerTypes,
    layer_name: &LayerName,
    layers_dir: &Path,
    empty_layer_cause: EmptyLayerCause<MAC, EAC>,
) -> Result<LayerRef<B, MAC, EAC>, crate::Error<B::Error>>
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
        contents: LayerContents::Empty {
            cause: empty_layer_cause,
        },
    })
}
