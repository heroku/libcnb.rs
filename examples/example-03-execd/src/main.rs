mod layer;

use crate::layer::ExampleLayer;
use libcnb::build::{BuildContext, BuildResult, BuildResultBuilder};
use libcnb::data::launch::{Launch, ProcessBuilder};
use libcnb::data::layer_name;
use libcnb::data::process_type;
use libcnb::detect::{DetectContext, DetectResult, DetectResultBuilder};
use libcnb::generic::{GenericError, GenericMetadata, GenericPlatform};
use libcnb::{buildpack_main, Buildpack};

pub struct ExampleBuildpack;

impl Buildpack for ExampleBuildpack {
    type Platform = GenericPlatform;
    type Metadata = GenericMetadata;
    type Error = GenericError;

    fn detect(&self, _context: DetectContext<Self>) -> libcnb::Result<DetectResult, Self::Error> {
        DetectResultBuilder::pass().build()
    }

    fn build(&self, context: BuildContext<Self>) -> libcnb::Result<BuildResult, Self::Error> {
        context.handle_layer(layer_name!("example"), ExampleLayer)?;

        BuildResultBuilder::new()
            .launch(
                Launch::new().process(
                    ProcessBuilder::new(process_type!("web"), "sleep 3600")
                        .default(true)
                        .build(),
                ),
            )
            .build()
    }
}

buildpack_main!(ExampleBuildpack);
