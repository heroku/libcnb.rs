use crate::output::background_timer::{start_timer, StopJoinGuard, StopTimer};
#[allow(clippy::wildcard_imports)]
pub use crate::output::interface::*;
use crate::output::style;
use std::fmt::Debug;
use std::io::Write;
use std::marker::PhantomData;
use std::sync::{Arc, Mutex};
use std::time::{Duration, Instant};

/// # Build output logging
///
/// Use the `BuildLog` to output structured text as a buildpack is executing
///
/// ```
/// use libherokubuildpack::output::build_log::*;
///
/// let mut logger = BuildLog::new(std::io::stdout())
///     .buildpack_name("Heroku Ruby Buildpack");
///
/// logger = logger
///     .section("Ruby version")
///     .step_timed("Installing")
///     .finish_timed_step()
///     .end_section();
///
/// logger.finish_logging();
/// ```
///
/// To log inside of a layer see `section_log`.
///
/// For usage details run `cargo run --bin print_style_guide`

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
}

impl<W> Logger for BuildLog<state::NotStarted, W>
where
    W: Write + Send + Sync + Debug + 'static,
{
    fn buildpack_name(mut self, buildpack_name: &str) -> Box<dyn StartedLogger> {
        write_now(
            &mut self.io,
            format!("{}\n\n", style::header(buildpack_name)),
        );

        Box::new(BuildLog {
            io: self.io,
            data: self.data,
            state: PhantomData::<state::Started>,
        })
    }

    fn without_buildpack_name(self) -> Box<dyn StartedLogger> {
        Box::new(BuildLog {
            io: self.io,
            data: self.data,
            state: PhantomData::<state::Started>,
        })
    }
}

impl<W> StartedLogger for BuildLog<state::Started, W>
where
    W: Write + Send + Sync + Debug + 'static,
{
    fn section(mut self: Box<Self>, s: &str) -> Box<dyn SectionLogger> {
        writeln_now(&mut self.io, style::section(s));

        Box::new(BuildLog {
            io: self.io,
            data: self.data,
            state: PhantomData::<state::InSection>,
        })
    }

    fn finish_logging(mut self: Box<Self>) {
        let elapsed = style::time::human(&self.data.started.elapsed());
        let details = style::details(format!("finished in {elapsed}"));

        writeln_now(&mut self.io, style::section(format!("Done {details}")));
    }

    fn announce(self: Box<Self>) -> Box<dyn AnnounceLogger<ReturnTo = Box<dyn StartedLogger>>> {
        Box::new(AnnounceBuildLog {
            io: self.io,
            data: self.data,
            state: PhantomData::<state::Started>,
            leader: Some("\n".to_string()),
        })
    }
}
impl<W> SectionLogger for BuildLog<state::InSection, W>
where
    W: Write + Send + Sync + Debug + 'static,
{
    fn mut_step(&mut self, s: &str) {
        writeln_now(&mut self.io, style::step(s));
    }

    fn step(mut self: Box<Self>, s: &str) -> Box<dyn SectionLogger> {
        self.mut_step(s);

        Box::new(BuildLog {
            io: self.io,
            state: PhantomData::<state::InSection>,
            data: self.data,
        })
    }

    fn step_timed(self: Box<Self>, s: &str) -> Box<dyn TimedStepLogger> {
        let start = style::step(format!("{s}{}", style::background_timer_start()));
        let tick = style::background_timer_tick();
        let end = style::background_timer_end();

        let arc_io = Arc::new(Mutex::new(self.io));
        let background = start_timer(&arc_io, Duration::from_secs(1), start, tick, end);

        Box::new(FinishTimedStep {
            arc_io,
            background,
            data: self.data,
        })
    }

    fn step_timed_stream(mut self: Box<Self>, s: &str) -> Box<dyn StreamLogger> {
        self.mut_step(s);

        let started = Instant::now();
        let arc_io = Arc::new(Mutex::new(self.io));
        let mut stream = StreamTimed {
            arc_io,
            data: self.data,
            started,
        };
        stream.start();

        Box::new(stream)
    }

    fn end_section(self: Box<Self>) -> Box<dyn StartedLogger> {
        Box::new(BuildLog {
            io: self.io,
            data: self.data,
            state: PhantomData::<state::Started>,
        })
    }

    fn announce(self: Box<Self>) -> Box<dyn AnnounceLogger<ReturnTo = Box<dyn SectionLogger>>> {
        Box::new(AnnounceBuildLog {
            io: self.io,
            data: self.data,
            state: PhantomData::<state::InSection>,
            leader: Some("\n".to_string()),
        })
    }
}

