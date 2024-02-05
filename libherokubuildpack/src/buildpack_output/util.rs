use std::fmt::Debug;
use std::io::Write;
use std::sync::{Arc, Mutex};

/// Iterator yielding every line in a string. Every line includes existing newline character(s).
pub(crate) struct LineIterator<'a> {
    input: &'a str,
}

impl<'a> LineIterator<'a> {
    pub(crate) fn from(input: &'a str) -> LineIterator<'a> {
        LineIterator { input }
    }
}

impl<'a> Iterator for LineIterator<'a> {
    type Item = &'a str;

    #[inline]
    fn next(&mut self) -> Option<&'a str> {
        if self.input.is_empty() {
            return None;
        }

        let newline_index = self.input.find('\n').map_or(self.input.len(), |i| i + 1);

        let (line, rest) = self.input.split_at(newline_index);
        self.input = rest;
        Some(line)
    }
}

pub(crate) fn prefix_lines<F: Fn(usize, &str) -> String>(contents: &str, f: F) -> String {
    use std::fmt::Write;

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
    #[cfg(test)]
    pub(crate) fn new(write: W) -> Self {
        LockedWriter {
            arc: Arc::new(Mutex::new(write)),
        }
    }

    #[cfg(test)]
    pub(crate) fn unwrap(self) -> W {
        let Ok(mutex) = Arc::try_unwrap(self.arc) else {
            panic!("Expected buildpack author to not retain any IO streaming IO instances")
        };

        mutex
            .into_inner()
            .expect("Thread holding locked writer should not panic")
    }
}

impl<W> Write for LockedWriter<W>
where
    W: Write + Send + Sync + 'static,
{
    fn write(&mut self, buf: &[u8]) -> std::io::Result<usize> {
        let mut io = self
            .arc
            .lock()
            .expect("Thread holding locked writer should not panic");
        io.write(buf)
    }

    fn flush(&mut self) -> std::io::Result<()> {
        let mut io = self
            .arc
            .lock()
            .expect("Thread holding locked writer should not panic");
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
mod test {
    use super::*;
    use std::fmt::Write;

    #[test]
    fn test_lines_with_endings() {
        let actual = LineIterator::from("foo\nbar").fold(String::new(), |mut output, line| {
            let _ = write!(output, "z{line}");
            output
        });

        assert_eq!("zfoo\nzbar", actual);

        let actual = LineIterator::from("foo\nbar\n").fold(String::new(), |mut output, line| {
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

    #[test]
    fn test_prefix_first_rest_lines() {
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
}
