use std::fmt::Debug;
use std::io::Write;

#[cfg(test)]
use std::sync::{Arc, Mutex};

pub(crate) fn prefix_first_rest_lines(
    first_prefix: &str,
    rest_prefix: &str,
    contents: &str,
) -> String {
    prefix_lines(contents, move |index, _| {
        if index == 0 {
            String::from(first_prefix)
        } else {
            String::from(rest_prefix)
        }
    })
}

pub(crate) fn prefix_lines<F: Fn(usize, &str) -> String>(contents: &str, f: F) -> String {
    if contents.is_empty() {
        f(0, "")
    } else {
        contents
            .split_inclusive('\n')
            .enumerate()
            .map(|(line_index, line)| {
                let prefix = f(line_index, line);
                prefix + line
            })
            .collect()
    }
}

/// A trailing newline aware writer
///
/// A paragraph style block of text has an empty newline before and after the text.
/// When multiple paragraphs are emitted, it's important that they don't double up on empty newlines
/// or the output will look off.
///
/// This writer seeks to solve that problem by preserving knowledge of prior newline writes and
/// exposing that information to the caller.
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
    /// We need to track newlines across multiple writes to eliminate the double empty newline
    /// problem described above.
    fn write(&mut self, buf: &[u8]) -> std::io::Result<usize> {
        let trailing_newline_count = buf.iter().rev().take_while(|&&c| c == b'\n').count();
        // The buffer contains only newlines
        if buf.len() == trailing_newline_count {
            self.newlines_since_last_char += trailing_newline_count;
        } else {
            self.newlines_since_last_char = trailing_newline_count;
        }

        self.was_paragraph = self.newlines_since_last_char > 1;
        self.inner.write(buf)
    }

    fn flush(&mut self) -> std::io::Result<()> {
        self.inner.flush()
    }
}

#[cfg(test)]
#[derive(Debug)]
pub(crate) struct LockedWriter<W> {
    arc: Arc<Mutex<W>>,
}

#[cfg(test)]
impl<W> Clone for LockedWriter<W> {
    fn clone(&self) -> Self {
        Self {
            arc: self.arc.clone(),
        }
    }
}

#[cfg(test)]
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

        mutex
            .into_inner()
            .expect("Thread holding locked writer should not panic")
    }
}

#[cfg(test)]
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

#[cfg(test)]
mod test {
    use super::*;

    #[test]
    #[allow(clippy::write_with_newline)]
    fn test_paragraph_inspect_write() {
        use std::io::Write;

        let buffer: Vec<u8> = vec![];
        let mut inspect_write = ParagraphInspectWrite::new(buffer);
        assert!(!inspect_write.was_paragraph);

        write!(&mut inspect_write, "Hello World").unwrap();
        assert!(!inspect_write.was_paragraph);

        write!(&mut inspect_write, "").unwrap();
        assert!(!inspect_write.was_paragraph);

        write!(&mut inspect_write, "\n\nHello World!\n").unwrap();
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
