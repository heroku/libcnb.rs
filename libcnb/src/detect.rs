use std::fmt::Debug;
use std::path::PathBuf;

use crate::buildpack::Buildpack;
use crate::{data::build_plan::BuildPlan, data::buildpack::BuildpackToml};

/// Context for the detect phase execution.
pub struct DetectContext<B: Buildpack + ?Sized> {
    pub app_dir: PathBuf,
    pub buildpack_dir: PathBuf,
    pub stack_id: String,
    pub platform: B::Platform,
    pub buildpack_descriptor: BuildpackToml<B::Metadata>,
}

/// Describes the result of the detect phase.
///
/// Besides indicating passing or failing detection, it also contains detect phase output such as
/// the build plan. To construct values of this type, use a [`DetectResultBuilder`].
#[derive(Debug)]
pub struct DetectResult(pub(crate) InnerDetectResult);

#[derive(Debug)]
pub(crate) enum InnerDetectResult {
    Fail,
    Pass { build_plan: Option<BuildPlan> },
}

/// Constructs [`DetectResult`] values.
///
/// # Examples:
/// ```
/// use libcnb::detect::DetectResultBuilder;
/// use libcnb_data::build_plan::{BuildPlan, BuildPlanBuilder};
///
/// let simple_pass = DetectResultBuilder::pass().build();
/// let simple_fail = DetectResultBuilder::fail().build();
///
/// let with_build_plan = DetectResultBuilder::pass()
///    .build_plan(BuildPlanBuilder::new().provides("something").build())
///    .build();
/// ```
pub struct DetectResultBuilder;

impl DetectResultBuilder {
    pub fn pass() -> PassDetectResultBuilder {
        PassDetectResultBuilder { build_plan: None }
    }

    pub fn fail() -> FailDetectResultBuilder {
        FailDetectResultBuilder {}
    }
}

pub struct PassDetectResultBuilder {
    build_plan: Option<BuildPlan>,
}

impl PassDetectResultBuilder {
    pub fn build(self) -> DetectResult {
        DetectResult(InnerDetectResult::Pass {
            build_plan: self.build_plan,
        })
    }

    pub fn build_plan(mut self, build_plan: BuildPlan) -> Self {
        self.build_plan = Some(build_plan);
        self
    }
}

pub struct FailDetectResultBuilder;

impl FailDetectResultBuilder {
    #[allow(clippy::unused_self)]
    pub fn build(self) -> DetectResult {
        DetectResult(InnerDetectResult::Fail)
    }
}
