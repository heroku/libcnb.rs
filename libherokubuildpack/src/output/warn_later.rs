use std::cell::RefCell;
use std::fmt::{Debug, Display};
use std::io::Write;
use std::marker::PhantomData;
use std::rc::Rc;
use std::thread::ThreadId;

pub type PhantomUnsync = PhantomData<Rc<()>>;

thread_local!(static WARN_LATER: RefCell<Option<Vec<String>>> = RefCell::new(None));

/// Queue a warning for later
///
/// Build logs can be quite large and people don't always scroll back up to read every line. Delaying
/// a warning and emitting it right before the end of the build can increase the chances the app
/// developer will read it.
///
/// ## Use - Setup a `WarnGuard` in your buildpack
///
/// To ensure warnings are printed, even in the event of errors, you must create a `WarnGuard`
/// in your buildpack that will print any delayed warnings when dropped:
///
/// ```no_run
/// // src/main.rs
/// use libherokubuildpack::output::warn_later::WarnGuard;
///
/// // fn build(&self, context: BuildContext<Self>) -> libcnb::Result<BuildResult, Self::Error> {
///     let warn_later = WarnGuard::new(std::io::stdout());
///     // ...
///
///     // Warnings will be emitted when the warn guard is dropped
///     drop(warn_later);
/// // }
/// ```
///
/// Alternatively you can manually print delayed warnings:
///
/// ```no_run
/// use libherokubuildpack::output::warn_later::WarnGuard;
///
/// // fn build(&self, context: BuildContext<Self>) -> libcnb::Result<BuildResult, Self::Error> {
///     let warn_later = WarnGuard::new(std::io::stdout());
///     // ...
///
///     // Consumes the guard, prints and clears any delayed warnings.
///     warn_later.warn_now();
/// // }
/// ```
///
/// ## Use - Issue a delayed warning
///
/// Once a warn guard is in place you can queue a warning using `section_log::log_warning_later` or `build_log::*`:
///
/// ```
/// use libherokubuildpack::output::warn_later::WarnGuard;
/// use libherokubuildpack::output::build_log::*;
///
/// // src/main.rs
/// let warn_later = WarnGuard::new(std::io::stdout());
///
/// BuildLog::new(std::io::stdout())
///     .buildpack_name("Julius Caesar")
///     .announce()
///     .warn_later("Beware the ides of march");
/// ```
///
/// ```
/// use libherokubuildpack::output::warn_later::WarnGuard;
/// use libherokubuildpack::output::section_log::log_warning_later;
///
/// // src/main.rs
/// let warn_later = WarnGuard::new(std::io::stdout());
///
/// // src/layers/greenday.rs
/// log_warning_later("WARNING: Live without warning");
/// ```

/// Pushes a string to a thread local warning vec for to be emitted later
///
/// # Errors
///
/// If the internal `WARN_LATER` option is `None` this will emit a `WarnLaterError` because
/// the function call might not be visible to the application owner using the buildpack.
///
/// This state can happen if no `WarnGuard` is created in the thread where the delayed warning
/// message is trying to be pushed. It can also happen if multiple `WarnGuard`-s are created in the
/// same thread and one of them "takes" the contents before the others go out of scope.
///
/// For best practice create one and only one `WarnGuard` per thread at a time to avoid this error
/// state.
pub(crate) fn try_push(s: impl AsRef<str>) -> Result<(), WarnLaterError> {
    WARN_LATER.with(|cell| match &mut *cell.borrow_mut() {
        Some(warnings) => {
            warnings.push(s.as_ref().to_string());
            Ok(())
        }
        None => Err(WarnLaterError::MissingGuardForThread(
            std::thread::current().id(),
        )),
    })
}

/// Ensures a warning vec is present and pushes to it
///
/// Should only ever be used within a `WarnGuard`.
///
/// The state where the warnings are taken, but a warn guard is still present
/// can happen when more than one warn guard is created in the same thread
fn force_push(s: impl AsRef<str>) {
    WARN_LATER.with(|cell| {
        let option = &mut *cell.borrow_mut();
        option
            .get_or_insert(Vec::new())
            .push(s.as_ref().to_string());
    });
}

/// Removes all delayed warnings from current thread
///
/// Should only execute from within a `WarnGuard`
fn take() -> Option<Vec<String>> {
    WARN_LATER.with(|cell| cell.replace(None))
}

#[allow(clippy::module_name_repetitions)]
#[derive(Debug)]
pub enum WarnLaterError {
    MissingGuardForThread(ThreadId),
}

impl Display for WarnLaterError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            WarnLaterError::MissingGuardForThread(id) => {
                writeln!(
                    f,
                    "Cannot use warn_later unless a WarnGuard has been created\n and not yet dropped in the current thread: {id:?}",
                )
            }
        }
    }
}

