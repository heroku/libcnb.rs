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

/// Describes the outcome of the detect phase.
///
/// Besides indicating passing or failing detection, it also contains detect phase output such as
/// the build plan. To construct values of this type, use a [`DetectOutcomeBuilder`].
#[derive(Debug)]
pub struct DetectOutcome(pub(crate) InnerDetectOutcome);

#[derive(Debug)]
pub(crate) enum InnerDetectOutcome {
    Fail,
    Pass { build_plan: Option<BuildPlan> },
}

/// Constructs [`DetectOutcome`] values.
///
/// # Examples:
/// ```
/// use libcnb::detect::DetectOutcomeBuilder;
/// use libcnb_data::build_plan::{BuildPlan, BuildPlanBuilder};
///
/// let simple_pass = DetectOutcomeBuilder::pass().build();
/// let simple_fail = DetectOutcomeBuilder::fail().build();
///
/// let with_build_plan = DetectOutcomeBuilder::pass()
///    .build_plan(BuildPlanBuilder::new().provides("something").build())
///    .build();
/// ```
pub struct DetectOutcomeBuilder;

impl DetectOutcomeBuilder {
    pub fn pass() -> PassDetectOutcomeBuilder {
        PassDetectOutcomeBuilder { build_plan: None }
    }

    pub fn fail() -> FailDetectOutcomeBuilder {
        FailDetectOutcomeBuilder {}
    }
}

pub struct PassDetectOutcomeBuilder {
    build_plan: Option<BuildPlan>,
}

impl PassDetectOutcomeBuilder {
    pub fn build(self) -> DetectOutcome {
        DetectOutcome(InnerDetectOutcome::Pass {
            build_plan: self.build_plan,
        })
    }

    pub fn build_plan(mut self, build_plan: BuildPlan) -> Self {
        self.build_plan = Some(build_plan);
        self
    }
}

pub struct FailDetectOutcomeBuilder;

impl FailDetectOutcomeBuilder {
    #[allow(clippy::unused_self)]
    pub fn build(self) -> DetectOutcome {
        DetectOutcome(InnerDetectOutcome::Fail)
    }
}
