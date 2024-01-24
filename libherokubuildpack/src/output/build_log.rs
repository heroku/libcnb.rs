//! # Build output logging
//!
//! Use the `BuildLog` to output structured text as a buildpack is executing
//!
//! ```
//! use libherokubuildpack::output::build_log::BuildLog;
//!
//! let mut logger = BuildLog::new(std::io::stdout())
//!     .buildpack_name("Heroku Ruby Buildpack");
//!
//! logger = logger
//!     .section("Ruby version")
//!     .step_timed("Installing")
//!     .finish_timed_step()
//!     .end_section();
//!
//! logger.finish_logging();
//! ```
//!
//! To log inside of a layer see `section_log`.
//!
//! For usage details run `cargo run --bin print_style_guide`
use crate::output::background::{print_interval, state::PrintGuard};
use crate::output::style;
use std::fmt::Debug;
use std::io::Write;
use std::marker::PhantomData;
use std::sync::{Arc, Mutex};
use std::time::{Duration, Instant};

/// See the module docs for example usage
#[allow(clippy::module_name_repetitions)]
#[derive(Debug)]
pub struct BuildLog<T, W: Debug> {
    pub(crate) io: W,
    pub(crate) data: BuildData,
    pub(crate) state: PhantomData<T>,
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

/// Various states for `BuildLog` to contain
///
/// The `BuildLog` struct acts as a logging state machine. These structs
/// are meant to represent those states
pub(crate) mod state {
    #[derive(Debug)]
    pub struct NotStarted;

    #[derive(Debug)]
    pub struct Started;

