use libcnb;
use libcnb::data::build_plan::BuildPlan;
use libcnb::detect::DetectOutcome;
use libcnb::detect::GenericDetectContext;
use libcnb::platform::Platform;
use libcnb::shared;
use std::error::Error;

fn main() {
    libcnb::detect::cnb_runtime_detect(detect)
}

fn detect(_context: GenericDetectContext) -> Result<DetectOutcome, std::io::Error> {
    let mut buildplan = BuildPlan::new();

    Ok(DetectOutcome::Pass(buildplan))
}
