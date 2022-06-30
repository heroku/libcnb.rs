use tokio_stream::{Stream, StreamExt};

/// Container log output.
#[derive(Debug, Default)]
pub struct LogOutput {
    pub stdout_raw: Vec<u8>,
    pub stderr_raw: Vec<u8>,
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
            let mut acc = LogOutput::default();

            for log_output_chunk in log_output_chunks {
                match log_output_chunk {
                    bollard::container::LogOutput::StdOut { message } => {
                        acc.stdout_raw.append(&mut message.to_vec());
                    }
                    bollard::container::LogOutput::StdErr { message } => {
                        acc.stderr_raw.append(&mut message.to_vec());
                    }
                    unimplemented_message => {
                        unimplemented!("message unimplemented: {unimplemented_message}")
                    }
                }
            }

            acc.stdout = String::from_utf8_lossy(&acc.stdout_raw).to_string();
            acc.stderr = String::from_utf8_lossy(&acc.stderr_raw).to_string();

            acc
        })
}
