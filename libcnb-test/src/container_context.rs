use crate::IntegrationTestContext;
use crate::{container_port_mapping, util};
use bollard::container::{
    Config, CreateContainerOptions, LogOutput, RemoveContainerOptions, StartContainerOptions,
};
use bollard::exec::{CreateExecOptions, StartExecResults};
use std::collections::HashMap;
use std::net::SocketAddr;
use tokio_stream::StreamExt;

pub struct PrepareContainerContext<'a> {
    integration_test_context: &'a IntegrationTestContext<'a>,
    exposed_ports: Vec<u16>,
    env: HashMap<String, String>,
}

impl<'a> PrepareContainerContext<'a> {
    pub(crate) fn new(integration_test_context: &'a IntegrationTestContext) -> Self {
        Self {
            integration_test_context,
            exposed_ports: Vec::new(),
            env: HashMap::new(),
        }
    }

    /// Exposes a given port of the container to the host machine.
    ///
    /// The given `exposed_port` is mapped to random ports on the host machine. Use
    /// [`ContainerContext::address_for_port`] to obtain the local port for a mapped port.
    pub fn expose_port(&mut self, port: u16) -> &mut Self {
        self.exposed_ports.push(port);
        self
    }

    /// Inserts or updates an environment variable mapping for the container.
    ///
    /// # Example
    /// ```no_run
    /// use libcnb_test::IntegrationTest;
    ///
    /// IntegrationTest::new("heroku/buildpacks:20", "test-fixtures/app").run_test(|context| {
    ///     context
    ///         .prepare_container()
    ///         .env("FOO", "FOO_VALUE")
    ///         .start(|container| {
    ///             // ...
    ///         })
    /// });
    /// ```
    pub fn env(&mut self, key: impl Into<String>, value: impl Into<String>) -> &mut Self {
        self.env.insert(key.into(), value.into());
        self
    }

    /// Adds or updates multiple environment variable mappings for the container.
    ///
    /// # Example
    /// ```no_run
    /// use libcnb_test::IntegrationTest;
    ///
    /// IntegrationTest::new("heroku/buildpacks:20", "test-fixtures/app").run_test(|context| {
    ///     context
    ///         .prepare_container()
    ///         .envs(vec![("FOO", "FOO_VALUE"), ("BAR", "BAR_VALUE")])
    ///         .start(|container| {
    ///             // ...
    ///         })
    /// });
    /// ```
    pub fn envs<K: Into<String>, V: Into<String>, I: IntoIterator<Item = (K, V)>>(
        &mut self,
        envs: I,
    ) -> &mut Self {
        envs.into_iter().for_each(|(key, value)| {
            self.env(key.into(), value.into());
        });

        self
    }

    /// Creates and starts the container configured by this context.
    ///
    /// # Panics
    /// - When the container could not be created
    /// - When the container could not be started
    pub fn start<F: FnOnce(ContainerContext)>(&self, f: F) {
        let container_name = util::random_docker_identifier();

        self.integration_test_context
            .integration_test
            .tokio_runtime
            .block_on(async {
                self.integration_test_context
                    .integration_test
                    .docker
                    .create_container(
                        Some(CreateContainerOptions {
                            name: container_name.clone(),
                        }),
                        Config {
                            image: Some(self.integration_test_context.image_name.clone()),
                            env: Some(self.env.iter().map(|(k, v)| format!("{k}={v}")).collect()),
                            ..container_port_mapping::port_mapped_container_config(
                                &self.exposed_ports,
                            )
                        },
                    )
                    .await
                    .expect("Could not create container");

                self.integration_test_context
                    .integration_test
                    .docker
                    .start_container(&container_name, None::<StartContainerOptions<String>>)
                    .await
                    .expect("Could not start container");
            });

        f(ContainerContext {
            container_name,
            integration_test_context: self.integration_test_context,
        });
    }
}

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
                            ..CreateExecOptions::default()
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

                    x => unimplemented!("message unimplemented: {x}"),
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
                            ..RemoveContainerOptions::default()
                        }),
                    ),
            );
    }
}
