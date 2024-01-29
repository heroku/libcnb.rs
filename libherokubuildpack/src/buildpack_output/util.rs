use std::fmt::Debug;
use std::io::Write;

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
///
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

#[derive(Debug)]
pub(crate) struct ParagraphInspectWrite<W: Debug> {
    pub(crate) inner: W,
    pub(crate) was_paragraph: bool,
}

impl<W> ParagraphInspectWrite<W>
where
    W: Debug,
{
    pub(crate) fn new(io: W) -> Self {
        Self {
            inner: io,
            was_paragraph: false,
        }
    }
}

impl<W: Write + Debug> Write for ParagraphInspectWrite<W> {
    fn write(&mut self, buf: &[u8]) -> std::io::Result<usize> {
        // Only modify `was_paragraph` if we write anything
        if !buf.is_empty() {
            // TODO: This will not work with Windows line endings
            self.was_paragraph =
                buf.len() >= 2 && buf[buf.len() - 2] == b'\n' && buf[buf.len() - 1] == b'\n';
        }

        self.inner.write(buf)
    }

    fn flush(&mut self) -> std::io::Result<()> {
        self.inner.flush()
    }
}

#[cfg(test)]
pub(crate) mod test_helpers {
    use super::*;
    use std::fmt::Write;

    /// Removes trailing whitespace from lines
    ///
    /// Useful because most editors strip trailing whitespace (in test fixtures)
    /// but commands <https://github.com/heroku/libcnb.rs/issues/582> emit newlines
    /// with leading spaces. These can be sanitized by removing trailing whitespace.
    pub(crate) fn trim_end_lines(s: impl AsRef<str>) -> String {
        LinesWithEndings::from(s.as_ref()).fold(String::new(), |mut output, line| {
            let _ = writeln!(output, "{}", line.trim_end());
            output
        })
    }
}

#[cfg(test)]
mod test {
    use super::*;
    use std::fmt::Write;

    #[test]
    fn test_trim_end_lines() {
        let actual = test_helpers::trim_end_lines("hello \n");
        assert_eq!("hello\n", &actual);

        let actual = test_helpers::trim_end_lines("hello\n    \nworld\n");
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

    #[test]
    #[allow(clippy::write_with_newline)]
    fn test_paragraph_inspect_write() {
        use std::io::Write;

        let buffer: Vec<u8> = vec![];
        let mut inspect_write = ParagraphInspectWrite::new(buffer);

        assert!(!inspect_write.was_paragraph);

        write!(&mut inspect_write, "Hello World!\n").unwrap();
        assert!(!inspect_write.was_paragraph);

        write!(&mut inspect_write, "Hello World!\n\n").unwrap();
        assert!(inspect_write.was_paragraph);

        write!(&mut inspect_write, "End.\n").unwrap();
        assert!(!inspect_write.was_paragraph);
    }
}
