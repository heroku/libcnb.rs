use std::fmt::Debug;
use std::io::Write;

/// Consuming stateful logger interface
///
/// The log pattern used by `BuildLog` is a consuming state machine that is designed to minimize
/// the amount of mistakes that can result in malformed build output.
///
/// The interface isn't stable and may need to change.

pub trait Logger: Debug {
    fn buildpack_name(self, s: &str) -> Box<dyn StartedLogger>;
    fn without_buildpack_name(self) -> Box<dyn StartedLogger>;
}

pub trait StartedLogger: Debug {
    fn section(self: Box<Self>, s: &str) -> Box<dyn SectionLogger>;
    fn finish_logging(self: Box<Self>);

    fn announce(self: Box<Self>) -> Box<dyn AnnounceLogger<ReturnTo = Box<dyn StartedLogger>>>;
}

pub trait SectionLogger: Debug {
    fn step(self: Box<Self>, s: &str) -> Box<dyn SectionLogger>;
    fn mut_step(&mut self, s: &str);
    fn step_timed(self: Box<Self>, s: &str) -> Box<dyn TimedStepLogger>;
    fn step_timed_stream(self: Box<Self>, s: &str) -> Box<dyn StreamLogger>;
    fn end_section(self: Box<Self>) -> Box<dyn StartedLogger>;

    fn announce(self: Box<Self>) -> Box<dyn AnnounceLogger<ReturnTo = Box<dyn SectionLogger>>>;
}

pub trait AnnounceLogger: ErrorLogger + Debug {
    type ReturnTo;

    fn warning(self: Box<Self>, s: &str) -> Box<dyn AnnounceLogger<ReturnTo = Self::ReturnTo>>;
    fn warn_later(self: Box<Self>, s: &str) -> Box<dyn AnnounceLogger<ReturnTo = Self::ReturnTo>>;
    fn important(self: Box<Self>, s: &str) -> Box<dyn AnnounceLogger<ReturnTo = Self::ReturnTo>>;

    fn end_announce(self: Box<Self>) -> Self::ReturnTo;
}

pub trait TimedStepLogger: Debug {
    fn finish_timed_step(self: Box<Self>) -> Box<dyn SectionLogger>;
}

pub trait StreamLogger: Debug {
    fn io(&mut self) -> Box<dyn Write + Send + Sync + 'static>;
    fn finish_timed_stream(self: Box<Self>) -> Box<dyn SectionLogger>;
}

pub trait ErrorLogger: Debug {
    fn error(self: Box<Self>, s: &str);
}
