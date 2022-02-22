use crate::ExecDBuildpack;
use libcnb::build::BuildContext;
use libcnb::data::layer_content_metadata::LayerTypes;
use libcnb::generic::GenericMetadata;
use libcnb::layer::{Layer, LayerResult, LayerResultBuilder};
use libcnb::{additional_buildpack_binary_path, Buildpack};
use std::path::Path;

pub struct ExecDLayer;

impl Layer for ExecDLayer {
    type Buildpack = ExecDBuildpack;
    type Metadata = GenericMetadata;

    fn types(&self) -> LayerTypes {
        LayerTypes {
            launch: true,
            build: false,
            cache: false,
        }
    }

    fn create(
        &self,
        _context: &BuildContext<Self::Buildpack>,
        _layer_path: &Path,
    ) -> Result<LayerResult<Self::Metadata>, <Self::Buildpack as Buildpack>::Error> {
        LayerResultBuilder::new(GenericMetadata::default())
            .exec_d_program(
                "dice_roller",
                additional_buildpack_binary_path!("dice_roller"),
            )
            .build()
    }
}
