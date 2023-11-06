use crate::output::util::LinesWithEndings;
use const_format::formatcp;
use std::fmt::Write;

/// Helpers for formatting and colorizing your output

/// Decorated str for prefixing "Help:"
pub const HELP: &str = formatcp!("{IMPORTANT_COLOR}! HELP{RESET}");

/// Decorated str for prefixing "Debug info:"
pub const DEBUG_INFO: &str = formatcp!("{IMPORTANT_COLOR}Debug info{RESET}");

/// Decorate a URL for the build output
#[must_use]
pub fn url(contents: impl AsRef<str>) -> String {
    colorize(URL_COLOR, contents)
}

/// Decorate the name of a command being run i.e. `bundle install`
#[must_use]
pub fn command(contents: impl AsRef<str>) -> String {
    value(colorize(COMMAND_COLOR, contents.as_ref()))
}

/// Decorate an important value i.e. `2.3.4`
#[must_use]
pub fn value(contents: impl AsRef<str>) -> String {
    let contents = colorize(VALUE_COLOR, contents.as_ref());
    format!("`{contents}`")
}

/// Decorate additional information at the end of a line
#[must_use]
pub fn details(contents: impl AsRef<str>) -> String {
    let contents = contents.as_ref();
    format!("({contents})")
}

pub(crate) const RED: &str = "\x1B[0;31m";
pub(crate) const YELLOW: &str = "\x1B[0;33m";
pub(crate) const CYAN: &str = "\x1B[0;36m";

pub(crate) const BOLD_CYAN: &str = "\x1B[1;36m";
pub(crate) const BOLD_PURPLE: &str = "\x1B[1;35m"; // magenta

pub(crate) const DEFAULT_DIM: &str = "\x1B[2;1m"; // Default color but softer/less vibrant
pub(crate) const RESET: &str = "\x1B[0m";

#[cfg(test)]
pub(crate) const NOCOLOR: &str = "\x1B[1;39m"; // Differentiate between color clear and explicit no color https://github.com/heroku/buildpacks-ruby/pull/155#discussion_r1260029915
pub(crate) const ALL_CODES: [&str; 7] = [
    RED,
    YELLOW,
    CYAN,
    BOLD_CYAN,
    BOLD_PURPLE,
    DEFAULT_DIM,
    RESET,
];

pub(crate) const HEROKU_COLOR: &str = BOLD_PURPLE;
pub(crate) const VALUE_COLOR: &str = YELLOW;
pub(crate) const COMMAND_COLOR: &str = BOLD_CYAN;
pub(crate) const URL_COLOR: &str = CYAN;
pub(crate) const IMPORTANT_COLOR: &str = CYAN;
pub(crate) const ERROR_COLOR: &str = RED;

#[allow(dead_code)]
pub(crate) const WARNING_COLOR: &str = YELLOW;

const SECTION_PREFIX: &str = "- ";
const STEP_PREFIX: &str = "  - ";
const CMD_INDENT: &str = "      ";

/// Used with libherokubuildpack linemapped command output
///
#[must_use]
pub(crate) fn cmd_stream_format(mut input: Vec<u8>) -> Vec<u8> {
    let mut result: Vec<u8> = CMD_INDENT.into();
    result.append(&mut input);
    result
}

#[must_use]
pub(crate) fn background_timer_start() -> String {
    colorize(DEFAULT_DIM, " .")
}

#[must_use]
pub(crate) fn background_timer_tick() -> String {
    colorize(DEFAULT_DIM, ".")
}

#[must_use]
pub(crate) fn background_timer_end() -> String {
    colorize(DEFAULT_DIM, ". ")
}

#[must_use]
pub(crate) fn section(topic: impl AsRef<str>) -> String {
    prefix_indent(SECTION_PREFIX, topic)
}

#[must_use]
pub(crate) fn step(contents: impl AsRef<str>) -> String {
    prefix_indent(STEP_PREFIX, contents)
}

/// Used to decorate a buildpack
#[must_use]
pub(crate) fn header(contents: impl AsRef<str>) -> String {
    let contents = contents.as_ref();
    colorize(HEROKU_COLOR, format!("\n# {contents}"))
}

// Prefix is expected to be a single line
//
// If contents is multi line then indent additional lines to align with the end of the prefix.
pub(crate) fn prefix_indent(prefix: impl AsRef<str>, contents: impl AsRef<str>) -> String {
    let prefix = prefix.as_ref();
    let contents = contents.as_ref();
    let non_whitespace_re = regex::Regex::new("\\S").expect("Clippy");
    let clean_prefix = strip_control_codes(prefix);

    let indent_str = non_whitespace_re.replace_all(&clean_prefix, " "); // Preserve whitespace characters like tab and space, replace all characters with spaces
    let lines = LinesWithEndings::from(contents).collect::<Vec<_>>();

    if let Some((first, rest)) = lines.split_first() {
        format!(
            "{prefix}{first}{}",
            rest.iter().fold(String::new(), |mut output, line| {
                let _ = write!(output, "{indent_str}{line}");
                output
            })
        )
    } else {
        prefix.to_string()
    }
}

#[must_use]
pub(crate) fn important(contents: impl AsRef<str>) -> String {
    colorize(IMPORTANT_COLOR, bangify(contents))
}

#[must_use]
pub(crate) fn warning(contents: impl AsRef<str>) -> String {
    colorize(WARNING_COLOR, bangify(contents))
}

#[must_use]
pub(crate) fn error(contents: impl AsRef<str>) -> String {
    colorize(ERROR_COLOR, bangify(contents))
}

/// Helper method that adds a bang i.e. `!` before strings
pub(crate) fn bangify(body: impl AsRef<str>) -> String {
    prepend_each_line("!", " ", body)
}

