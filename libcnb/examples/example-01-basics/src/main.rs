use libcnb::build::{BuildContext, BuildOutcome, BuildOutcomeBuilder};
use libcnb::detect::{DetectContext, DetectOutcome, DetectOutcomeBuilder};
use libcnb::{cnb_runtime, Buildpack, GenericError, GenericMetadata, GenericPlatform};

struct BasicBuildpack;
impl Buildpack for BasicBuildpack {
    type Platform = GenericPlatform;
    type Metadata = GenericMetadata;
    type Error = GenericError;

    fn detect(&self, _context: DetectContext<Self>) -> libcnb::Result<DetectOutcome, Self::Error> {
        Ok(DetectOutcomeBuilder::pass().build())
    }

    fn build(&self, context: BuildContext<Self>) -> libcnb::Result<BuildOutcome, Self::Error> {
        println!("Build runs on stack {}!", context.stack_id);
        Ok(BuildOutcomeBuilder::new().build())
    }
}

fn main() {
    cnb_runtime(BasicBuildpack);
}
