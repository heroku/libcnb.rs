//! This module is responsible for the logic involved in the printing to output while
//! other work is being performed. Such as printing dots while a download is being performed.
use std::io::Write;
use std::sync::mpsc::{channel, Sender};
use std::thread::JoinHandle;
use std::time::Duration;

/// Repeatedly prints `tick` to the given buffer at the given interval. The `start` argument will be printed before the first `tick` and the `end` argument will be printed after the last `tick` when the timer is stopped.
///
/// Returns a struct that allows for manually stopping the timer or will automatically stop
/// the timer if the guard is dropped. This functionality allows for errors that trigger
/// an exit of the function to not accidentally have a timer printing in the background
/// forever.
#[must_use]
pub(crate) fn print_interval<W>(
    mut buffer: W,
    interval: Duration,
    start: String,
    tick: String,
    end: String,
) -> PrintGuard<W>
where
    W: Write + Send + 'static,
{
    let (sender, receiver) = channel::<()>();

    let join_handle = std::thread::spawn(move || {
        write!(buffer, "{start}").expect("Writer should not be closed");
        buffer.flush().expect("Writer should not be closed");

        loop {
            write!(buffer, "{tick}").expect("Writer should not be closed");
            buffer.flush().expect("Writer should not be closed");

            if receiver.recv_timeout(interval).is_ok() {
                break;
            }
        }

        write!(buffer, "{end}").expect("Writer should not be closed");
        buffer.flush().expect("Writer should not be closed");

        buffer
    });

    PrintGuard::new(join_handle, sender)
}

/// Holds the reference to the background printer.
///
/// Ensures that the dot printer is stopped in the event of an error. By signaling
/// it and joining when this struct is dropped.
///
/// Gives access to the original io/buffer struct passed to the background writer
/// when the thread is manually stopped.
///
/// # Panics
///
/// Updates to this code need to take care to not introduce a panic. See
/// documentation in `PrintGuard::stop` below for more details.
#[derive(Debug)]
pub(crate) struct PrintGuard<W> {
    /// Holds the handle to the thread printing ticks in the background.
    ///
    /// Structs that implement `Drop` must ensure a valid internal state at
    /// all times due to E0509. The handle is wrapped in an option to allow the
    /// inner value to be removed while preserving internal state.
    join_handle: Option<JoinHandle<W>>,

    /// Holds the signaling method to tell the background printer
    /// to stop emitting.
    stop_signal: Sender<()>,
}

impl<W> Drop for PrintGuard<W> {
    fn drop(&mut self) {
        // A note on correctness. It might seem that it's enough to signal the thread to
        // stop, that we don't also have to join and wait for it to finish, but that's not
        // the case. The printer can still emit a value after it's told to stop.
        //
        // When that happens the output can appear in the middle of another output, such
        // as an error message if a global writer is being used such as stdout.
        // As a result we have to signal AND ensure the thread is stopped before
        // continuing.
        if let Some(join_handle) = self.join_handle.take() {
            let _ = self.stop_signal.send(());
            let _ = join_handle.join();
        }
    }
}

impl<W> PrintGuard<W> {
    /// Preserve internal state by ensuring the `Option` is always populated
    fn new(join_handle: JoinHandle<W>, sender: Sender<()>) -> Self {
        let guard = PrintGuard {
            join_handle: Some(join_handle),
            stop_signal: sender,
        };
        debug_assert!(guard.join_handle.is_some());

        guard
    }

    /// The only thing a consumer can do is stop the background printer and receive
    /// the original buffer.
    ///
    /// # Panics
    ///
    /// This code can panic if it encounters an unexpected internal state.
    /// If that happens it means there is an internal bug in this logic.
    /// To avoid a panic, developers modifying this file must obey the following
    /// rules:
    ///
    /// - Always consume the struct when accessing the Option value outside of Drop.
    /// - Never construct this struct with a `None` option value.
    ///
    /// This `Option` wrapping is needed to support implementing Drop to
    /// ensure the printing is stopped. When a struct implements drop it cannot
    /// remove it's internal state due to E0509:
    ///
    /// <https://github.com/rust-lang/rust/blob/27d8a577138c0d319a572cd1a464c2b755e577de/compiler/rustc_error_codes/src/error_codes/E0509.md>
    ///
    /// The workaround is to never allow invalid internal state by replacing the
    /// inner value with a `None` when removing it. We don't want to expose this
    /// implementation detail to the user, so instead we accept the panic, ensure
    /// the code is exercised under test, and exhaustively document why this panic
    /// exists and how developers working with this code can maintain safety.
    #[allow(clippy::panic_in_result_fn)]
    pub(crate) fn stop(mut self) -> std::thread::Result<W> {
        // Ignore if the channel is closed, likely means the thread died which
        // we want in this case.
        match self.join_handle.take() {
            Some(join_handle) => {
                let _ = self.stop_signal.send(());
                join_handle.join()
            }
            None => panic!("Internal error: Dot print internal state should never be None"),
        }
    }
}

#[cfg(test)]
mod test {
    use super::*;
    use std::fs::{File, OpenOptions};
    use tempfile::NamedTempFile;

    #[test]
    fn does_stop_does_not_panic() {
        let mut buffer: Vec<u8> = vec![];
        write!(buffer, "before").unwrap();

        let dot = print_interval(
            buffer,
            Duration::from_millis(1),
            String::from(" ."),
            String::from("."),
            String::from(". "),
        );
        let mut writer = dot.stop().unwrap();

        write!(writer, "after").unwrap();
        writer.flush().unwrap();

        assert_eq!("before ... after", String::from_utf8_lossy(&writer));
    }

    #[test]
    fn test_drop_stops_timer() {
        let tempfile = NamedTempFile::new().unwrap();
        let mut log = File::create(tempfile.path()).unwrap();
        write!(log, "before").unwrap();

        let dot = print_interval(
            log,
            Duration::from_millis(1),
            String::from(" ."),
            String::from("."),
            String::from(". "),
        );
        drop(dot);

        let mut log = OpenOptions::new()
            .append(true)
            .open(tempfile.path())
            .unwrap();
        write!(log, "after").unwrap();
        log.flush().unwrap();

        assert_eq!(
            String::from("before ... after"),
            std::fs::read_to_string(tempfile.path()).unwrap()
        );
    }
}
