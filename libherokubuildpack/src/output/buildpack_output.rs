//! # Build output logging
//!
//! Use the `BuildLog` to output structured text as a buildpack is executing
//!
//! ```
//! use libherokubuildpack::output::buildpack_output::BuildpackOutput;
//!
//! let mut output = BuildpackOutput::new(std::io::stdout())
//!     .start("Heroku Ruby Buildpack");
//!
//! output = output
//!     .section("Ruby version")
//!     .end_section();
//!
//! output.finish();
//! ```
//!
//! To log inside of a layer see `section_log`.
//!
//! For usage details run `cargo run --bin print_style_guide`
use crate::output::style;
use std::fmt::Debug;
use std::io::Write;
use std::sync::{Arc, Mutex};
use std::time::Instant;

/// See the module docs for example usage
#[allow(clippy::module_name_repetitions)]
#[derive(Debug)]
pub struct BuildpackOutput<T, W: Debug> {
    pub(crate) io: W,
    pub(crate) data: BuildData,
    pub(crate) _state: T,
}

/// A bag of data passed throughout the lifecycle of a `BuildLog`
#[derive(Debug)]
pub(crate) struct BuildData {
    pub(crate) started: Instant,
}

impl Default for BuildData {
    fn default() -> Self {
        Self {
            started: Instant::now(),
        }
    }
}

/// Various states for `BuildOutput` to contain
///
/// The `BuildLog` struct acts as a logging state machine. These structs
/// are meant to represent those states
pub(crate) mod state {
    #[derive(Debug)]
    pub struct NotStarted;

    #[derive(Debug)]
    pub struct Started;

    #[derive(Debug)]
    pub struct Section;
}

impl<W> BuildpackOutput<state::NotStarted, W>
where
    W: Write + Debug,
{
    pub fn new(io: W) -> Self {
        Self {
            io,
            _state: state::NotStarted,
            data: BuildData::default(),
        }
    }

    pub fn start(mut self, buildpack_name: &str) -> BuildpackOutput<state::Started, W> {
        write_now(
            &mut self.io,
            format!("{}\n\n", style::header(buildpack_name)),
        );

        self.start_silent()
    }

    pub fn start_silent(self) -> BuildpackOutput<state::Started, W> {
        BuildpackOutput {
            io: self.io,
            data: self.data,
            _state: state::Started,
        }
    }
}

impl<W> BuildpackOutput<state::Started, W>
where
    W: Write + Send + Sync + Debug + 'static,
{
    pub fn section(mut self, s: &str) -> BuildpackOutput<state::Section, W> {
        writeln_now(&mut self.io, style::section(s));

        BuildpackOutput {
            io: self.io,
            data: self.data,
            _state: state::Section,
        }
    }

    pub fn finish(mut self) -> W {
        let elapsed = style::time::human(&self.data.started.elapsed());
        let details = style::details(format!("finished in {elapsed}"));

        writeln_now(&mut self.io, style::section(format!("Done {details}")));
        self.io
    }

    pub fn announce(self) -> AnnounceLog<state::Started, W> {
        AnnounceLog {
            io: self.io,
            data: self.data,
            _state: state::Started,
            leader: Some("\n".to_string()),
        }
    }
}

impl<W> BuildpackOutput<state::Section, W>
where
    W: Write + Send + Sync + Debug + 'static,
{
    pub fn mut_step(&mut self, s: &str) {
        writeln_now(&mut self.io, style::step(s));
    }

    #[must_use]
    pub fn step(mut self, s: &str) -> BuildpackOutput<state::Section, W> {
        self.mut_step(s);

        self
    }

    pub fn step_timed_stream(mut self, s: &str) -> StreamLog<W> {
        self.mut_step(s);

        let started = Instant::now();
        let arc_io = Arc::new(Mutex::new(self.io));
        let mut stream = StreamLog {
            arc_io,
            data: self.data,
            started,
        };
        stream.start();

        stream
    }

    pub fn end_section(self) -> BuildpackOutput<state::Started, W> {
        BuildpackOutput {
            io: self.io,
            data: self.data,
            _state: state::Started,
        }
    }

    pub fn announce(self) -> AnnounceLog<state::Section, W> {
        AnnounceLog {
            io: self.io,
            data: self.data,
            _state: state::Section,
            leader: Some("\n".to_string()),
        }
    }
}

// Store internal state, print leading character exactly once on warning or important
#[derive(Debug)]
pub struct AnnounceLog<T, W>
where
    W: Write + Send + Sync + Debug + 'static,
{
    io: W,
    data: BuildData,
    _state: T,
    leader: Option<String>,
}

