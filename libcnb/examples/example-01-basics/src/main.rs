use libcnb::data::build_plan::BuildPlan;
use libcnb::{
    cnb_runtime, BuildContext, BuildOutcome, Buildpack, DetectContext, DetectOutcome, GenericError,
    GenericMetadata, GenericPlatform,
};

struct BasicBuildpack;
impl Buildpack for BasicBuildpack {
    type Platform = GenericPlatform;
    type Metadata = GenericMetadata;
    type Error = GenericError;

    fn detect(&self, _context: DetectContext<Self>) -> libcnb::Result<DetectOutcome, Self::Error> {
        Ok(DetectOutcome::Pass(BuildPlan::new()))
    }

    fn build(&self, context: BuildContext<Self>) -> libcnb::Result<BuildOutcome, Self::Error> {
        println!("Build runs on stack {}!", context.stack_id);
        Ok(BuildOutcome::success())
    }
}

fn main() {
    cnb_runtime(BasicBuildpack);
}
