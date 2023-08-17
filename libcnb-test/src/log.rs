use std::fmt::Display;

/// Log output from a command.
#[derive(Debug, Default)]
pub struct LogOutput {
    pub stdout: String,
    pub stderr: String,
}

impl Display for LogOutput {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        let LogOutput { stdout, stderr } = self;
        write!(f, "## stderr:\n\n{stderr}\n## stdout:\n\n{stdout}\n")
    }
}
