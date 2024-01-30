//! Write to the build output in a [`BuildpackOutput`] format with functions.
//!
//! ## What
//!
//! Outputting from within a layer can be difficult because calls to the layer interface are not
//! mutable nor consumable. Functions can be used at any time with no restrictions. The
//! only downside is that the buildpack author (you) is now responsible for:
//!
//! - Ensuring that [`BuildpackOutput::section`] function was called right before any of these
//! functions are called.
//! - Ensuring that you are not attempting to output while already streaming i.e. calling [`step`] within
//! a [`step_stream`] call.
//!
//! ## Use
//!
//! The main use case is writing output in a layer:
//!
//! ```no_run
//! use libherokubuildpack::buildpack_output::inline_output;
//!
//! inline_output::step("Clearing the cache")
//! ```
use crate::buildpack_output::state::TimedStream;
use crate::buildpack_output::{state, BuildpackOutput, ParagraphInspectWrite};
use std::io::Stdout;
use std::time::Instant;

/// Output a message as a single step, ideally a short message.
///
/// ```
/// use libherokubuildpack::buildpack_output::inline_output;
///
/// inline_output::step("Clearing cache (ruby version changed)");
/// ```
pub fn step(s: impl AsRef<str>) {
    let _ = build_buildpack_output().step(s.as_ref());
}

/// Will print the input string and yield a [`Stream`] that can be used to print
/// to the output. The main use case is running commands.
///
/// Timing information will be output at the end of the step.
pub fn step_stream<T>(
    s: impl AsRef<str>,
    f: impl FnOnce(&mut BuildpackOutput<TimedStream<Stdout>>) -> T,
) -> T {
    let mut stream = build_buildpack_output().step_timed_stream(s.as_ref());
    let out = f(&mut stream);
    let _ = stream.finish_timed_stream();
    out
}

/// Print an error block to the output.
pub fn error(s: impl AsRef<str>) {
    build_buildpack_output().error(s.as_ref());
}

/// Print an warning block to the output.
pub fn warning(s: impl AsRef<str>) {
    let _ = build_buildpack_output().warning(s.as_ref());
}

/// Print an important block to the output.
pub fn important(s: impl AsRef<str>) {
    let _ = build_buildpack_output().important(s.as_ref());
}

fn build_buildpack_output() -> BuildpackOutput<state::Section<Stdout>> {
    BuildpackOutput::<state::Section<Stdout>> {
        // Be careful not to do anything that might access this state
        // as it's ephemeral data (i.e. not passed in from the start of the build)
        started: Some(Instant::now()),
        state: state::Section {
            write: ParagraphInspectWrite::new(std::io::stdout()),
        },
    }
}