impl<T, W> AnnounceLog<T, W>
where
    T: Debug,
    W: Write + Send + Sync + Debug + 'static,
{
    fn log_warning(&mut self, s: &str) {
        if let Some(leader) = self.leader.take() {
            write_now(&mut self.io, leader);
        }

        writeln_now(&mut self.io, style::warning(s.trim()));
        writeln_now(&mut self.io, "");
    }

    fn log_important(&mut self, s: &str) {
        if let Some(leader) = self.leader.take() {
            write_now(&mut self.io, leader);
        }
        writeln_now(&mut self.io, style::important(s.trim()));
        writeln_now(&mut self.io, "");
    }

    pub fn error(mut self, s: &str) {
        if let Some(leader) = self.leader.take() {
            write_now(&mut self.io, leader);
        }
        writeln_now(&mut self.io, style::error(s.trim()));
        writeln_now(&mut self.io, "");
    }
}

impl<W> AnnounceLog<state::Section, W>
where
    W: Write + Send + Sync + Debug + 'static,
{
    #[must_use]
    pub fn warning(mut self, s: &str) -> AnnounceLog<state::Section, W> {
        self.log_warning(s);

        self
    }

    #[must_use]
    pub fn important(mut self, s: &str) -> AnnounceLog<state::Section, W> {
        self.log_important(s);

        self
    }

    pub fn end_announce(self) -> BuildpackOutput<state::Section, W> {
        BuildpackOutput {
            io: self.io,
            data: self.data,
            _state: state::Section,
        }
    }
}

impl<W> AnnounceLog<state::Started, W>
where
    W: Write + Send + Sync + Debug + 'static,
{
    #[must_use]
    pub fn warning(mut self, s: &str) -> AnnounceLog<state::Started, W> {
        self.log_warning(s);
        self
    }

    #[must_use]
    pub fn important(mut self, s: &str) -> AnnounceLog<state::Started, W> {
        self.log_important(s);
        self
    }

    #[must_use]
    pub fn end_announce(self) -> BuildpackOutput<state::Started, W> {
        BuildpackOutput {
            io: self.io,
            data: self.data,
            _state: state::Started,
        }
    }
}

// TODO: Decide if we need documentation for this
#[derive(Debug)]
struct LockedWriter<W> {
    arc: Arc<Mutex<W>>,
}

impl<W> Write for LockedWriter<W>
where
    W: Write + Send + Sync + Debug + 'static,
{
    fn write(&mut self, buf: &[u8]) -> std::io::Result<usize> {
        let mut io = self.arc.lock().expect("Logging mutex poisoned");
        io.write(buf)
    }

    fn flush(&mut self) -> std::io::Result<()> {
        let mut io = self.arc.lock().expect("Logging mutex poisoned");
        io.flush()
    }
}

/// Stream output to the user
///
/// Mostly used for logging a running command
#[derive(Debug)]
pub struct StreamLog<W> {
    data: BuildData,
    arc_io: Arc<Mutex<W>>,
    started: Instant,
}

impl<W> StreamLog<W>
where
    W: Write + Send + Sync + Debug + 'static,
{
    fn start(&mut self) {
        let mut guard = self.arc_io.lock().expect("Logging mutex poisoned");
        let mut io = guard.by_ref();
        // Newline before stream https://github.com/heroku/libcnb.rs/issues/582
        writeln_now(&mut io, "");
    }

    /// Yield boxed writer that can be used for formatting and streaming contents
    /// back to the logger.
    pub fn io(&mut self) -> Box<dyn Write + Send + Sync> {
        Box::new(crate::write::line_mapped(
            LockedWriter {
                arc: self.arc_io.clone(),
            },
            style::cmd_stream_format,
        ))
    }

    /// # Panics
    ///
    /// Ensure that the return of any calls to the `io` function
    /// are not retained before calling this function.
    ///
    /// This struct yields a `Box<dyn Write>` which is effectively an
    /// `Arc<Write>` to allow using the same writer for streaming stdout and stderr.
    ///
    /// If any of those boxed writers are retained then the `W` cannot
    /// be reclaimed and returned. This will cause a panic.
    #[must_use]
    pub fn finish_timed_stream(self) -> BuildpackOutput<state::Section, W> {
        let duration = self.started.elapsed();

        let mut io = Arc::try_unwrap(self.arc_io)
            .expect("Expected buildpack author to not retain any IO streaming IO instances")
            .into_inner()
            .expect("Logging mutex was poisioned");

        // // Newline after stream
        writeln_now(&mut io, "");

        let mut section = BuildpackOutput {
            io,
            data: self.data,
            _state: state::Section,
        };

        section.mut_step(&format!(
            "Done {}",
            style::details(style::time::human(&duration))
        ));

        section
    }
}

