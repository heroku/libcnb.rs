use libcnb::build::{BuildContext, BuildResult, BuildResultBuilder};
use libcnb::data::layer_name;
use libcnb::detect::{DetectContext, DetectResult, DetectResultBuilder};
use libcnb::generic::{GenericMetadata, GenericPlatform};
use libcnb::layer::{CachedLayerDefinition, InspectExistingAction, InvalidMetadataAction};
use libcnb::{buildpack_main, Buildpack};
use std::fs;
use std::fs::Permissions;
use std::os::unix::fs::PermissionsExt;

// Suppress warnings due to the `unused_crate_dependencies` lint not handling integration tests well.
#[cfg(test)]
use libcnb_test as _;

pub(crate) struct TestBuildpack;

impl Buildpack for TestBuildpack {
    type Platform = GenericPlatform;
    type Metadata = GenericMetadata;
    type Error = TestBuildpackError;

    fn detect(&self, _context: DetectContext<Self>) -> libcnb::Result<DetectResult, Self::Error> {
        DetectResultBuilder::pass().build()
    }

    fn build(&self, context: BuildContext<Self>) -> libcnb::Result<BuildResult, Self::Error> {
        let layer = context.cached_layer(
            layer_name!("test"),
            CachedLayerDefinition {
                build: true,
                launch: true,
                invalid_metadata: &|_| InvalidMetadataAction::DeleteLayer,
                inspect_existing: &|_: &GenericMetadata, _| InspectExistingAction::Delete,
            },
        )?;

        let directory = layer.path().join("sub_directory");
        fs::create_dir_all(&directory).map_err(TestBuildpackError::IOError)?;

        fs::write(directory.join("foo.txt"), "hello world!")
            .map_err(TestBuildpackError::IOError)?;

        // By making the sub-directory read-only, files inside it cannot be deleted. This would
        // cause issues when libcnb.rs tries to delete a cached layer directory unless libcnb.rs
        // handles this case explicitly.
        fs::set_permissions(&directory, Permissions::from_mode(0o555))
            .map_err(TestBuildpackError::IOError)?;

        BuildResultBuilder::new().build()
    }
}

#[derive(Debug)]
pub(crate) enum TestBuildpackError {
    IOError(std::io::Error),
}

impl From<TestBuildpackError> for libcnb::Error<TestBuildpackError> {
    fn from(error: TestBuildpackError) -> Self {
        libcnb::Error::BuildpackError(error)
    }
}

buildpack_main!(TestBuildpack);