// Ensures every line starts with `prepend`
pub(crate) fn prepend_each_line(
    prepend: impl AsRef<str>,
    separator: impl AsRef<str>,
    contents: impl AsRef<str>,
) -> String {
    let body = contents.as_ref();
    let prepend = prepend.as_ref();
    let separator = separator.as_ref();

    let lines = LinesWithEndings::from(body)
        .map(|line| {
            if line.trim().is_empty() {
                format!("{prepend}{line}")
            } else {
                format!("{prepend}{separator}{line}")
            }
        })
        .collect::<String>();
    lines
}

/// Colorizes a body while preserving existing color/reset combinations and clearing before newlines
///
/// Colors with newlines are a problem since the contents stream to git which prepends `remote:` before the `libcnb_test`
/// if we don't clear, then we will colorize output that isn't ours.
///
/// Explicitly uncolored output is handled by treating `\x1b[1;39m` (NOCOLOR) as a distinct case from `\x1b[0m`
pub(crate) fn colorize(color: &str, body: impl AsRef<str>) -> String {
    body.as_ref()
        .split('\n')
        // If sub contents are colorized it will contain SUBCOLOR ... RESET. After the reset,
        // ensure we change back to the current color
        .map(|line| line.replace(RESET, &format!("{RESET}{color}"))) // Handles nested color
        // Set the main color for each line and reset after so we don't colorize `remote:` by accident
        .map(|line| format!("{color}{line}{RESET}"))
        // The above logic causes redundant colors and resets, clean them up
        .map(|line| line.replace(&format!("{RESET}{color}{RESET}"), RESET))
        .map(|line| line.replace(&format!("{color}{color}"), color)) // Reduce useless color
        .collect::<Vec<String>>()
        .join("\n")
}

pub(crate) fn strip_control_codes(contents: impl AsRef<str>) -> String {
    let mut contents = contents.as_ref().to_string();
    for code in ALL_CODES {
        contents = contents.replace(code, "");
    }
    contents
}

#[cfg(test)]
mod test {
    use super::*;

    #[test]
    fn test_prefix_indent() {
        assert_eq!("- hello", &prefix_indent("- ", "hello"));
        assert_eq!("- hello\n  world", &prefix_indent("- ", "hello\nworld"));
        assert_eq!("- hello\n  world\n", &prefix_indent("- ", "hello\nworld\n"));
        let actual = prefix_indent(format!("- {RED}help:{RESET} "), "hello\nworld\n");
        assert_eq!(
            &format!("- {RED}help:{RESET} hello\n        world\n"),
            &actual
        );
    }

    #[test]
    fn test_bangify() {
        let actual = bangify("hello");
        assert_eq!("! hello", actual);

        let actual = bangify("\n");
        assert_eq!("!\n", actual);
    }

    #[test]
    fn handles_explicitly_removed_colors() {
        let nested = colorize(NOCOLOR, "nested");

        let out = colorize(RED, format!("hello {nested} color"));
        let expected = format!("{RED}hello {NOCOLOR}nested{RESET}{RED} color{RESET}");

        assert_eq!(expected, out);
    }

    #[test]
    fn handles_nested_colors() {
        let nested = colorize(CYAN, "nested");

        let out = colorize(RED, format!("hello {nested} color"));
        let expected = format!("{RED}hello {CYAN}nested{RESET}{RED} color{RESET}");

        assert_eq!(expected, out);
    }

    #[test]
    fn splits_newlines() {
        let actual = colorize(RED, "hello\nworld");
        let expected = format!("{RED}hello{RESET}\n{RED}world{RESET}");

        assert_eq!(expected, actual);
    }

    #[test]
    fn simple_case() {
        let actual = colorize(RED, "hello world");
        assert_eq!(format!("{RED}hello world{RESET}"), actual);
    }
}

pub(crate) mod time {
    use std::time::Duration;

    // Returns the part of a duration only in miliseconds
    pub(crate) fn milliseconds(duration: &Duration) -> u32 {
        duration.subsec_millis()
    }

    pub(crate) fn seconds(duration: &Duration) -> u64 {
        duration.as_secs() % 60
    }

    pub(crate) fn minutes(duration: &Duration) -> u64 {
        (duration.as_secs() / 60) % 60
    }

    pub(crate) fn hours(duration: &Duration) -> u64 {
        (duration.as_secs() / 3600) % 60
    }

    #[must_use]
    pub(crate) fn human(duration: &Duration) -> String {
        let hours = hours(duration);
        let minutes = minutes(duration);
        let seconds = seconds(duration);
        let miliseconds = milliseconds(duration);

        if hours > 0 {
            format!("{hours}h {minutes}m {seconds}s")
        } else if minutes > 0 {
            format!("{minutes}m {seconds}s")
        } else if seconds > 0 || miliseconds > 100 {
            // 0.1
            format!("{seconds}.{miliseconds:0>3}s")
        } else {
            String::from("< 0.1s")
        }
    }

    #[cfg(test)]
    mod test {
        use super::*;

        #[test]
        fn test_millis_and_seconds() {
            let duration = Duration::from_millis(1024);
            assert_eq!(24, milliseconds(&duration));
            assert_eq!(1, seconds(&duration));
        }

        #[test]
        fn test_display_duration() {
            let duration = Duration::from_millis(99);
            assert_eq!("< 0.1s", human(&duration).as_str());

            let duration = Duration::from_millis(1024);
            assert_eq!("1.024s", human(&duration).as_str());

            let duration = std::time::Duration::from_millis(60 * 1024);
            assert_eq!("1m 1s", human(&duration).as_str());

            let duration = std::time::Duration::from_millis(3600 * 1024);
            assert_eq!("1h 1m 26s", human(&duration).as_str());
        }
    }
}
