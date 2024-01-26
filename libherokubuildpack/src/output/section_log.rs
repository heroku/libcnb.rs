//! Write to the build output in a `Box<dyn SectionLogger>` format with functions
//!
//! ## What
//!
//! Logging from within a layer can be difficult because calls to the layer interface are not
//! mutable nor consumable. Functions can be used at any time with no restrictions. The
//! only downside is that the buildpack author (you) is now responsible for:
//!
//! - Ensuring that `Box<dyn StartedLogger>::section()` was called right before any of these
//! functions are called.
//! - Ensuring that you are not attempting to log while already logging i.e. calling `step()` within a
//! `step_timed()` call.
//!
//! For usage details run `cargo run --bin print_style_guide`
//!
//! ## Use
//!
//! The main use case is logging inside of a layer:
//!
//! ```no_run
//! use libherokubuildpack::output::section_log::log_step;
//!
//! log_step("Clearing the cache")
//! ```
use crate::output::build_log::StreamLog;
use crate::output::build_log::{state, BuildData, BuildpackOutput};
use std::io::Stdout;

/// Output a message as a single step, ideally a short message
///
/// ```
/// use libherokubuildpack::output::section_log::log_step;
///
/// log_step("Clearing cache (ruby version changed)");
/// ```
pub fn log_step(s: impl AsRef<str>) {
    let _ = logger().step(s.as_ref());
}

/// Will print the input string and yield a `Box<dyn StreamLogger>` that can be used to print
/// to the output. The main use case is running commands
///
/// ```no_run
/// use fun_run::CommandWithName;
/// use libherokubuildpack::output::section_log::log_step_stream;
/// use libherokubuildpack::output::style;
///
/// let mut cmd = std::process::Command::new("bundle");
/// cmd.arg("install");
///
/// log_step_stream(format!("Running {}", style::command(cmd.name())), |stream| {
///     cmd.stream_output(stream.io(), stream.io()).unwrap()
/// });
/// ```
///
/// Timing information will be output at the end of the step.
pub fn log_step_stream<T>(s: impl AsRef<str>, f: impl FnOnce(&mut StreamLog<Stdout>) -> T) -> T {
    let mut stream = logger().step_timed_stream(s.as_ref());
    let out = f(&mut stream);
    let _ = stream.finish_timed_stream();
    out
}

/// Print an error block to the output
pub fn log_error(s: impl AsRef<str>) {
    logger().announce().error(s.as_ref());
}

/// Print an warning block to the output
pub fn log_warning(s: impl AsRef<str>) {
    let _ = logger().announce().warning(s.as_ref());
}

/// Print an important block to the output
pub fn log_important(s: impl AsRef<str>) {
    let _ = logger().announce().important(s.as_ref());
}

fn logger() -> BuildpackOutput<state::InSection, Stdout> {
    BuildpackOutput::<state::InSection, Stdout> {
        io: std::io::stdout(),
        // Be careful not to do anything that might access this state
        // as it's ephemeral data (i.e. not passed in from the start of the build)
        data: BuildData::default(),
        _state: state::InSection,
    }
}
