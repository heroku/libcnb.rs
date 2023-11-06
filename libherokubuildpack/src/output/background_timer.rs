use std::io::Write;
use std::sync::mpsc::Sender;
use std::sync::{mpsc, Arc, Mutex};
use std::thread::JoinHandle;
use std::time::{Duration, Instant};

/// This module is responsible for the logic involved in the printing to output while
/// other work is being performed.

/// Prints a start, then a tick every second, and an end to the given `Write` value.
///
/// Returns a struct that allows for manually stopping the timer or will automatically stop
/// the timer if the guard is dropped. This functionality allows for errors that trigger
/// an exit of the function to not accidentally have a timer printing in the background
/// forever.
pub(crate) fn start_timer<T>(
    arc_io: &Arc<Mutex<T>>,
    tick_duration: Duration,
    start: impl AsRef<str>,
    tick: impl AsRef<str>,
    end: impl AsRef<str>,
) -> StopJoinGuard<StopTimer>
where
    // The 'static lifetime means as long as something holds a reference to it, nothing it references
    // will go away.
    //
    // From https://users.rust-lang.org/t/why-does-thread-spawn-need-static-lifetime-for-generic-bounds/4541
    //
    //   [lifetimes] refer to the minimum possible lifetime of any borrowed references that the object contains.
    T: Write + Send + Sync + 'static,
{
    let instant = Instant::now();
    let (sender, receiver) = mpsc::channel::<()>();
    let start = start.as_ref().to_string();
    let tick = tick.as_ref().to_string();
    let end = end.as_ref().to_string();

    let arc_io = arc_io.clone();
    let handle = std::thread::spawn(move || {
        let mut io = arc_io.lock().expect("Logging mutex poisoned");
        write!(&mut io, "{start}").expect("Internal error");
        io.flush().expect("Internal error");
        loop {
            write!(&mut io, "{tick}").expect("Internal error");
            io.flush().expect("Internal error");

            if receiver.recv_timeout(tick_duration).is_ok() {
                write!(&mut io, "{end}").expect("Internal error");
                io.flush().expect("Internal error");
                break;
            }
        }
    });

    StopJoinGuard {
        inner: Some(StopTimer {
            handle: Some(handle),
            sender: Some(sender),
            instant,
        }),
    }
}

/// Responsible for stopping a running timer thread
#[derive(Debug)]
pub(crate) struct StopTimer {
    instant: Instant,
    handle: Option<JoinHandle<()>>,
    sender: Option<Sender<()>>,
}

impl StopTimer {
    pub(crate) fn elapsed(&self) -> Duration {
        self.instant.elapsed()
    }
}

pub(crate) trait StopJoin: std::fmt::Debug {
    fn stop_join(self) -> Self;
}

impl StopJoin for StopTimer {
    fn stop_join(mut self) -> Self {
        if let Some(inner) = self.sender.take() {
            inner.send(()).expect("Internal error");
        }

        if let Some(inner) = self.handle.take() {
            inner.join().expect("Internal error");
        }

        self
    }
}

// Guarantees that stop is called on the inner
#[derive(Debug)]
pub(crate) struct StopJoinGuard<T: StopJoin> {
    inner: Option<T>,
}

impl<T: StopJoin> StopJoinGuard<T> {
    /// Since this consumes self and `stop_join` consumes
    /// the inner, the option will never be empty unless
    /// it was created with a None inner.
    ///
    /// Since inner is private we guarantee it's always Some
    /// until this struct is consumed.
    pub(crate) fn stop(mut self) -> T {
        self.inner
            .take()
            .map(StopJoin::stop_join)
            .expect("Internal error: Should never panic, codepath tested")
    }
}

impl<T: StopJoin> Drop for StopJoinGuard<T> {
    fn drop(&mut self) {
        if let Some(inner) = self.inner.take() {
            inner.stop_join();
        }
    }
}

#[cfg(test)]
mod test {
    use super::*;
    use crate::output::util::ReadYourWrite;
    use libcnb_test::assert_contains;
    use std::thread::sleep;

    #[test]
    fn does_stop_does_not_panic() {
        let writer = ReadYourWrite::writer(Vec::new());
        let reader = writer.reader();
        let done = start_timer(&writer.arc_io(), Duration::from_millis(1), " .", ".", ". ");

        let _ = done.stop();

        assert_contains!(String::from_utf8_lossy(&reader.lock().unwrap()), " ... ");
    }

    #[test]
    fn test_drop_stops_timer() {
        let writer = ReadYourWrite::writer(Vec::new());
        let reader = writer.reader();
        let done = start_timer(&writer.arc_io(), Duration::from_millis(1), " .", ".", ". ");

        drop(done);
        sleep(Duration::from_millis(2));

        let before = String::from_utf8_lossy(&reader.lock().unwrap()).to_string();
        sleep(Duration::from_millis(100));
        let after = String::from_utf8_lossy(&reader.lock().unwrap()).to_string();
        assert_eq!(before, after);
    }
}
