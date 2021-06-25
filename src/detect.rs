use std::fmt::Debug;
use std::path::PathBuf;

use crate::{data::build_plan::BuildPlan, data::buildpack::BuildpackToml, platform::Platform};

/// Context for a buildpack's detect phase execution.
pub struct DetectContext<P: Platform, BM> {
    pub app_dir: PathBuf,
    pub buildpack_dir: PathBuf,
    pub stack_id: String,
    pub platform: P,
    pub buildpack_descriptor: BuildpackToml<BM>,
}

/// Describes the outcome of the buildpack's detect phase.
#[derive(Debug)]
pub enum DetectOutcome {
    Pass(BuildPlan),
    Fail,
}
