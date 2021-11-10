use libcnb::build::{BuildContext, BuildResult, BuildResultBuilder};
use libcnb::detect::{DetectContext, DetectResult, DetectResultBuilder};
use libcnb::{buildpack_main, Buildpack, GenericError, GenericMetadata, GenericPlatform};

struct BasicBuildpack;
impl Buildpack for BasicBuildpack {
    type Platform = GenericPlatform;
    type Metadata = GenericMetadata;
    type Error = GenericError;

    fn detect(&self, _context: DetectContext<Self>) -> libcnb::Result<DetectResult, Self::Error> {
        Ok(DetectResultBuilder::pass().build())
    }

    fn build(&self, context: BuildContext<Self>) -> libcnb::Result<BuildResult, Self::Error> {
        println!("Build runs on stack {}!", context.stack_id);
        Ok(BuildResultBuilder::new().build())
    }
}

buildpack_main!(BasicBuildpack);