// Store internal state, print leading character exactly once on warning or important
#[derive(Debug)]
struct AnnounceBuildLog<T, W>
where
    W: Write + Send + Sync + Debug + 'static,
{
    io: W,
    data: BuildData,
    state: PhantomData<T>,
    leader: Option<String>,
}

impl<T, W> AnnounceBuildLog<T, W>
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

    fn log_warn_later_shared(&mut self, s: &str) {
        let mut formatted = style::warning(s.trim());
        formatted.push('\n');

        match crate::output::warn_later::try_push(formatted) {
            Ok(()) => {}
            Err(error) => {
                eprintln!("[Buildpack Warning]: Cannot use the delayed warning feature due to error: {error}");
                self.log_warning_shared(s);
            }
        };
    }
}

impl<T, W> ErrorLogger for AnnounceBuildLog<T, W>
where
    T: Debug,
    W: Write + Send + Sync + Debug + 'static,
{
    fn error(mut self: Box<Self>, s: &str) {
        if let Some(leader) = self.leader.take() {
            write_now(&mut self.io, leader);
        }
        writeln_now(&mut self.io, style::error(s.trim()));
        writeln_now(&mut self.io, "");
    }
}

impl<W> AnnounceLogger for AnnounceBuildLog<state::InSection, W>
where
    W: Write + Send + Sync + Debug + 'static,
{
    type ReturnTo = Box<dyn SectionLogger>;

    fn warning(mut self: Box<Self>, s: &str) -> Box<dyn AnnounceLogger<ReturnTo = Self::ReturnTo>> {
        self.log_warning_shared(s);

        self
    }

    fn warn_later(
        mut self: Box<Self>,
        s: &str,
    ) -> Box<dyn AnnounceLogger<ReturnTo = Self::ReturnTo>> {
        self.log_warn_later_shared(s);

        self
    }

    fn important(
        mut self: Box<Self>,
        s: &str,
    ) -> Box<dyn AnnounceLogger<ReturnTo = Self::ReturnTo>> {
        self.log_important_shared(s);

        self
    }

    fn end_announce(self: Box<Self>) -> Box<dyn SectionLogger> {
        Box::new(BuildLog {
            io: self.io,
            data: self.data,
            state: PhantomData::<state::InSection>,
        })
    }
}

impl<W> AnnounceLogger for AnnounceBuildLog<state::Started, W>
where
    W: Write + Send + Sync + Debug + 'static,
{
    type ReturnTo = Box<dyn StartedLogger>;

    fn warning(mut self: Box<Self>, s: &str) -> Box<dyn AnnounceLogger<ReturnTo = Self::ReturnTo>> {
        self.log_warning_shared(s);
        self
    }

    fn warn_later(
        mut self: Box<Self>,
        s: &str,
    ) -> Box<dyn AnnounceLogger<ReturnTo = Self::ReturnTo>> {
        self.log_warn_later_shared(s);
        self
    }

    fn important(
        mut self: Box<Self>,
        s: &str,
    ) -> Box<dyn AnnounceLogger<ReturnTo = Self::ReturnTo>> {
        self.log_important_shared(s);
        self
    }

    fn end_announce(self: Box<Self>) -> Box<dyn StartedLogger> {
        Box::new(BuildLog {
            io: self.io,
            data: self.data,
            state: PhantomData::<state::Started>,
        })
    }
}

/// Implements Box<dyn Write + Send + Sync>
///
/// Ensures that the `W` can be passed across thread boundries
/// by wrapping in a mutex.
///
/// It implements writing by unlocking and delegating to the internal writer.
/// Can be used for `Box<dyn StreamLogger>::io()`
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

