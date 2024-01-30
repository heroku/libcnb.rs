use std::fmt::Debug;
use std::io::Write;
use std::sync::{Arc, Mutex};

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
pub(crate) struct LockedWriter<W> {
    arc: Arc<Mutex<W>>,
}

impl<W> Clone for LockedWriter<W> {
    fn clone(&self) -> Self {
        Self {
            arc: self.arc.clone(),
        }
    }
}

impl<W> LockedWriter<W> {
    pub(crate) fn new(write: W) -> Self {
        LockedWriter {
            arc: Arc::new(Mutex::new(write)),
        }
    }

    pub(crate) fn unwrap(self) -> W {
        let Ok(mutex) = Arc::try_unwrap(self.arc) else {
            panic!("Expected buildpack author to not retain any IO streaming IO instances")
        };

        mutex.into_inner().expect("Output mutex was poisoned")
    }
}

impl<W> Write for LockedWriter<W>
where
    W: Write + Send + Sync + 'static,
{
    fn write(&mut self, buf: &[u8]) -> std::io::Result<usize> {
        let mut io = self.arc.lock().expect("Output mutex poisoned");
        io.write(buf)
    }

    fn flush(&mut self) -> std::io::Result<()> {
        let mut io = self.arc.lock().expect("Output mutex poisoned");
        io.flush()
    }
}

#[derive(Debug)]
pub(crate) struct ParagraphInspectWrite<W> {
    pub(crate) inner: W,
    pub(crate) was_paragraph: bool,
    pub(crate) newlines_since_last_char: usize,
}

impl<W> ParagraphInspectWrite<W> {
    pub(crate) fn new(io: W) -> Self {
        Self {
            inner: io,
            newlines_since_last_char: 0,
            was_paragraph: false,
        }
    }
}

impl<W: Write> Write for ParagraphInspectWrite<W> {
    fn write(&mut self, buf: &[u8]) -> std::io::Result<usize> {
        let newline_count = buf.iter().rev().take_while(|&&c| c == b'\n').count();
        if buf.len() == newline_count {
            self.newlines_since_last_char += newline_count;
        } else {
            self.newlines_since_last_char = newline_count;
        }

        self.was_paragraph = self.newlines_since_last_char > 1;
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

        // Double end, on multiple writes
        write!(&mut inspect_write, "End.\n").unwrap();
        write!(&mut inspect_write, "\n").unwrap();
        assert!(inspect_write.was_paragraph);

        write!(&mut inspect_write, "- The scenery here is wonderful\n").unwrap();
        write!(&mut inspect_write, "\n").unwrap();
        assert!(inspect_write.was_paragraph);
    }
}
