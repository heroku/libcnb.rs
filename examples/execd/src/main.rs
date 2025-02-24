use libcnb::build::{BuildContext, BuildResult, BuildResultBuilder};
use libcnb::data::layer_name;
use libcnb::detect::{DetectContext, DetectResult, DetectResultBuilder};
use libcnb::generic::{GenericError, GenericMetadata, GenericPlatform};
use libcnb::{Buildpack, additional_buildpack_binary_path, buildpack_main};

// Suppress warnings due to the `unused_crate_dependencies` lint not handling integration tests well.
use fastrand as _;
use libcnb::layer::UncachedLayerDefinition;
#[cfg(test)]
use libcnb_test as _;

pub(crate) struct ExecDBuildpack;

impl Buildpack for ExecDBuildpack {
    type Platform = GenericPlatform;
    type Metadata = GenericMetadata;
    type Error = GenericError;

    fn detect(&self, _context: DetectContext<Self>) -> libcnb::Result<DetectResult, Self::Error> {
        DetectResultBuilder::pass().build()
    }

    fn build(&self, context: BuildContext<Self>) -> libcnb::Result<BuildResult, Self::Error> {
        let layer_ref = context.uncached_layer(
            layer_name!("layer_name"),
            UncachedLayerDefinition {
                build: false,
                launch: true,
            },
        )?;

        layer_ref.write_exec_d_programs([(
            "dice_roller",
            additional_buildpack_binary_path!("dice_roller"),
        )])?;

        BuildResultBuilder::new().build()
    }
}

buildpack_main!(ExecDBuildpack);
