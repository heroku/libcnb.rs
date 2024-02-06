#![doc = include_str!("./README.md")]

use crate::buildpack_output::ansi_escape::{BOLD_PURPLE, CYAN, RED, YELLOW};
use crate::buildpack_output::util::{prefix_first_rest_lines, prefix_lines, ParagraphInspectWrite};
use crate::write::line_mapped;
use std::fmt::Debug;
use std::io::Write;
use std::time::Instant;

mod ansi_escape;
mod duration_format;
pub mod style;
mod util;

#[doc = include_str!("./README.md")]
#[allow(clippy::module_name_repetitions)]
#[derive(Debug)]
pub struct BuildpackOutput<T> {
    pub(crate) started: Option<Instant>,
    pub(crate) state: T,
}

/// Various states for [`BuildpackOutput`] to contain.
///
/// The [`BuildpackOutput`] struct acts as an output state machine. These structs
/// represent the various states. See struct documentation for more details.
pub mod state {
    use crate::buildpack_output::util::ParagraphInspectWrite;
    use crate::write::MappedWrite;
    use std::time::Instant;

    /// An initialized buildpack output that has not announced its start.
    ///
    /// It is represented by the`state::NotStarted` type and is transitioned into a `state::Started` type.
    ///
    /// Example:
    ///
    /// ```rust
    /// use libherokubuildpack::buildpack_output::{BuildpackOutput, state::{NotStarted, Started}};
    /// use std::io::Write;
    ///
    /// let mut not_started = BuildpackOutput::new(std::io::stdout());
    /// let output = start_buildpack(not_started);
    ///
    /// output.section("Ruby version").step("Installing Ruby").finish();
    ///
    /// fn start_buildpack<W>(mut output: BuildpackOutput<NotStarted<W>>) -> BuildpackOutput<Started<W>>
    /// where W: Write + Send + Sync + 'static {
    ///     output.start("Example Buildpack")
    ///}
    /// ```
    #[derive(Debug)]
    pub struct NotStarted<W> {
        pub(crate) write: ParagraphInspectWrite<W>,
    }

    /// After the buildpack output has started, its top-level output will be represented by the
    /// `state::Started` type and is transitioned into a `state::Section` to provide additional
    /// details.
    ///
    /// Example:
    ///
    /// ```rust
    /// use libherokubuildpack::buildpack_output::{BuildpackOutput, state::{Started, Section}};
    /// use std::io::Write;
    ///
    /// let mut output = BuildpackOutput::new(std::io::stdout())
    ///     .start("Example Buildpack");
    ///
    /// output = install_ruby(output).finish();
    ///
    /// fn install_ruby<W>(mut output: BuildpackOutput<Started<W>>) -> BuildpackOutput<Section<W>>
    /// where W: Write + Send + Sync + 'static {
    ///     let out = output.section("Ruby version")
    ///         .step("Installing Ruby");
    ///     // ...
    ///     out
    ///}
    /// ```
    #[derive(Debug)]
    pub struct Started<W> {
        pub(crate) write: ParagraphInspectWrite<W>,
    }

    /// The `state::Section` is intended to provide addiitonal details about the buildpack's
    /// actions. When a section is finished, it transitions back to a `state::Started` type.
    ///
    /// A streaming type can be started from a `state::Section`, usually to run and stream a
    /// `process::Command` to the end user.
    ///
    /// Example:
    ///
    /// ```rust
    /// use libherokubuildpack::buildpack_output::{BuildpackOutput, state::{Started, Section}};
    /// use std::io::Write;
    ///
    /// let mut output = BuildpackOutput::new(std::io::stdout())
    ///     .start("Example Buildpack")
    ///     .section("Ruby version");
    ///
    /// install_ruby(output).finish();
    ///
    /// fn install_ruby<W>(mut output: BuildpackOutput<Section<W>>) -> BuildpackOutput<Started<W>>
    /// where W: Write + Send + Sync + 'static {
    ///     let output = output.step("Installing Ruby");
    ///     // ...
    ///
    ///     output.finish()
    ///}
    /// ```
    #[derive(Debug)]
    pub struct Section<W> {
        pub(crate) write: ParagraphInspectWrite<W>,
    }

