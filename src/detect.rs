use std::fmt::Debug;
use std::path::PathBuf;

use crate::{data::build_plan::BuildPlan, data::buildpack::BuildpackToml, platform::Platform};

pub struct DetectContext<P: Platform, BM> {
    pub app_dir: PathBuf,
    pub buildpack_dir: PathBuf,
    pub stack_id: String,
    pub platform: P,
    pub buildpack_descriptor: BuildpackToml<BM>,
}

#[derive(Debug)]
pub enum DetectResult {
    Pass(BuildPlan),
    Fail,
    Error(i32),
}