/// Delayed Warning emitter
///
/// To use the delayed warnings feature you'll need to first register a guard.
/// This guard will emit warnings when it goes out of scope or when you manually force it
/// to emit warnings.
///
/// This struct allows delayed warnings to be emitted even in the even there's an error.
///
/// See the `warn_later` module docs for usage instructions.
///
/// The internal design of this features relies on state tied to the current thread.
/// As a result, this struct is not send or sync:
///
/// ```compile_fail
/// // Fails to compile
/// # // Do not remove this test, it is the only thing that asserts this is not sync
/// use libherokubuildpack::output::warn_later::WarnGuard;
///
/// fn is_sync(t: impl Sync) {}
///
/// is_sync(WarnGuard::new(std::io::stdout()))
/// ```
///
/// ```compile_fail
/// // Fails to compile
/// # // Do not remove this test, it is the only thing that asserts this is not send
/// use libherokubuildpack::output::warn_later::WarnGuard;
///
/// fn is_send(t: impl Send) {}
///
/// is_send(WarnGuard::new(std::io::stdout()))
/// ```
///
/// If you are warning in multiple threads you can pass queued warnings from one thread to another.
///
/// ```rust
/// use libherokubuildpack::output::warn_later::{WarnGuard, DelayedWarnings};
///
/// let main_guard = WarnGuard::new(std::io::stdout());
///
/// let (delayed_send, delayed_recv) = std::sync::mpsc::channel::<DelayedWarnings>();
///
/// std::thread::spawn(move || {
///     let sub_guard = WarnGuard::new(std::io::stdout());
///     // ...
///     delayed_send
///         .send(sub_guard.consume_quiet())
///         .unwrap();
/// })
/// .join();
///
/// main_guard
///     .extend_warnings(delayed_recv.recv().unwrap());
/// ```
#[derive(Debug)]
pub struct WarnGuard<W>
where
    W: Write + Debug,
{
    // Private inner to force public construction through `new()` which tracks creation state per thread.
    io: W,
    /// The use of WarnGuard is directly tied to the thread where it was created
    /// This forces the struct to not be send or sync
    ///
    /// To move warn later data between threads, drain quietly, send the data to another
    /// thread, and re-apply those warnings to a WarnGuard in the other thread.
    unsync: PhantomUnsync,
}

impl<W> WarnGuard<W>
where
    W: Write + Debug,
{
    #[must_use]
    #[allow(clippy::new_without_default)]
    pub fn new(io: W) -> Self {
        WARN_LATER.with(|cell| {
            let maybe_warnings = &mut *cell.borrow_mut();
            if let Some(warnings) = maybe_warnings.take() {
                let _ = maybe_warnings.insert(warnings);
                eprintln!("[Buildpack warning]: Multiple `WarnGuard`-s in thread {id:?}, this may cause unexpected delayed warning output", id = std::thread::current().id());
            } else {
                let _ = maybe_warnings.insert(Vec::new());
            }
        });

        Self {
            io,
            unsync: PhantomData,
        }
    }

    /// Use to move warnings from a different thread into this one
    pub fn extend_warnings(&self, warnings: DelayedWarnings) {
        for warning in warnings.inner {
            force_push(warning.clone());
        }
    }

    /// Use to move warnings out of the current thread without emitting to the UI.
    pub fn consume_quiet(self) -> DelayedWarnings {
        DelayedWarnings {
            inner: take().unwrap_or_default(),
        }
    }

    /// Consumes self, prints and drains all existing delayed warnings
    pub fn warn_now(self) {
        drop(self);
    }
}

impl<W> Drop for WarnGuard<W>
where
    W: Write + Debug,
{
    fn drop(&mut self) {
        if let Some(warnings) = take() {
            if !warnings.is_empty() {
                for warning in &warnings {
                    writeln!(&mut self.io).expect("warn guard IO is writeable");
                    write!(&mut self.io, "{warning}").expect("warn guard IO is writeable");
                }
            }
        }
    }
}

/// Holds warnings from a consumed `WarnGuard`
///
/// The intended use of this struct is to pass warnings from one `WarnGuard` to another.
#[derive(Debug)]
pub struct DelayedWarnings {
    // Private inner, must be constructed within a WarnGuard
    inner: Vec<String>,
}

#[cfg(test)]
mod test {
    use super::*;
    use crate::output::util::ReadYourWrite;
    use libcnb_test::assert_contains;

    #[test]
    fn test_warn_guard_registers_itself() {
        // Err when a guard is not yet created
        assert!(try_push("lol").is_err());

        // Don't err when a guard is created
        let _warn_guard = WarnGuard::new(Vec::new());
        try_push("lol").unwrap();
    }

    #[test]
    fn test_logging_a_warning() {
        let writer = ReadYourWrite::writer(Vec::new());
        let reader = writer.reader();
        let warn_guard = WarnGuard::new(writer);
        drop(warn_guard);

        assert_eq!(String::new(), reader.read_lossy().unwrap());

        let writer = ReadYourWrite::writer(Vec::new());
        let reader = writer.reader();
        let warn_guard = WarnGuard::new(writer);
        let message =
            "Possessing knowledge and performing an action are two entirely different processes";

        try_push(message).unwrap();
        drop(warn_guard);

        assert_contains!(reader.read_lossy().unwrap(), message);

        // Assert empty after calling drain
        assert!(take().is_none());
    }

    #[test]
    fn test_delayed_warnings_on_drop() {
        let writer = ReadYourWrite::writer(Vec::new());
        let reader = writer.reader();
        let guard = WarnGuard::new(writer);

        let message = "You don't have to have a reason to be tired. You don't have to earn rest or comfort. You're allowed to just be.";
        try_push(message).unwrap();
        drop(guard);

        assert_contains!(reader.read_lossy().unwrap(), message);
    }

    #[test]
    fn does_not_double_whitespace() {
        let writer = ReadYourWrite::writer(Vec::new());
        let reader = writer.reader();
        let guard = WarnGuard::new(writer);

        let message = "Caution: This test is hot\n";
        try_push(message).unwrap();
        drop(guard);

        let expected = "\nCaution: This test is hot\n".to_string();
        assert_eq!(expected, reader.read_lossy().unwrap());
    }
}
