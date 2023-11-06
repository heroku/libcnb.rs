use lazy_static::lazy_static;
use std::fmt::Debug;
use std::io::Write;
use std::ops::Deref;
use std::sync::{Arc, Mutex, MutexGuard, PoisonError};

lazy_static! {
    static ref TRAILING_WHITESPACE_RE: regex::Regex = regex::Regex::new(r"\s+$").expect("clippy");
}

/// Threadsafe writer that can be read from
///
/// Useful for testing
#[derive(Debug)]
pub(crate) struct ReadYourWrite<W>
where
    W: Write + AsRef<[u8]>,
{
    arc: Arc<Mutex<W>>,
}

impl<W> Clone for ReadYourWrite<W>
where
    W: Write + AsRef<[u8]> + Debug,
{
    fn clone(&self) -> Self {
        Self {
            arc: self.arc.clone(),
        }
    }
}

impl<W> Write for ReadYourWrite<W>
where
    W: Write + AsRef<[u8]> + Debug,
{
    fn write(&mut self, buf: &[u8]) -> std::io::Result<usize> {
        let mut writer = self.arc.lock().expect("Internal error");
        writer.write(buf)
    }

    fn flush(&mut self) -> std::io::Result<()> {
        let mut writer = self.arc.lock().expect("Internal error");
        writer.flush()
    }
}

impl<W> ReadYourWrite<W>
where
    W: Write + AsRef<[u8]>,
{
    #[allow(dead_code)]
    pub(crate) fn writer(writer: W) -> Self {
        Self {
            arc: Arc::new(Mutex::new(writer)),
        }
    }

    #[must_use]
    #[allow(dead_code)]
    pub(crate) fn reader(&self) -> Reader<W> {
        Reader {
            arc: self.arc.clone(),
        }
    }

    #[must_use]
    #[allow(dead_code)]
    pub(crate) fn arc_io(&self) -> Arc<Mutex<W>> {
        self.arc.clone()
    }
}

pub(crate) struct Reader<W>
where
    W: Write + AsRef<[u8]>,
{
    arc: Arc<Mutex<W>>,
}

impl<W> Reader<W>
where
    W: Write + AsRef<[u8]>,
{
    #[allow(dead_code)]
    pub(crate) fn read_lossy(&self) -> Result<String, PoisonError<MutexGuard<'_, W>>> {
        let io = &self.arc.lock()?;

        Ok(String::from_utf8_lossy(io.as_ref()).to_string())
    }
}

impl<W> Deref for Reader<W>
where
    W: Write + AsRef<[u8]>,
{
    type Target = Arc<Mutex<W>>;

    fn deref(&self) -> &Self::Target {
        &self.arc
    }
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
/// with leading spaces. These can be sanatized by removing trailing whitespace.
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
