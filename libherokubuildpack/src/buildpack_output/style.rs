use crate::buildpack_output::util::LineIterator;
use const_format::formatcp;
use std::fmt::Write;

/// Helpers for formatting and colorizing your output.

/// Decorate a URL for the build output.
#[must_use]
pub fn url(contents: impl AsRef<str>) -> String {
    colorize(URL_COLOR, contents)
}

/// Decorate the name of a command being run i.e. `bundle install`.
#[must_use]
pub fn command(contents: impl AsRef<str>) -> String {
    value(colorize(COMMAND_COLOR, contents.as_ref()))
}

/// Decorate an important value i.e. `2.3.4`.
#[must_use]
pub fn value(contents: impl AsRef<str>) -> String {
    let contents = colorize(VALUE_COLOR, contents.as_ref());
    format!("`{contents}`")
}

/// Decorate additional information at the end of a line.
#[must_use]
pub fn details(contents: impl AsRef<str>) -> String {
    let contents = contents.as_ref();
    format!("({contents})")
}

pub(crate) const RED: &str = "\x1B[0;31m";
pub(crate) const YELLOW: &str = "\x1B[0;33m";
pub(crate) const CYAN: &str = "\x1B[0;36m";

pub(crate) const BOLD_CYAN: &str = "\x1B[1;36m";
pub(crate) const BOLD_PURPLE: &str = "\x1B[1;35m"; // Magenta

#[cfg(test)]
pub(crate) const DEFAULT_DIM: &str = "\x1B[2;1m"; // Default color but softer/less vibrant
pub(crate) const RESET: &str = "\x1B[0m";

#[cfg(test)]
pub(crate) const NO_COLOR: &str = "\x1B[1;39m"; // Differentiate between color clear and explicit no color https://github.com/heroku/buildpacks-ruby/pull/155#discussion_r1260029915
#[cfg(test)]
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
pub(crate) const WARNING_COLOR: &str = YELLOW;

pub(crate) fn prefix_lines<F: Fn(usize, &str) -> String>(contents: &str, f: F) -> String {
    let lines = LineIterator::from(contents).enumerate().fold(
        String::new(),
        |mut acc, (line_index, line)| {
            let prefix = f(line_index, line);
            let _ = write!(acc, "{prefix}{line}");
            acc
        },
    );

    if lines.is_empty() {
        f(0, "")
    } else {
        lines
    }
}

pub(crate) fn prefix_first_rest_lines(
    first_prefix: &str,
    rest_prefix: &str,
    contents: &str,
) -> String {
    let first_prefix = String::from(first_prefix);
    let rest_prefix = String::from(rest_prefix);

    prefix_lines(contents, move |index, _| {
        if index == 0 {
            first_prefix.clone()
        } else {
            rest_prefix.clone()
        }
    })
}

/// Helper method that adds a bang i.e. `!` before strings.
pub(crate) fn bangify(body: impl AsRef<str>) -> String {
    prefix_lines(body.as_ref(), |_, line| {
        if line.chars().all(char::is_whitespace) {
            String::from("!")
        } else {
            String::from("! ")
        }
    })
}

/// Colorizes a body while preserving existing color/reset combinations and clearing before newlines.
///
/// Colors with newlines are a problem since the contents stream to git which prepends `remote:` before the `libcnb_test`
/// if we don't clear, then we will colorize output that isn't ours.
///
/// Explicitly uncolored output is handled by treating `\x1b[1;39m` (`NO_COLOR`) as a distinct case from `\x1b[0m`.
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
        .map(|line| line.replace(&format!("{color}{RESET}"), "")) // Do not colorize empty lines
        .collect::<Vec<String>>()
        .join("\n")
}

#[cfg(test)]
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
        assert_eq!("- hello", &prefix_first_rest_lines("- ", "  ", "hello"));
        assert_eq!(
            "- hello\n  world",
            &prefix_first_rest_lines("- ", "  ", "hello\nworld")
        );
        assert_eq!(
            "- hello\n  world\n",
            &prefix_first_rest_lines("- ", "  ", "hello\nworld\n")
        );

        assert_eq!("- ", &prefix_first_rest_lines("- ", "  ", ""));
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
        let nested = colorize(NO_COLOR, "nested");

        let out = colorize(RED, format!("hello {nested} color"));
        let expected = format!("{RED}hello {NO_COLOR}nested{RESET}{RED} color{RESET}");

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