/// Internal helper, ensures that all contents are always flushed (never buffered)
///
/// This is especially important for writing individual characters to the same line
fn write_now<D: Write>(destination: &mut D, msg: impl AsRef<str>) {
    write!(destination, "{}", msg.as_ref()).expect("Logging error: UI writer closed");

    destination
        .flush()
        .expect("Logging error: UI writer closed");
}

/// Internal helper, ensures that all contents are always flushed (never buffered)
fn writeln_now<D: Write>(destination: &mut D, msg: impl AsRef<str>) {
    writeln!(destination, "{}", msg.as_ref()).expect("Logging error: UI writer closed");

    destination
        .flush()
        .expect("Logging error: UI writer closed");
}

#[cfg(test)]
mod test {
    use super::*;
    use crate::command::CommandExt;
    use crate::output::style::strip_control_codes;
    use crate::output::util::test_helpers::trim_end_lines;
    use indoc::formatdoc;
    use libcnb_test::assert_contains;
    use pretty_assertions::assert_eq;

    #[test]
    fn test_captures() {
        let writer = Vec::new();
        let mut stream = BuildpackOutput::new(writer)
            .start("Heroku Ruby Buildpack")
            .section("Ruby version `3.1.3` from `Gemfile.lock`")
            .end_section()
            .section("Hello world")
            .step_timed_stream("Streaming stuff");

        let value = "stuff".to_string();
        writeln!(stream.io(), "{value}").unwrap();

        let io = stream.finish_timed_stream().end_section().finish();

        let expected = formatdoc! {"

            # Heroku Ruby Buildpack

            - Ruby version `3.1.3` from `Gemfile.lock`
            - Hello world
              - Streaming stuff

                  stuff

              - Done (< 0.1s)
            - Done (finished in < 0.1s)
        "};

        assert_eq!(
            expected,
            trim_end_lines(strip_control_codes(String::from_utf8_lossy(&io)))
        );
    }

    #[test]
    fn test_streaming_a_command() {
        let writer = Vec::new();
        let mut stream = BuildpackOutput::new(writer)
            .start("Streaming buildpack demo")
            .section("Command streaming")
            .step_timed_stream("Streaming stuff");

        std::process::Command::new("echo")
            .arg("hello world")
            .output_and_write_streams(stream.io(), stream.io())
            .unwrap();

        let io = stream.finish_timed_stream().end_section().finish();

        let actual = trim_end_lines(strip_control_codes(String::from_utf8_lossy(&io)));

        assert_contains!(actual, "      hello world\n");
    }

    #[test]
    fn warning_step_padding() {
        let writer = Vec::new();
        let io = BuildpackOutput::new(writer)
            .start("RCT")
            .section("Guest thoughs")
            .step("The scenery here is wonderful")
            .announce()
            .warning("It's too crowded here\nI'm tired")
            .end_announce()
            .step("The jumping fountains are great")
            .step("The music is nice here")
            .end_section()
            .finish();

        let expected = formatdoc! {"

            # RCT

            - Guest thoughs
              - The scenery here is wonderful

            ! It's too crowded here
            ! I'm tired

              - The jumping fountains are great
              - The music is nice here
            - Done (finished in < 0.1s)
        "};

        assert_eq!(expected, strip_control_codes(String::from_utf8_lossy(&io)));
    }

    #[test]
    fn double_warning_step_padding() {
        let writer = Vec::new();
        let logger = BuildpackOutput::new(writer)
            .start("RCT")
            .section("Guest thoughs")
            .step("The scenery here is wonderful")
            .announce();

        let io = logger
            .warning("It's too crowded here")
            .warning("I'm tired")
            .end_announce()
            .step("The jumping fountains are great")
            .step("The music is nice here")
            .end_section()
            .finish();

        let expected = formatdoc! {"

            # RCT

            - Guest thoughs
              - The scenery here is wonderful

            ! It's too crowded here

            ! I'm tired

              - The jumping fountains are great
              - The music is nice here
            - Done (finished in < 0.1s)
        "};

        assert_eq!(expected, strip_control_codes(String::from_utf8_lossy(&io)));
    }

    #[test]
    fn announce_and_exit_makes_no_whitespace() {
        let writer = Vec::new();
        let io = BuildpackOutput::new(writer)
            .start("Quick and simple")
            .section("Start")
            .step("Step")
            .announce() // <== Here
            .end_announce() // <== Here
            .end_section()
            .finish();

        let expected = formatdoc! {"

            # Quick and simple

            - Start
              - Step
            - Done (finished in < 0.1s)
        "};

        assert_eq!(expected, strip_control_codes(String::from_utf8_lossy(&io)));
    }
}
