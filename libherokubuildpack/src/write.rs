use std::io;
use std::mem;
use std::sync::Arc;

/// Constructs a writer that buffers written data until given marker byte is encountered and
/// then applies a given mapping function to the data before passing the result to the wrapped
/// writer.
///
/// See the [`mappers`] module for a collection of commonly used mappers.
pub fn mapped<W: io::Write, F: (Fn(Vec<u8>) -> Vec<u8>) + Sync + Send + 'static>(
    w: W,
    marker_byte: u8,
    f: F,
) -> MappedWrite<W> {
    MappedWrite {
        inner: w,
        marker_byte,
        buffer: vec![],
        mapping_fn: Arc::new(f),
    }
}

/// Constructs a writer that buffers written data until an ASCII/UTF-8 newline byte (`0x0A`) is
/// encountered and then applies a given mapping function to the data before passing the result to
/// the wrapped writer.
///
/// See the [`mappers`] module for a collection of commonly used mappers.
pub fn line_mapped<W: io::Write, F: (Fn(Vec<u8>) -> Vec<u8>) + Sync + Send + 'static>(
    w: W,
    f: F,
) -> MappedWrite<W> {
    mapped(w, NEWLINE_ASCII_BYTE, f)
}

/// Constructs a writer that writes to two other writers. Similar to the UNIX `tee` command.
pub fn tee<A: io::Write, B: io::Write>(a: A, b: B) -> TeeWrite<A, B> {
    TeeWrite {
        inner_a: a,
        inner_b: b,
    }
}

/// A mapped writer that was created with the [`mapped`] or [`line_mapped`] function.
pub struct MappedWrite<W: io::Write> {
    inner: W,
    marker_byte: u8,
    buffer: Vec<u8>,
    mapping_fn: Arc<dyn (Fn(Vec<u8>) -> Vec<u8>) + Sync + Send>,
}

/// A tee writer that was created with the [`tee`] function.
pub struct TeeWrite<A: io::Write, B: io::Write> {
    inner_a: A,
    inner_b: B,
}

impl<W: io::Write> MappedWrite<W> {
    fn map_and_write_current_buffer(&mut self) -> io::Result<()> {
        self.inner
            .write_all(&(self.mapping_fn)(mem::take(&mut self.buffer)))
    }
}

impl<W: io::Write> io::Write for MappedWrite<W> {
    fn write(&mut self, buf: &[u8]) -> io::Result<usize> {
        for byte in buf {
            self.buffer.push(*byte);

            if *byte == self.marker_byte {
                self.map_and_write_current_buffer()?;
            }
        }

        Ok(buf.len())
    }

    fn flush(&mut self) -> io::Result<()> {
        self.inner.flush()
    }
}

impl<W: io::Write> Drop for MappedWrite<W> {
    fn drop(&mut self) {
        // Drop implementations must not panic. We intentionally ignore the potential error here.
        let _result = self.map_and_write_current_buffer();
    }
}

impl<A: io::Write, B: io::Write> io::Write for TeeWrite<A, B> {
    fn write(&mut self, buf: &[u8]) -> io::Result<usize> {
        self.inner_a.write_all(buf)?;
        self.inner_b.write_all(buf)?;
        Ok(buf.len())
    }

    fn flush(&mut self) -> io::Result<()> {
        self.inner_a.flush()?;
        self.inner_b.flush()
    }
}

const NEWLINE_ASCII_BYTE: u8 = 0x0Au8;

#[cfg(test)]
mod test {
    use super::tee;
    use crate::write::line_mapped;

    #[test]
    fn test_tee_write() {
        let mut a = vec![];
        let mut b = vec![];

        let mut input = "foo bar baz".as_bytes();
        std::io::copy(&mut input, &mut tee(&mut a, &mut b)).unwrap();

        assert_eq!(a, "foo bar baz".as_bytes());
        assert_eq!(a, b);
    }

    #[test]
    fn test_mapped_write() {
        let mut output = vec![];

        let mut input = "foo\nbar\nbaz".as_bytes();
        std::io::copy(
            &mut input,
            &mut line_mapped(&mut output, |line| line.repeat(2)),
        )
        .unwrap();

        assert_eq!(output, "foo\nfoo\nbar\nbar\nbazbaz".as_bytes());
    }
}

/// Mapper functions for use with the [`mapped`] and [`line_mapped`] functions.
pub mod mappers {
    /// Adds a prefix.
    ///
    /// # Example
    /// ```no_run
    /// use libherokubuildpack::write::line_mapped;
    /// use libherokubuildpack::write::mappers::add_prefix;
    /// use libherokubuildpack::command::CommandExt;
    /// use std::process::Command;
    ///
    /// Command::new("date")
    ///     .spawn_and_write_streams(
    ///         line_mapped(
    ///             std::io::stdout(),
    ///             add_prefix("date stdout> "),
    ///         ),
    ///         std::io::stderr(),
    ///     )
    ///     .and_then(|mut child| child.wait())
    ///     .unwrap();
    /// ```
    pub fn add_prefix<P: Into<Vec<u8>>>(prefix: P) -> impl Fn(Vec<u8>) -> Vec<u8> {
        let prefix = prefix.into();

        move |mut input| {
            let mut result = prefix.clone();
            result.append(&mut input);
            result
        }
    }

    /// Allows mapping the data as an UTF-8 string that was lossy converted from the data to be mapped.
    ///
    /// # Example
    /// ```no_run
    /// use libherokubuildpack::write::line_mapped;
    /// use libherokubuildpack::write::mappers::map_utf8_lossy;
    /// use libherokubuildpack::command::CommandExt;
    /// use std::process::Command;
    ///
    /// Command::new("date")
    ///     .spawn_and_write_streams(
    ///         line_mapped(
    ///             std::io::stdout(),
    ///             map_utf8_lossy(|string| string.replace("foo", "bar")),
    ///         ),
    ///         std::io::stderr(),
    ///     )
    ///     .and_then(|mut child| child.wait())
    ///     .unwrap();
    /// ```
    pub fn map_utf8_lossy<F: Fn(String) -> String>(f: F) -> impl Fn(Vec<u8>) -> Vec<u8> {
        move |input| f(String::from_utf8_lossy(&input).to_string()).into_bytes()
    }

    #[cfg(test)]
    mod test {
        use super::add_prefix;
        use super::map_utf8_lossy;

        #[test]
        fn test_add_prefix() {
            let result = (add_prefix(">> "))(String::from("Hello World!").into_bytes());
            assert_eq!(result, String::from(">> Hello World!").into_bytes());
        }

        #[test]
        fn test_map_utf8_lossy() {
            let result = (map_utf8_lossy(|input| input.replace("foo", "bar")))(
                String::from("foo = foo").into_bytes(),
            );

            assert_eq!(result, String::from("bar = bar").into_bytes());
        }
    }
}