/// Used to implement `Box<dyn StreamLogger>` interface
///
/// Mostly used for logging a running command
#[derive(Debug)]
struct StreamTimed<W> {
    data: BuildData,
    arc_io: Arc<Mutex<W>>,
    started: Instant,
}

impl<W> StreamTimed<W>
where
    W: Write + Send + Sync + Debug,
{
    fn start(&mut self) {
        let mut guard = self.arc_io.lock().expect("Logging mutex posioned");
        let mut io = guard.by_ref();
        // Newline before stream
        writeln_now(&mut io, "");
    }
}

// Need a trait that is both write a debug
trait WriteDebug: Write + Debug {}
impl<T> WriteDebug for T where T: Write + Debug {}

/// Attempt to unwrap an io inside of an `Arc<Mutex>` if this fails because there is more
/// than a single reference don't panic, return the original IO instead.
///
/// This prevents a runtime panic and allows us to continue logging
fn try_unwrap_arc_io<W>(arc_io: Arc<Mutex<W>>) -> Box<dyn WriteDebug + Send + Sync + 'static>
where
    W: Write + Send + Sync + Debug + 'static,
{
    match Arc::try_unwrap(arc_io) {
        Ok(mutex) => Box::new(mutex.into_inner().expect("Logging mutex was poisioned")),
        Err(original) => Box::new(LockedWriter { arc: original }),
    }
}

impl<W> StreamLogger for StreamTimed<W>
where
    W: Write + Send + Sync + Debug + 'static,
{
    /// Yield boxed writer that can be used for formatting and streaming contents
    /// back to the logger.
    fn io(&mut self) -> Box<dyn Write + Send + Sync> {
        Box::new(crate::write::line_mapped(
            LockedWriter {
                arc: self.arc_io.clone(),
            },
            style::cmd_stream_format,
        ))
    }

    fn finish_timed_stream(self: Box<Self>) -> Box<dyn SectionLogger> {
        let duration = self.started.elapsed();
        let mut io = try_unwrap_arc_io(self.arc_io);

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

        Box::new(section)
    }
}

/// Implements `Box<dyn FinishTimedStep>`
///
/// Used to end a background inline timer i.e. Installing ...... (<0.1s)
#[derive(Debug)]
struct FinishTimedStep<W> {
    data: BuildData,
    arc_io: Arc<Mutex<W>>,
    background: StopJoinGuard<StopTimer>,
}

impl<W> TimedStepLogger for FinishTimedStep<W>
where
    W: Write + Send + Sync + Debug + 'static,
{
    fn finish_timed_step(self: Box<Self>) -> Box<dyn SectionLogger> {
        // Must stop background writing thread before retrieving IO
        let duration = self.background.stop().elapsed();
        let mut io = try_unwrap_arc_io(self.arc_io);

        writeln_now(&mut io, style::details(style::time::human(&duration)));

        Box::new(BuildLog {
            io,
            data: self.data,
            state: PhantomData::<state::InSection>,
        })
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
    use crate::output::warn_later::WarnGuard;
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
    fn warn_later_doesnt_output_newline() {
        let writer = ReadYourWrite::writer(Vec::new());
        let reader = writer.reader();

        let warn_later = WarnGuard::new(writer.clone());
        BuildLog::new(writer)
            .buildpack_name("Walkin' on the Sun")
            .section("So don't delay, act now, supplies are running out")
            .step("Allow if you're still alive, six to eight years to arrive")
            .step("And if you follow, there may be a tomorrow")
            .announce()
            .warn_later("And all that glitters is gold")
            .warn_later("Only shooting stars break the mold")
            .end_announce()
            .step("But if the offer's shunned")
            .step("You might as well be walking on the Sun")
            .end_section()
            .finish_logging();

        drop(warn_later);

        let expected = formatdoc! {"

            # Walkin' on the Sun

            - So don't delay, act now, supplies are running out
              - Allow if you're still alive, six to eight years to arrive
              - And if you follow, there may be a tomorrow
              - But if the offer's shunned
              - You might as well be walking on the Sun
            - Done (finished in < 0.1s)

            ! And all that glitters is gold

            ! Only shooting stars break the mold
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