    /// A this state is intended for streaming output from a process to the end user. It is
    /// started from a `state::Section` and finished back to a `state::Section`.
    ///
    /// The `BuildpackOutput<state::Stream<W>>` implements [`std::io::Write`], so you can stream from anything that accepts a [`std::io::Write`].
    ///
    /// ```rust
    /// use libherokubuildpack::buildpack_output::{BuildpackOutput, state::{Started, Section}};
    /// use std::io::Write;
    ///
    /// let mut output = BuildpackOutput::new(std::io::stdout())
    ///     .start("Example Buildpack")
    ///     .section("Ruby version");
    ///
    /// install_ruby(output).finish();
    ///
    /// fn install_ruby<W>(mut output: BuildpackOutput<Section<W>>) -> BuildpackOutput<Section<W>>
    /// where W: Write + Send + Sync + 'static {
    ///     let mut stream = output.step("Installing Ruby")
    ///         .start_stream("Streaming stuff");
    ///
    ///     write!(&mut stream, "...").unwrap();
    ///
    ///     stream.finish()
    ///}
    /// ```
    pub struct Stream<W: std::io::Write> {
        pub(crate) started: Instant,
        pub(crate) write: MappedWrite<ParagraphInspectWrite<W>>,
    }
}

trait AnnounceSupportedState {
    type Inner: Write;

    fn write_mut(&mut self) -> &mut ParagraphInspectWrite<Self::Inner>;
}

impl<W> AnnounceSupportedState for state::Section<W>
where
    W: Write,
{
    type Inner = W;

    fn write_mut(&mut self) -> &mut ParagraphInspectWrite<Self::Inner> {
        &mut self.write
    }
}

impl<W> AnnounceSupportedState for state::Started<W>
where
    W: Write,
{
    type Inner = W;

    fn write_mut(&mut self) -> &mut ParagraphInspectWrite<Self::Inner> {
        &mut self.write
    }
}

#[allow(private_bounds)]
impl<S> BuildpackOutput<S>
where
    S: AnnounceSupportedState,
{
    /// Emit an error and end the build output
    ///
    /// When an unrecoverable situation is encountered, you can emit an error message to the user.
    /// This associated function will consume the build output, so you may only emit one error per build output.
    ///
    /// An error message should describe what went wrong and why the buildpack cannot continue.
    /// It is best practice to include debugging information in the error message. For example,
    /// if a file is missing, consider showing the user the contents of the directory where the file was expected to be
    /// and the full path of the file.
    ///
    /// If you are confident about what action needs to be taken to fix the error, you should include that in the error message.
    /// Do not write a generic suggestion like "try again later" unless you are certain that the error is transient.
    ///
    /// If you detect something problematic but not bad enough to halt buildpack execution, consider using a [`BuildpackOutput::warning`] instead.
    pub fn error(mut self, s: impl AsRef<str>) {
        self.write_paragraph(RED, s);
    }

    /// Emit a warning message to the end user.
    ///
    /// A warning should be used to emit a message to the end user about a potential problem.
    ///
    /// Multiple warnings can be emitted in sequence. The buildpack author should take care not to overwhelm the end user with
    /// unnecessary warnings.
    ///
    /// When emitting a warning, describe the problem to the user, if possible, and tell them how to fix it or where to look next.
    ///
    /// Warnings should often come with some disabling mechanism, if possible. If the user can turn off the warning,
    /// that information should be included in the warning message. If you're confident that the user should not be able to
    /// turn off a warning; use an [`BuildpackOutput::error`] instead
    ///
    /// Warnings will be output in a multi-line paragraph style. A warning can be emitted from any state except for [`state::NotStarted`].
    #[must_use]
    pub fn warning(mut self, s: impl AsRef<str>) -> BuildpackOutput<S> {
        self.write_paragraph(YELLOW, s);
        self
    }

    /// Emit an important message to the end user.
    ///
    /// When something significant happens but is not inherently negative, you can use an important message. For example,
    /// if a buildpack detects that the operating system or architecture has changed since the last build, it might not be a problem,
    ///, but if something goes wrong, the user should know about it.
    ///
    /// Important messages should be used sparingly and only for things the user should be aware of but not necessarily act on.
    /// If the message is actionable, consider using a [`BuildpackOutput::warning`] instead.
    #[must_use]
    pub fn important(mut self, s: impl AsRef<str>) -> BuildpackOutput<S> {
        self.write_paragraph(CYAN, s);
        self
    }

    fn write_paragraph(&mut self, color: &str, s: impl AsRef<str>) {
        let io = self.state.write_mut();

        if !io.was_paragraph {
            writeln_now(io, "");
        }

        writeln_now(
            io,
            ansi_escape::inject_default_ansi_escape(
                color,
                prefix_lines(s.as_ref(), |_, line| {
                    if line.chars().all(char::is_whitespace) {
                        String::from("!")
                    } else {
                        String::from("! ")
                    }
                }),
            ),
        );
        writeln_now(io, "");
    }
}

