use crate::buildpack_output::ansi_escape::*;
use crate::buildpack_output::util::LineIterator;
use std::fmt::Write;

/// Helpers for formatting and colorizing your output.

/// Decorate a URL for the build output.
#[must_use]
pub fn url(contents: impl AsRef<str>) -> String {
    colorize_multiline(CYAN, contents)
}

/// Decorate the name of a command being run i.e. `bundle install`.
#[must_use]
pub fn command(contents: impl AsRef<str>) -> String {
    value(colorize_multiline(BOLD_CYAN, contents.as_ref()))
}

/// Decorate an important value i.e. `2.3.4`.
#[must_use]
pub fn value(contents: impl AsRef<str>) -> String {
    let contents = colorize_multiline(YELLOW, contents.as_ref());
    format!("`{contents}`")
}

/// Decorate additional information at the end of a line.
#[must_use]
pub fn details(contents: impl AsRef<str>) -> String {
    let contents = contents.as_ref();
    format!("({contents})")
}

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
}
