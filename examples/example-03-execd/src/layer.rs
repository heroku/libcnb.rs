use crate::ExampleBuildpack;
use libcnb::build::BuildContext;
use libcnb::data::layer_content_metadata::LayerTypes;
use libcnb::generic::GenericMetadata;
use libcnb::layer::{Layer, LayerResult, LayerResultBuilder};
use libcnb::{path_to_packaged_crate_binary, Buildpack};
use std::path::Path;

pub struct ExampleLayer;

impl Layer for ExampleLayer {
    type Buildpack = ExampleBuildpack;
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
            .execd("env_vars", path_to_packaged_crate_binary!("env_vars"))
            .build()
    }
}