    #[derive(Debug)]
    pub struct InSection;
}

impl<W> BuildLog<state::NotStarted, W>
where
    W: Write + Debug,
{
    pub fn new(io: W) -> Self {
        Self {
            io,
            state: PhantomData::<state::NotStarted>,
            data: BuildData::default(),
        }
    }

    pub fn buildpack_name(mut self, buildpack_name: &str) -> BuildLog<state::Started, W> {
        write_now(
            &mut self.io,
            format!("{}\n\n", style::header(buildpack_name)),
        );

        BuildLog {
            io: self.io,
            data: self.data,
            state: PhantomData::<state::Started>,
        }
    }

    pub fn without_buildpack_name(self) -> BuildLog<state::Started, W> {
        BuildLog {
            io: self.io,
            data: self.data,
            state: PhantomData::<state::Started>,
        }
    }
}

impl<W> BuildLog<state::Started, W>
where
    W: Write + Send + Sync + Debug + 'static,
{
    pub fn section(mut self, s: &str) -> BuildLog<state::InSection, W> {
        writeln_now(&mut self.io, style::section(s));

        BuildLog {
            io: self.io,
            data: self.data,
            state: PhantomData::<state::InSection>,
        }
    }

    pub fn finish_logging(mut self) {
        let elapsed = style::time::human(&self.data.started.elapsed());
        let details = style::details(format!("finished in {elapsed}"));

        writeln_now(&mut self.io, style::section(format!("Done {details}")));
    }

    pub fn announce(self) -> AnnounceLog<state::Started, W> {
        AnnounceLog {
            io: self.io,
            data: self.data,
            state: PhantomData::<state::Started>,
            leader: Some("\n".to_string()),
        }
    }
}

impl<W> BuildLog<state::InSection, W>
where
    W: Write + Send + Sync + Debug + 'static,
{
    pub fn mut_step(&mut self, s: &str) {
        writeln_now(&mut self.io, style::step(s));
    }

    #[must_use]
    pub fn step(mut self, s: &str) -> BuildLog<state::InSection, W> {
        self.mut_step(s);

        self
    }

    pub fn step_timed(self, s: &str) -> BackgroundLog<W> {
        let mut io = self.io;
        let data = self.data;
        let timer = Instant::now();

        write_now(&mut io, style::step(s));
        let dot_printer = print_interval(
            io,
            Duration::from_secs(1),
            style::background_timer_start(),
            style::background_timer_tick(),
            style::background_timer_end(),
        );

        BackgroundLog {
            data,
            timer,
            dot_printer,
        }
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

    pub fn end_section(self) -> BuildLog<state::Started, W> {
        BuildLog {
            io: self.io,
            data: self.data,
            state: PhantomData::<state::Started>,
        }
    }

    pub fn announce(self) -> AnnounceLog<state::InSection, W> {
        AnnounceLog {
            io: self.io,
            data: self.data,
            state: PhantomData::<state::InSection>,
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
    state: PhantomData<T>,
    leader: Option<String>,
}

impl<T, W> AnnounceLog<T, W>
where
    T: Debug,
    W: Write + Send + Sync + Debug + 'static,
{
    fn log_warning_shared(&mut self, s: &str) {
        if let Some(leader) = self.leader.take() {
            write_now(&mut self.io, leader);
        }

        writeln_now(&mut self.io, style::warning(s.trim()));
        writeln_now(&mut self.io, "");
    }

    fn log_important_shared(&mut self, s: &str) {
        if let Some(leader) = self.leader.take() {
            write_now(&mut self.io, leader);
        }
        writeln_now(&mut self.io, style::important(s.trim()));
        writeln_now(&mut self.io, "");
    }
}

impl<T, W> AnnounceLog<T, W>
where
    T: Debug,
    W: Write + Send + Sync + Debug + 'static,
{
    pub fn error(mut self, s: &str) {
        if let Some(leader) = self.leader.take() {
            write_now(&mut self.io, leader);
        }
        writeln_now(&mut self.io, style::error(s.trim()));
        writeln_now(&mut self.io, "");
    }
}

impl<W> AnnounceLog<state::InSection, W>
where
    W: Write + Send + Sync + Debug + 'static,
{
    #[must_use]
    pub fn warning(mut self, s: &str) -> AnnounceLog<state::InSection, W> {
        self.log_warning_shared(s);

        self
    }

    #[must_use]
    pub fn important(mut self, s: &str) -> AnnounceLog<state::InSection, W> {
        self.log_important_shared(s);

        self
    }

    pub fn end_announce(self) -> BuildLog<state::InSection, W> {
        BuildLog {
            io: self.io,
            data: self.data,
            state: PhantomData::<state::InSection>,
        }
    }
}

impl<W> AnnounceLog<state::Started, W>
where
    W: Write + Send + Sync + Debug + 'static,
{
    #[must_use]
    pub fn warning(mut self, s: &str) -> AnnounceLog<state::Started, W> {
        self.log_warning_shared(s);
        self
    }

    #[must_use]
    pub fn important(mut self, s: &str) -> AnnounceLog<state::Started, W> {
        self.log_important_shared(s);
        self
    }

    #[must_use]
    pub fn end_announce(self) -> BuildLog<state::Started, W> {
        BuildLog {
            io: self.io,
            data: self.data,
            state: PhantomData::<state::Started>,
        }
    }
}

/// Implements Box<dyn Write + Send + Sync>
///
/// Ensures that the `W` can be passed across thread boundries
/// by wrapping in a mutex.
///
/// It implements writing by unlocking and delegating to the internal writer.
/// Can be used for streaming stdout and stderr to the same writer.
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
        let mut guard = self.arc_io.lock().expect("Logging mutex posioned");
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
    pub fn finish_timed_stream(self) -> BuildLog<state::InSection, W> {
        let duration = self.started.elapsed();

        let mut io = Arc::try_unwrap(self.arc_io)
            .expect("Expected buildpack author to not retain any IO streaming IO instances")
            .into_inner()
            .expect("Logging mutex was poisioned");

        // // Newline after stream
        writeln_now(&mut io, "");

        let mut section = BuildLog {
            io,
            data: self.data,
            state: PhantomData::<state::InSection>,
        };

        section.mut_step(&format!(
            "Done {}",
            style::details(style::time::human(&duration))
        ));

        section
    }
}

/// Logs to the user while work is being performed in the background
///
/// Used to end a background inline timer i.e. Installing ...... (<0.1s)
#[derive(Debug)]
pub struct BackgroundLog<W>
where
    W: Write + Debug,
{
    data: BuildData,
    timer: Instant,
    dot_printer: PrintGuard<W>,
}

impl<W> BackgroundLog<W>
where
    W: Write + Send + Sync + Debug + 'static,
{
    #[must_use]
    pub fn finish_timed_step(self) -> BuildLog<state::InSection, W> {
        // Must stop background writing thread before retrieving IO
        let data = self.data;
        let timer = self.timer;

        let mut io = match self.dot_printer.stop() {
            Ok(io) => io,
            Err(e) => std::panic::resume_unwind(e),
        };
        let duration = timer.elapsed();

        writeln_now(&mut io, style::details(style::time::human(&duration)));

        BuildLog {
            io,
            data,
            state: PhantomData::<state::InSection>,
        }
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
    use crate::output::style::{self, strip_control_codes};
    use crate::output::util::{strip_trailing_whitespace, ReadYourWrite};
    use indoc::formatdoc;
    use libcnb_test::assert_contains;
    use pretty_assertions::assert_eq;

    #[test]
    fn test_captures() {
        let writer = ReadYourWrite::writer(Vec::new());
        let reader = writer.reader();

        let mut stream = BuildLog::new(writer)
            .buildpack_name("Heroku Ruby Buildpack")
            .section("Ruby version `3.1.3` from `Gemfile.lock`")
            .step_timed("Installing")
            .finish_timed_step()
            .end_section()
            .section("Hello world")
            .step_timed_stream("Streaming stuff");

        let value = "stuff".to_string();
        writeln!(stream.io(), "{value}").unwrap();

        stream.finish_timed_stream().end_section().finish_logging();

        let expected = formatdoc! {"

            # Heroku Ruby Buildpack

            - Ruby version `3.1.3` from `Gemfile.lock`
              - Installing ... (< 0.1s)
            - Hello world
              - Streaming stuff

                  stuff

              - Done (< 0.1s)
            - Done (finished in < 0.1s)
        "};

        assert_eq!(
            expected,
            strip_trailing_whitespace(style::strip_control_codes(reader.read_lossy().unwrap()))
        );
    }

    #[test]
    fn test_streaming_a_command() {
        let writer = ReadYourWrite::writer(Vec::new());
        let reader = writer.reader();

        let mut stream = BuildLog::new(writer)
            .buildpack_name("Streaming buildpack demo")
            .section("Command streaming")
            .step_timed_stream("Streaming stuff");

        std::process::Command::new("echo")
            .arg("hello world")
            .output_and_write_streams(stream.io(), stream.io())
            .unwrap();

        stream.finish_timed_stream().end_section().finish_logging();

        let actual =
            strip_trailing_whitespace(style::strip_control_codes(reader.read_lossy().unwrap()));

        assert_contains!(actual, "      hello world\n");
    }

    #[test]
    fn warning_step_padding() {
        let writer = ReadYourWrite::writer(Vec::new());
        let reader = writer.reader();

        BuildLog::new(writer)
            .buildpack_name("RCT")
            .section("Guest thoughs")
            .step("The scenery here is wonderful")
            .announce()
            .warning("It's too crowded here\nI'm tired")
            .end_announce()
            .step("The jumping fountains are great")
            .step("The music is nice here")
            .end_section()
            .finish_logging();

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

        assert_eq!(expected, strip_control_codes(reader.read_lossy().unwrap()));
    }

    #[test]
    fn double_warning_step_padding() {
        let writer = ReadYourWrite::writer(Vec::new());
        let reader = writer.reader();

        let logger = BuildLog::new(writer)
            .buildpack_name("RCT")
            .section("Guest thoughs")
            .step("The scenery here is wonderful")
            .announce();

        logger
            .warning("It's too crowded here")
            .warning("I'm tired")
            .end_announce()
            .step("The jumping fountains are great")
            .step("The music is nice here")
            .end_section()
            .finish_logging();

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

        assert_eq!(expected, strip_control_codes(reader.read_lossy().unwrap()));
    }

    #[test]
    fn announce_and_exit_makes_no_whitespace() {
        let writer = ReadYourWrite::writer(Vec::new());
        let reader = writer.reader();

        BuildLog::new(writer)
            .buildpack_name("Quick and simple")
            .section("Start")
            .step("Step")
            .announce() // <== Here
            .end_announce() // <== Here
            .end_section()
            .finish_logging();

        let expected = formatdoc! {"

            # Quick and simple

            - Start
              - Step
            - Done (finished in < 0.1s)
        "};

        assert_eq!(expected, strip_control_codes(reader.read_lossy().unwrap()));
    }
}
