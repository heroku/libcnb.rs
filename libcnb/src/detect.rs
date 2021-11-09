use std::fmt::Debug;
use std::path::PathBuf;

use crate::buildpack::Buildpack;
use crate::{data::build_plan::BuildPlan, data::buildpack::BuildpackToml};

/// Context for a buildpack's detect phase execution.
pub struct DetectContext<B: Buildpack + ?Sized> {
    pub app_dir: PathBuf,
    pub buildpack_dir: PathBuf,
    pub stack_id: String,
    pub platform: B::Platform,
    pub buildpack_descriptor: BuildpackToml<B::Metadata>,
}

/// Describes the outcome of the buildpack's detect phase.
#[derive(Debug)]
pub enum DetectOutcome {
    Pass(BuildPlan),
    Fail,
}