impl<W> BuildpackOutput<state::NotStarted<W>>
where
    W: Write,
{
    /// Create a buildpack output struct, but do not announce the buildpack's start.
    ///
    /// See the [`BuildpackOutput::start`] method for more details.
    pub fn new(io: W) -> Self {
        Self {
            state: state::NotStarted {
                write: ParagraphInspectWrite::new(io),
            },
            started: None,
        }
    }

    /// Announce the start of the buildpack.
    ///
    /// The input should be the human-readable name of your buildpack. Most buildpack names include
    /// the feature they provide.
    ///
    /// It is common to use a title case for the buildpack name and to include the word "Buildpack" at the end.
    /// For example, `Ruby Buildpack`. Do not include a period at the end of the name.
    ///
    /// Avoid starting your buildpack with "Heroku" unless you work for Heroku. If you wish to express that your
    /// buildpack is built to target only Heroku; you can include that in the description of the buildpack.
    ///
    /// This function will transition your buildpack output to [`state::Started`].
    pub fn start(mut self, buildpack_name: impl AsRef<str>) -> BuildpackOutput<state::Started<W>> {
        writeln_now(
            &mut self.state.write,
            ansi_escape::inject_default_ansi_escape(
                BOLD_PURPLE,
                format!("\n# {}\n", buildpack_name.as_ref()),
            ),
        );

        self.start_silent()
    }

    /// Start a buildpack output without announcing the name
    pub fn start_silent(self) -> BuildpackOutput<state::Started<W>> {
        BuildpackOutput {
            started: Some(Instant::now()),
            state: state::Started {
                write: self.state.write,
            },
        }
    }
}

impl<W> BuildpackOutput<state::Started<W>>
where
    W: Write + Send + Sync + 'static,
{
    const PREFIX_FIRST: &'static str = "- ";
    const PREFIX_REST: &'static str = "  ";

    fn style(s: impl AsRef<str>) -> String {
        prefix_first_rest_lines(Self::PREFIX_FIRST, Self::PREFIX_REST, s.as_ref())
    }

    /// Begin a new section of the buildpack output.
    ///
    /// A section should be a noun, e.g., 'Ruby version'. Anything emitted within the section should be in the context of this output.
    ///
    /// If the following steps can change based on input, consider grouping shared information such as version numbers and sources
    /// in the section name e.g., 'Ruby version ``3.1.3`` from ``Gemfile.lock``'.
    ///
    /// This function will transition your buildpack output to [`state::Section`].
    #[must_use]
    pub fn section(mut self, s: impl AsRef<str>) -> BuildpackOutput<state::Section<W>> {
        writeln_now(&mut self.state.write, Self::style(s));

        BuildpackOutput {
            started: self.started,
            state: state::Section {
                write: self.state.write,
            },
        }
    }

    /// Announce that your buildpack has finished execution successfully.
    pub fn finish(mut self) -> W {
        if let Some(started) = &self.started {
            let elapsed = duration_format::human(&started.elapsed());
            let details = style::details(format!("finished in {elapsed}"));
            writeln_now(
                &mut self.state.write,
                Self::style(format!("Done {details}")),
            );
        } else {
            writeln_now(&mut self.state.write, Self::style("Done"));
        }

        self.state.write.inner
    }
}

