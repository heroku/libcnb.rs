use libcnb;
use libcnb::detect::{DetectResult, DetectContext};
use libcnb::data::build_plan::BuildPlan;
use libcnb::shared;
use libcnb::shared::GenericPlatform;
use libcnb::shared::Platform;

fn main() {
    libcnb::detect::cnb_runtime_detect(detect)
}

fn detect(_context: DetectContext<GenericPlatform>) -> DetectResult {
    let mut buildplan = BuildPlan::new();

    DetectResult::Pass(buildplan)
}
