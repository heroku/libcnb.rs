use crate::TestBuildpack;
use libcnb::build::BuildContext;
use libcnb::data::layer_content_metadata::LayerTypes;
use libcnb::generic::GenericMetadata;
use libcnb::layer::{ExistingLayerStrategy, Layer, LayerData, LayerResult, LayerResultBuilder};
use libcnb::Buildpack;
use std::fs;
use std::fs::Permissions;
use std::os::unix::fs::PermissionsExt;
use std::path::Path;

pub struct TestLayer;

impl Layer for TestLayer {
    type Buildpack = TestBuildpack;
    type Metadata = GenericMetadata;

    fn types(&self) -> LayerTypes {
        LayerTypes {
            launch: true,
            build: true,
            cache: true,
        }
    }

    fn create(
        &self,
        _context: &BuildContext<Self::Buildpack>,
        layer_path: &Path,
    ) -> Result<LayerResult<Self::Metadata>, <Self::Buildpack as Buildpack>::Error> {
        let directory = layer_path.join("sub_directory");
        fs::create_dir_all(&directory)?;

        fs::write(directory.join("foo.txt"), "hello world!")?;

        // By making the sub-directory read-only, files inside it cannot be deleted. This would
        // cause issues when libcnb.rs tries to delete a cached layer directory unless libcnb.rs
        // handles this case explicitly.
        fs::set_permissions(&directory, Permissions::from_mode(0o555))?;

        LayerResultBuilder::new(GenericMetadata::default()).build()
    }

    fn existing_layer_strategy(
        &self,
        _context: &BuildContext<Self::Buildpack>,
        _layer_data: &LayerData<Self::Metadata>,
    ) -> Result<ExistingLayerStrategy, <Self::Buildpack as Buildpack>::Error> {
        // Even though this is (currently) the default, we explicitly declare it here to make sure
        // the layer will be recreated, even if the default in libcnb changes.
        Ok(ExistingLayerStrategy::Recreate)
    }
}