impl<W> BuildpackOutput<state::Section<W>>
where
    W: Write + Send + Sync + 'static,
{
    const PREFIX_FIRST: &'static str = "  - ";
    const PREFIX_REST: &'static str = "    ";
    const CMD_INDENT: &'static str = "      ";

    fn style(s: impl AsRef<str>) -> String {
        prefix_first_rest_lines(Self::PREFIX_FIRST, Self::PREFIX_REST, s.as_ref())
    }

    /// Emit a step in the buildpack output within a section.
    ///
    /// A step should be a verb, i.e., 'Downloading'. Related verbs should be nested under a single section.
    ///
    /// Some example verbs to use:
    ///
    /// - Downloading
    /// - Writing
    /// - Using
    /// - Reading
    /// - Clearing
    /// - Skipping
    /// - Detecting
    /// - Compiling
    /// - etc.
    ///
    /// Steps should be short and stand-alone sentences within the context of the section header.
    ///
    /// In general, if the buildpack did something different between two builds, it should be observable
    /// by the user through the buildpack output. For example, if a cache needs to be cleared, emit that your buildpack is clearing it and why.
    ///
    /// Multiple steps are allowed within a section. This function returns to the same [`state::Section`].
    #[must_use]
    pub fn step(mut self, s: impl AsRef<str>) -> BuildpackOutput<state::Section<W>> {
        writeln_now(&mut self.state.write, Self::style(s));
        self
    }

    /// Stream output to the end user
    ///
    /// The most common use case is to stream the output of a running `std::process::Command` to the end user.
    /// Streaming lets the end user know that something is happening and provides them with the output of the process.
    ///
    /// The result of this function is a `BuildpackOutput<state::Stream<W>>` which implements [`std::io::Write`].
    ///
    /// If you do not wish the end user to view the output of the process, consider using a `step` instead.
    ///
    /// This function will transition your buildpack output to [`state::Stream`].
    pub fn start_stream(mut self, s: impl AsRef<str>) -> BuildpackOutput<state::Stream<W>> {
        writeln_now(&mut self.state.write, Self::style(s));
        writeln_now(&mut self.state.write, "");

        BuildpackOutput {
            started: self.started,
            state: state::Stream {
                started: Instant::now(),
                write: line_mapped(self.state.write, |mut input| {
                    if input.iter().all(u8::is_ascii_whitespace) {
                        input
                    } else {
                        let mut result: Vec<u8> = Self::CMD_INDENT.into();
                        result.append(&mut input);
                        result
                    }
                }),
            },
        }
    }

    /// Finish a section and transition back to [`state::Started`].
    pub fn finish(self) -> BuildpackOutput<state::Started<W>> {
        BuildpackOutput {
            started: self.started,
            state: state::Started {
                write: self.state.write,
            },
        }
    }
}

impl<W> BuildpackOutput<state::Stream<W>>
where
    W: Write + Send + Sync + 'static,
{
    /// Finalize a stream's output
    ///
    /// Once you're finished streaming to the output, calling this function
    /// finalizes the stream's output and transitions back to a [`state::Section`].
    pub fn finish(mut self) -> BuildpackOutput<state::Section<W>> {
        let duration = self.state.started.elapsed();

        writeln_now(&mut self.state.write, "");

        BuildpackOutput {
            started: self.started,
            state: state::Section {
                write: self.state.write.unwrap(),
            },
        }
        .step(format!(
            "Done {}",
            style::details(duration_format::human(&duration))
        ))
    }
}

impl<W> Write for BuildpackOutput<state::Stream<W>>
where
    W: Write,
{
    fn write(&mut self, buf: &[u8]) -> std::io::Result<usize> {
        self.state.write.write(buf)
    }

    fn flush(&mut self) -> std::io::Result<()> {
        self.state.write.flush()
    }
}

/// Internal helper, ensures that all contents are always flushed (never buffered).
fn writeln_now<D: Write>(destination: &mut D, msg: impl AsRef<str>) {
    writeln!(destination, "{}", msg.as_ref()).expect("Output error: UI writer closed");

    destination.flush().expect("Output error: UI writer closed");
}

