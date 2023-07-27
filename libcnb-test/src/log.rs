use tokio_stream::{Stream, StreamExt};

/// Log output from a command.
#[derive(Debug, Default)]
pub struct LogOutput {
    pub stdout: String,
    pub stderr: String,
}

pub(crate) async fn consume_container_log_output<
    E,
    S: Stream<Item = Result<bollard::container::LogOutput, E>> + Unpin,
>(
    stream: S,
) -> Result<LogOutput, E> {
    stream
        .collect::<Result<Vec<bollard::container::LogOutput>, E>>()
        .await
        .map(|log_output_chunks| {
            let mut stdout_raw = Vec::new();
            let mut stderr_raw = Vec::new();

            for log_output_chunk in log_output_chunks {
                match log_output_chunk {
                    bollard::container::LogOutput::StdOut { message } => {
                        stdout_raw.append(&mut message.to_vec());
                    }
                    bollard::container::LogOutput::StdErr { message } => {
                        stderr_raw.append(&mut message.to_vec());
                    }
                    unimplemented_message => {
                        unimplemented!("message unimplemented: {unimplemented_message}")
                    }
                }
            }

            LogOutput {
                stdout: String::from_utf8_lossy(&stdout_raw).to_string(),
                stderr: String::from_utf8_lossy(&stderr_raw).to_string(),
            }
        })
}
