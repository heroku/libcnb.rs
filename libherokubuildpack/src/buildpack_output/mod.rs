//! # Buildpack output
//!
//! Use the [`BuildpackOutput`] to output structured text as a buildpack is executing.
//!
//! ```
//! use libherokubuildpack::buildpack_output::BuildpackOutput;
//!
//! let mut output = BuildpackOutput::new(std::io::stdout())
//!     .start("Heroku Ruby Buildpack")
//!     .warning("No Gemfile.lock found");
//!
//! output = output
//!     .section("Ruby version")
//!     .end_section();
//!
//! output.finish();
//! ```
//!
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

/// See the module docs for example usage.
#[allow(clippy::module_name_repetitions)]
#[derive(Debug)]
pub struct BuildpackOutput<T> {
    pub(crate) started: Option<Instant>,
    pub(crate) state: T,
}

/// Various states for [`BuildpackOutput`] to contain.
///
/// The [`BuildpackOutput`] struct acts as an output state machine. These structs
/// are meant to represent those states.
#[doc(hidden)]
pub mod state {
    use crate::buildpack_output::util::ParagraphInspectWrite;
    use crate::write::MappedWrite;
    use std::time::Instant;

    #[derive(Debug)]
    pub struct NotStarted<W> {
        pub(crate) write: ParagraphInspectWrite<W>,
    }

    #[derive(Debug)]
    pub struct Started<W> {
        pub(crate) write: ParagraphInspectWrite<W>,
    }

    #[derive(Debug)]
    pub struct Section<W> {
        pub(crate) write: ParagraphInspectWrite<W>,
    }

    pub struct Stream<W: std::io::Write> {
        pub(crate) started: Instant,
        pub(crate) write: MappedWrite<ParagraphInspectWrite<W>>,
    }
}

#[doc(hidden)]
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
    #[must_use]
    pub fn warning(mut self, s: impl AsRef<str>) -> BuildpackOutput<S> {
        self.write_paragraph(YELLOW, s);
        self
    }

    #[must_use]
    pub fn important(mut self, s: impl AsRef<str>) -> BuildpackOutput<S> {
        self.write_paragraph(CYAN, s);
        self
    }

    pub fn error(mut self, s: impl AsRef<str>) {
        self.write_paragraph(RED, s);
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
    pub fn new(io: W) -> Self {
        Self {
            state: state::NotStarted {
                write: ParagraphInspectWrite::new(io),
            },
            started: None,
        }
    }

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

    #[must_use]
    pub fn step(mut self, s: impl AsRef<str>) -> BuildpackOutput<state::Section<W>> {
        writeln_now(&mut self.state.write, Self::style(s));
        self
    }

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

    pub fn end_section(self) -> BuildpackOutput<state::Started<W>> {
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
    pub fn finish_stream(mut self) -> BuildpackOutput<state::Section<W>> {
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
            .end_section()
            .section("Hello world")
            .start_stream("Streaming stuff");

        let value = "stuff".to_string();
        writeln!(&mut stream, "{value}").unwrap();

        let io = stream.finish_stream().end_section().finish();

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

        let io = stream.finish_stream().end_section().finish();

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
            .end_section()
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
            .end_section()
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
            .end_section()
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