#[cfg(test)]
mod test {
    use super::*;
    use crate::buildpack_output::util::LockedWriter;
    use crate::command::CommandExt;
    use indoc::formatdoc;
    use libcnb_test::assert_contains;

    #[test]
    fn test_captures() {
        let writer = Vec::new();
        let mut stream = BuildpackOutput::new(writer)
            .start("Heroku Ruby Buildpack")
            .section("Ruby version `3.1.3` from `Gemfile.lock`")
            .finish()
            .section("Hello world")
            .start_stream("Streaming stuff");

        let value = "stuff".to_string();
        writeln!(&mut stream, "{value}").unwrap();

        let io = stream.finish().finish().finish();

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
            strip_ansi_escape_sequences(String::from_utf8_lossy(&io))
        );
    }

    #[test]
    fn test_streaming_a_command() {
        let writer = Vec::new();
        let mut stream = BuildpackOutput::new(writer)
            .start("Streaming buildpack demo")
            .section("Command streaming")
            .start_stream("Streaming stuff");

        let locked_writer = LockedWriter::new(stream);

        std::process::Command::new("echo")
            .arg("hello world")
            .output_and_write_streams(locked_writer.clone(), locked_writer.clone())
            .unwrap();

        stream = locked_writer.unwrap();

        let io = stream.finish().finish().finish();

        let actual = strip_ansi_escape_sequences(String::from_utf8_lossy(&io));

        assert_contains!(actual, "      hello world\n");
    }

    #[test]
    fn warning_after_buildpack() {
        let writer = Vec::new();
        let io = BuildpackOutput::new(writer)
            .start("RCT")
            .warning("It's too crowded here\nI'm tired")
            .section("Guest thoughts")
            .step("The jumping fountains are great")
            .step("The music is nice here")
            .finish()
            .finish();

        let expected = formatdoc! {"

            # RCT

            ! It's too crowded here
            ! I'm tired

            - Guest thoughts
              - The jumping fountains are great
              - The music is nice here
            - Done (finished in < 0.1s)
        "};

        assert_eq!(
            expected,
            strip_ansi_escape_sequences(String::from_utf8_lossy(&io))
        );
    }

    #[test]
    fn warning_step_padding() {
        let writer = Vec::new();
        let io = BuildpackOutput::new(writer)
            .start("RCT")
            .section("Guest thoughts")
            .step("The scenery here is wonderful")
            .warning("It's too crowded here\nI'm tired")
            .step("The jumping fountains are great")
            .step("The music is nice here")
            .finish()
            .finish();

        let expected = formatdoc! {"

            # RCT

            - Guest thoughts
              - The scenery here is wonderful

            ! It's too crowded here
            ! I'm tired

              - The jumping fountains are great
              - The music is nice here
            - Done (finished in < 0.1s)
        "};

        assert_eq!(
            expected,
            strip_ansi_escape_sequences(String::from_utf8_lossy(&io))
        );
    }

    #[test]
    fn double_warning_step_padding() {
        let writer = Vec::new();
        let output = BuildpackOutput::new(writer)
            .start("RCT")
            .section("Guest thoughts")
            .step("The scenery here is wonderful");

        let io = output
            .warning("It's too crowded here")
            .warning("I'm tired")
            .step("The jumping fountains are great")
            .step("The music is nice here")
            .finish()
            .finish();

        let expected = formatdoc! {"

            # RCT

            - Guest thoughts
              - The scenery here is wonderful

            ! It's too crowded here

            ! I'm tired

              - The jumping fountains are great
              - The music is nice here
            - Done (finished in < 0.1s)
        "};

        assert_eq!(
            expected,
            strip_ansi_escape_sequences(String::from_utf8_lossy(&io))
        );
    }

    fn strip_ansi_escape_sequences(contents: impl AsRef<str>) -> String {
        let mut result = String::new();
        let mut in_ansi_escape = false;
        for char in contents.as_ref().chars() {
            if in_ansi_escape {
                if char == 'm' {
                    in_ansi_escape = false;
                    continue;
                }
            } else {
                if char == '\x1B' {
                    in_ansi_escape = true;
                    continue;
                }

                result.push(char);
            }
        }

        result
    }
}
