use libcnb::data::build_plan::BuildPlan;
use libcnb::{
    cnb_runtime, DetectOutcome, GenericBuildContext, GenericDetectContext, GenericErrorHandler,
    Result,
};

fn main() {
    cnb_runtime(detect, build, GenericErrorHandler);
}

fn detect(_context: GenericDetectContext) -> Result<DetectOutcome, std::io::Error> {
    let buildplan = BuildPlan::new();
    Ok(DetectOutcome::Pass(buildplan))
}

fn build(context: GenericBuildContext) -> Result<(), std::io::Error> {
    println!("Build runs on stack {}!", context.stack_id);
    Ok(())
}
