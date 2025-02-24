use libcnb::build::{BuildContext, BuildResult, BuildResultBuilder};
use libcnb::detect::{DetectContext, DetectResult, DetectResultBuilder};
use libcnb::generic::{GenericError, GenericMetadata, GenericPlatform};
use libcnb::{Buildpack, buildpack_main};

pub(crate) struct BasicBuildpack;

impl Buildpack for BasicBuildpack {
    type Platform = GenericPlatform;
    type Metadata = GenericMetadata;
    type Error = GenericError;

    fn detect(&self, _context: DetectContext<Self>) -> libcnb::Result<DetectResult, Self::Error> {
        DetectResultBuilder::pass().build()
    }

    fn build(&self, context: BuildContext<Self>) -> libcnb::Result<BuildResult, Self::Error> {
        println!(
            "The build is running on {} ({})!",
            context.target.os, context.target.arch
        );

        BuildResultBuilder::new().build()
    }
}

buildpack_main!(BasicBuildpack);
