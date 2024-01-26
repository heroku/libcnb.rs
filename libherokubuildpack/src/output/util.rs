use lazy_static::lazy_static;
lazy_static! {
    static ref TRAILING_WHITESPACE_RE: regex::Regex = regex::Regex::new(r"\s+$").expect("clippy");
}

/// Iterator yielding every line in a string. The line includes newline character(s).
///
/// <https://stackoverflow.com/a/40457615>
///
/// The problem this solves is when iterating over lines of a string, the whitespace may be significant.
/// For example if you want to split a string and then get the original string back then calling
/// `lines().collect<Vec<_>>().join("\n")` will never preserve trailing newlines.
///
/// There's another option to `lines().fold(String::new(), |s, l| s + l + "\n")`, but that
/// always adds a trailing newline even if the original string doesn't have one.
pub(crate) struct LinesWithEndings<'a> {
    input: &'a str,
}

impl<'a> LinesWithEndings<'a> {
    pub(crate) fn from(input: &'a str) -> LinesWithEndings<'a> {
        LinesWithEndings { input }
    }
}

impl<'a> Iterator for LinesWithEndings<'a> {
    type Item = &'a str;

    #[inline]
    fn next(&mut self) -> Option<&'a str> {
        if self.input.is_empty() {
            return None;
        }
        let split = self.input.find('\n').map_or(self.input.len(), |i| i + 1);

        let (line, rest) = self.input.split_at(split);
        self.input = rest;
        Some(line)
    }
}

/// Removes trailing whitespace from lines
///
/// Useful because most editors strip trailing whitespace (in test fixtures)
/// but commands <https://github.com/heroku/libcnb.rs/issues/582> emit newlines
/// with leading spaces. These can be sanitized by removing trailing whitespace.
#[allow(dead_code)]
pub(crate) fn strip_trailing_whitespace(s: impl AsRef<str>) -> String {
    LinesWithEndings::from(s.as_ref())
        .map(|line| {
            // Remove empty indented lines
            TRAILING_WHITESPACE_RE.replace(line, "\n").to_string()
        })
        .collect::<String>()
}

#[cfg(test)]
mod test {
    use super::*;
    use std::fmt::Write;

    #[test]
    fn test_trailing_whitespace() {
        let actual = strip_trailing_whitespace("hello \n");
        assert_eq!("hello\n", &actual);

        let actual = strip_trailing_whitespace("hello\n    \nworld\n");
        assert_eq!("hello\n\nworld\n", &actual);
    }

    #[test]
    fn test_lines_with_endings() {
        let actual = LinesWithEndings::from("foo\nbar").fold(String::new(), |mut output, line| {
            let _ = write!(output, "z{line}");
            output
        });

        assert_eq!("zfoo\nzbar", actual);

        let actual =
            LinesWithEndings::from("foo\nbar\n").fold(String::new(), |mut output, line| {
                let _ = write!(output, "z{line}");
                output
            });

        assert_eq!("zfoo\nzbar\n", actual);
    }
}
