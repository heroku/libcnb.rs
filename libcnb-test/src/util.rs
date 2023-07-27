use crate::LogOutput;
use std::fmt::Display;
use std::io;
use std::iter::repeat_with;
use std::process::Command;

/// Generate a random Docker identifier.
///
/// It is suitable to be used as an image tag or container name.
///
/// See: [Docker Image Specification](https://github.com/moby/moby/blob/master/image/spec/v1.1.md)
pub(crate) fn random_docker_identifier() -> String {
    format!(
        "libcnbtest_{}",
        repeat_with(fastrand::lowercase)
            .take(12)
            .collect::<String>()
    )
}

pub(crate) const CNB_LAUNCHER_BINARY: &str = "launcher";

/// A helper for running an external process using [`Command`].
pub(crate) fn run_command(command: impl Into<Command>) -> Result<LogOutput, CommandError> {
    let mut command = command.into();
    let program = command.get_program().to_string_lossy().to_string();

    command
        .output()
        .map_err(|io_error| {
            if io_error.kind() == std::io::ErrorKind::NotFound {
                CommandError::NotFound {
                    program: program.clone(),
                }
            } else {
                CommandError::Io {
                    io_error,
                    program: program.clone(),
                }
            }
        })
        .and_then(|output| {
            let stdout = String::from_utf8_lossy(&output.stdout).into_owned();
            let stderr = String::from_utf8_lossy(&output.stderr).into_owned();

            if output.status.success() {
                Ok(LogOutput { stdout, stderr })
            } else {
                Err(CommandError::NonZeroExitCode {
                    program,
                    exit_code: output.status.code(),
                    stdout,
                    stderr,
                })
            }
        })
}

/// Errors that can occur when running an external process using [`run_command`].
#[derive(Debug)]
pub(crate) enum CommandError {
    Io {
        io_error: io::Error,
        program: String,
    },
    NotFound {
        program: String,
    },
    NonZeroExitCode {
        exit_code: Option<i32>,
        program: String,
        stdout: String,
        stderr: String,
    },
}

impl Display for CommandError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            CommandError::Io { program, io_error } => {
                write!(f, "Couldn't spawn external `{program}` process: {io_error}")
            }
            CommandError::NotFound { program } => {
                write!(
                    f,
                    "Couldn't find external program `{program}`. Ensure it is installed and on PATH."
                )
            }
            CommandError::NonZeroExitCode {
                program,
                exit_code,
                stdout,
                stderr,
            } => write!(
                f,
                "{program} command failed with exit code {}!\n\n## stderr:\n\n{stderr}\n## stdout:\n\n{stdout}\n",
                exit_code.map_or(String::from("<unknown>"), |exit_code| exit_code.to_string())
            ),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use indoc::indoc;

    #[test]
    fn run_command_succeeded() {
        let mut command = Command::new("bash");
        command.args(["-c", "echo 'some stdout'; echo 'some stderr' >&2; exit 0"]);
        let output = run_command(command).unwrap();

        assert_eq!(output.stdout, "some stdout\n");
        assert_eq!(output.stderr, "some stderr\n");
    }

    #[test]
    fn run_command_nonzero_exit_code() {
        let mut command = Command::new("bash");
        command.args(["-c", "echo 'some stdout'; echo 'some stderr' >&2; exit 1"]);
        let err = run_command(command).unwrap_err();

        assert!(matches!(err, CommandError::NonZeroExitCode { .. }));
        assert_eq!(
            err.to_string(),
            indoc! {"
                bash command failed with exit code 1!
                
                ## stderr:
                
                some stderr
                
                ## stdout:
                
                some stdout
                
            "}
        );
    }

    #[test]
    fn run_command_program_not_found() {
        let err = run_command(Command::new("nonexistent-program")).unwrap_err();
        assert!(matches!(err, CommandError::NotFound { .. }));
        assert_eq!(
            err.to_string(),
            "Couldn't find external program `nonexistent-program`. Ensure it is installed and on PATH."
        );
    }
}
