use crate::container_port_mapping;
use crate::IntegrationTestContext;
use bollard::container::{LogOutput, RemoveContainerOptions};
use bollard::exec::{CreateExecOptions, StartExecResults};
use std::net::SocketAddr;
use tokio_stream::StreamExt;

pub struct ContainerContext<'a> {
    pub container_name: String,
    pub(crate) integration_test_context: &'a IntegrationTestContext<'a>,
}

impl<'a> ContainerContext<'a> {
    /// # Panics
    #[must_use]
    pub fn address_for_port(&self, port: u16) -> Option<SocketAddr> {
        self.integration_test_context
            .integration_test
            .tokio_runtime
            .block_on(async {
                self.integration_test_context
                    .integration_test
                    .docker
                    .inspect_container(&self.container_name, None)
                    .await
                    .unwrap()
                    .network_settings
                    .and_then(|network_settings| network_settings.ports)
                    .and_then(|ports| {
                        container_port_mapping::parse_port_map(&ports)
                            .unwrap()
                            .get(&port)
                            .copied()
                    })
            })
    }

    /// # Panics
    pub fn shell_exec(&self, command: impl AsRef<str>) -> ContainerExecResult {
        self.integration_test_context
            .integration_test
            .tokio_runtime
            .block_on(async {
                let create_exec_result = self
                    .integration_test_context
                    .integration_test
                    .docker
                    .create_exec(
                        &self.container_name,
                        CreateExecOptions {
                            cmd: Some(vec![
                                "/cnb/lifecycle/launcher",
                                "/bin/sh",
                                "-c",
                                command.as_ref(),
                            ]),
                            attach_stdout: Some(true),
                            ..Default::default()
                        },
                    )
                    .await
                    .unwrap();

                let start_exec_result = self
                    .integration_test_context
                    .integration_test
                    .docker
                    .start_exec(&create_exec_result.id, None)
                    .await
                    .unwrap();

                container_exec_result_from_bollard(start_exec_result).await
            })
    }
}

pub struct ContainerExecResult {
    pub stdout_raw: Vec<u8>,
    pub stderr_raw: Vec<u8>,
    pub stdout: String,
    pub stderr: String,
}

async fn container_exec_result_from_bollard(
    start_exec_result: StartExecResults,
) -> ContainerExecResult {
    let mut container_exec_result = ContainerExecResult {
        stdout_raw: vec![],
        stderr_raw: vec![],
        stdout: "".to_string(),
        stderr: "".to_string(),
    };

    match start_exec_result {
        StartExecResults::Attached { mut output, .. } => {
            while let Some(Ok(log_output)) = output.next().await {
                match log_output {
                    LogOutput::StdErr { message } => container_exec_result
                        .stderr_raw
                        .append(&mut message.to_vec()),

                    LogOutput::StdOut { message } => container_exec_result
                        .stdout_raw
                        .append(&mut message.to_vec()),

                    x => unimplemented!("message unimplemented: {}", x),
                }
            }
        }
        StartExecResults::Detached => {}
    }

    container_exec_result.stdout =
        String::from_utf8_lossy(&container_exec_result.stdout_raw).to_string();

    container_exec_result.stderr =
        String::from_utf8_lossy(&container_exec_result.stderr_raw).to_string();

    container_exec_result
}

impl<'a> Drop for ContainerContext<'a> {
    fn drop(&mut self) {
        // We do not care if container removal succeeded or not. Panicking here would result in
        // SIGILL since this function might be called in a Tokio runtime.
        let _remove_container_result = self
            .integration_test_context
            .integration_test
            .tokio_runtime
            .block_on(
                self.integration_test_context
                    .integration_test
                    .docker
                    .remove_container(
                        &self.container_name,
                        Some(RemoveContainerOptions {
                            force: true,
                            ..Default::default()
                        }),
                    ),
            );
    }
}
