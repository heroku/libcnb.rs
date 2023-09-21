use crate::write::tee;
use crossbeam_utils::thread::ScopedJoinHandle;
use std::io::Write;
use std::{io, process, thread};
use std::{mem, panic};

/// Extension trait for [`process::Command`] that adds functions for use within buildpacks.
pub trait CommandExt {
    /// Spawns the command process and sends the output of stdout and stderr to the given writers.
    ///
    /// This allows for additional flexibility when dealing with these output streams compared to
    /// functionality that the stock [`process::Command`] provides. See the [`write`](crate::write)
    /// module for [`std::io::Write`] implementations designed for common buildpack tasks.
    ///
    /// This function will redirect the output unbuffered and in parallel for both streams. This
    /// means that it can be used to output data from these streams while the command is running,
    /// providing a live view into the process' output. This function will block until both streams
    /// have been closed.
    ///
    /// # Example:
    /// ```no_run
    /// use libherokubuildpack::command::CommandExt;
    /// use libherokubuildpack::write::tee;
    /// use std::fs;
    /// use std::process::Command;
    ///
    /// let logfile = fs::File::open("log.txt").unwrap();
    /// let exit_status = Command::new("date")
    ///     .spawn_and_write_streams(tee(std::io::stdout(), &logfile), std::io::stderr())
    ///     .and_then(|mut child| child.wait())
    ///     .unwrap();
    /// ```
    fn spawn_and_write_streams<OW: Write + Send, EW: Write + Send>(
        &mut self,
        stdout_write: OW,
        stderr_write: EW,
    ) -> io::Result<process::Child>;

    /// Spawns the command process and sends the output of stdout and stderr to the given writers.
    ///
    /// In addition to what [`spawn_and_write_streams`](Self::spawn_and_write_streams) does, this
    /// function captures stdout and stderr as `Vec<u8>` and returns them after waiting for the
    /// process to finish. This function is meant as a drop-in replacement for existing
    /// `Command:output` calls.
    ///
    /// # Example:
    /// ```no_run
    /// use libherokubuildpack::command::CommandExt;
    /// use libherokubuildpack::write::tee;
    /// use std::fs;
    /// use std::process::Command;
    ///
    /// let logfile = fs::File::open("log.txt").unwrap();
    /// let output = Command::new("date")
    ///     .output_and_write_streams(tee(std::io::stdout(), &logfile), std::io::stderr())
    ///     .unwrap();
    ///
    /// // Return value can be used as with Command::output, but the streams will also be written to
    /// // the given writers.
    /// println!(
    ///     "Process exited with {}, stdout: {:?}, stderr: {:?}",
    ///     output.status, output.stdout, output.stderr
    /// );
    /// ```
    fn output_and_write_streams<OW: Write + Send, EW: Write + Send>(
        &mut self,
        stdout_write: OW,
        stderr_write: EW,
    ) -> io::Result<process::Output>;
}

impl CommandExt for process::Command {
    fn spawn_and_write_streams<OW: Write + Send, EW: Write + Send>(
        &mut self,
        stdout_write: OW,
        stderr_write: EW,
    ) -> io::Result<process::Child> {
        self.stdout(process::Stdio::piped())
            .stderr(process::Stdio::piped())
            .spawn()
            .and_then(|child| write_child_process_output(child, stdout_write, stderr_write))
    }

    fn output_and_write_streams<OW: Write + Send, EW: Write + Send>(
        &mut self,
        stdout_write: OW,
        stderr_write: EW,
    ) -> io::Result<process::Output> {
        let mut stdout_buffer = Vec::new();
        let mut stderr_buffer = Vec::new();

        self.spawn_and_write_streams(
            tee(&mut stdout_buffer, stdout_write),
            tee(&mut stderr_buffer, stderr_write),
        )
        .and_then(|mut child| child.wait())
        .map(|status| process::Output {
            status,
            stdout: stdout_buffer,
            stderr: stderr_buffer,
        })
    }
}

fn write_child_process_output<OW: Write + Send, EW: Write + Send>(
    mut child: process::Child,
    mut stdout_writer: OW,
    mut stderr_writer: EW,
) -> io::Result<process::Child> {
    // Copying the data to the writers happens in separate threads for stdout and stderr to ensure
    // they're processed in parallel. Example: imagine the caller uses io::stdout() and io::stderr()
    // as the writers so that the user can follow along with the command's output. If we copy stdout
    // first and then stderr afterwards, interleaved stdout and stderr messages will no longer be
    // interleaved (stderr output is always printed after stdout has been closed).
    //
    // The rust compiler currently cannot figure out how long a thread will run (doesn't take the
    // almost immediate join calls into account) and therefore requires that data used in a thread
    // lives forever. To avoid requiring 'static lifetimes for the writers, we use crossbeam's
    // scoped threads here. This enables writers that write, for example, to a mutable buffer.
    unwind_panic(crossbeam_utils::thread::scope(|scope| {
        let stdout_copy_thread = mem::take(&mut child.stdout)
            .map(|mut stdout| scope.spawn(move |_| std::io::copy(&mut stdout, &mut stdout_writer)));

        let stderr_copy_thread = mem::take(&mut child.stderr)
            .map(|mut stderr| scope.spawn(move |_| std::io::copy(&mut stderr, &mut stderr_writer)));

        let stdout_copy_result = stdout_copy_thread.map_or_else(|| Ok(0), join_and_unwind_panic);
        let stderr_copy_result = stderr_copy_thread.map_or_else(|| Ok(0), join_and_unwind_panic);

        // Return the first error from either Result, or the child process value
        stdout_copy_result.and(stderr_copy_result).map(|_| child)
    }))
}

fn join_and_unwind_panic<T>(h: ScopedJoinHandle<T>) -> T {
    unwind_panic(h.join())
}

fn unwind_panic<T>(t: thread::Result<T>) -> T {
    match t {
        Ok(value) => value,
        Err(err) => panic::resume_unwind(err),
    }
}

#[cfg(test)]
mod test {
    use crate::command::CommandExt;
    use std::process::Command;

    #[test]
    #[cfg(unix)]
    fn test_spawn_and_write_streams() {
        let mut stdout_buf = Vec::new();
        let mut stderr_buf = Vec::new();

        Command::new("echo")
            .args(["-n", "Hello World!"])
            .spawn_and_write_streams(&mut stdout_buf, &mut stderr_buf)
            .and_then(|mut child| child.wait())
            .unwrap();

        assert_eq!(stdout_buf, "Hello World!".as_bytes());
        assert_eq!(stderr_buf, Vec::<u8>::new());
    }

    #[test]
    #[cfg(unix)]
    fn test_output_and_write_streams() {
        let mut stdout_buf = Vec::new();
        let mut stderr_buf = Vec::new();

        let output = Command::new("echo")
            .args(["-n", "Hello World!"])
            .output_and_write_streams(&mut stdout_buf, &mut stderr_buf)
            .unwrap();

        assert_eq!(stdout_buf, "Hello World!".as_bytes());
        assert_eq!(stderr_buf, Vec::<u8>::new());

        assert_eq!(output.status.code(), Some(0));
        assert_eq!(output.stdout, "Hello World!".as_bytes());
        assert_eq!(output.stderr, Vec::<u8>::new());
    }
}
