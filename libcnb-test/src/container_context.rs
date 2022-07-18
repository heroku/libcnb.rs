use crate::log::LogOutput;
use crate::{container_port_mapping, util};
use crate::{log, TestContext};
use bollard::container::RemoveContainerOptions;
use bollard::exec::{CreateExecOptions, StartExecResults};
use serde::Serialize;
use std::net::SocketAddr;
use std::time::{SystemTime, UNIX_EPOCH};

/// Context of a launched container.
pub struct ContainerContext<'a> {
    /// The randomly generated name of this container.
    pub container_name: String,
    pub(crate) test_context: &'a TestContext<'a>,
}

impl<'a> ContainerContext<'a> {
    /// Gets the container's log output until the current point in time.
    ///
    /// Note: This method will only return logs until the current point in time. It will not
    /// block until the container stops. Since the output of this method depends on timing, directly
    /// asserting on its contents might result in flaky tests.
    ///
    /// See: [`logs_wait`](Self::logs_wait) for a blocking alternative.
    ///
    /// # Example
    /// ```no_run
    /// use libcnb_test::{assert_contains, assert_empty, ContainerConfig, TestConfig, TestRunner};
    ///
    /// TestRunner::default().run_test(
    ///     TestConfig::new("heroku/builder:22", "test-fixtures/app"),
    ///     |context| {
    ///         // ...
    ///         context.start_container(&ContainerConfig::new(), |container| {
    ///             let log_output_until_now = container.logs_now();
    ///             assert_empty!(log_output_until_now.stderr);
    ///             assert_contains!(log_output_until_now.stdout, "Expected output");
    ///         });
    ///     },
    /// );
    /// ```
    #[must_use]
    pub fn logs_now(&self) -> LogOutput {
        // Bollard forces us to cast to i64
        #[allow(clippy::cast_possible_wrap)]
        self.logs_internal(bollard::container::LogsOptions {
            stdout: true,
            stderr: true,
            since: 0,
            until: SystemTime::now()
                .duration_since(UNIX_EPOCH)
                .expect("System time is before UNIX epoch")
                .as_secs() as i64,
            tail: "all",
            ..bollard::container::LogsOptions::default()
        })
    }

    /// Gets the container's log output until the container stops.
    ///
    /// Note: This method will block until the container stops. If the container never stops by
    /// itself, your test will hang indefinitely. This is common when the container hosts an HTTP
    /// service.
    ///
    /// See: [`logs_now`](Self::logs_now) for a non-blocking alternative.
    ///
    /// # Example
    /// ```no_run
    /// use libcnb_test::{assert_contains, assert_empty, ContainerConfig, TestConfig, TestRunner};
    ///
    /// TestRunner::default().run_test(
    ///     TestConfig::new("heroku/builder:22", "test-fixtures/app"),
    ///     |context| {
    ///         // ...
    ///         context.start_container(&ContainerConfig::new(), |container| {
    ///             let all_log_output = container.logs_wait();
    ///             assert_empty!(all_log_output.stderr);
    ///             assert_contains!(all_log_output.stdout, "Expected output");
    ///         });
    ///     },
    /// );
    /// ```
    #[must_use]
    pub fn logs_wait(&self) -> LogOutput {
        self.logs_internal(bollard::container::LogsOptions {
            follow: true,
            stdout: true,
            stderr: true,
            tail: "all",
            ..bollard::container::LogsOptions::default()
        })
    }

    #[must_use]
    fn logs_internal<T: Into<String> + Serialize>(
        &self,
        logs_options: bollard::container::LogsOptions<T>,
    ) -> LogOutput {
        self.test_context
            .runner
            .tokio_runtime
            .block_on(log::consume_container_log_output(
                self.test_context
                    .runner
                    .docker
                    .logs(&self.container_name, Some(logs_options)),
            ))
            .expect("Could not consume container log output")
    }

    /// Returns the local address of an exposed container port.
    ///
    /// # Example
    /// ```no_run
    /// use libcnb_test::{ContainerConfig, TestConfig, TestRunner};
    ///
    /// TestRunner::default().run_test(
    ///     TestConfig::new("heroku/builder:22", "test-fixtures/app"),
    ///     |context| {
    ///         // ...
    ///         context.start_container(
    ///             ContainerConfig::new().env("PORT", "12345").expose_port(12345),
    ///             |container| {
    ///                 let port_on_host = container.address_for_port(12345).unwrap();
    ///                 // ...
    ///             },
    ///         );
    ///     },
    /// );
    /// ```
    #[must_use]
    pub fn address_for_port(&self, port: u16) -> Option<SocketAddr> {
        self.test_context.runner.tokio_runtime.block_on(async {
            self.test_context
                .runner
                .docker
                .inspect_container(&self.container_name, None)
                .await
                .expect("Could not inspect container")
                .network_settings
                .and_then(|network_settings| network_settings.ports)
                .and_then(|ports| {
                    container_port_mapping::parse_port_map(&ports)
                        .expect("Could not parse container port mapping")
                        .get(&port)
                        .copied()
                })
        })
    }

    /// Executes a shell command inside an already running container.
    ///
    /// # Example
    /// ```no_run
    /// use libcnb_test::{assert_contains, ContainerConfig, TestConfig, TestRunner};
    ///
    /// TestRunner::default().run_test(
    ///     TestConfig::new("heroku/builder:22", "test-fixtures/app"),
    ///     |context| {
    ///         // ...
    ///         context.start_container(&ContainerConfig::new(), |container| {
    ///             let log_output = container.shell_exec("ps");
    ///             assert_contains!(log_output.stdout, "gunicorn");
    ///         });
    ///     },
    /// );
    /// ```
    pub fn shell_exec(&self, command: impl AsRef<str>) -> LogOutput {
        self.test_context.runner.tokio_runtime.block_on(async {
            let create_exec_result = self
                .test_context
                .runner
                .docker
                .create_exec(
                    &self.container_name,
                    CreateExecOptions {
                        cmd: Some(vec![util::CNB_LAUNCHER_BINARY, command.as_ref()]),
                        attach_stdout: Some(true),
                        ..CreateExecOptions::default()
                    },
                )
                .await
                .expect("Could not create container exec instance");

            let start_exec_result = self
                .test_context
                .runner
                .docker
                .start_exec(&create_exec_result.id, None)
                .await
                .expect("Could not start container exec instance");

            match start_exec_result {
                StartExecResults::Attached { output, .. } => {
                    log::consume_container_log_output(output)
                        .await
                        .expect("Could not consume container log output")
                }
                StartExecResults::Detached => LogOutput::default(),
            }
        })
    }
}

impl<'a> Drop for ContainerContext<'a> {
    fn drop(&mut self) {
        // We do not care if container removal succeeded or not. Panicking here would result in
        // SIGILL since this function might be called in a Tokio runtime.
        let _remove_container_result = self.test_context.runner.tokio_runtime.block_on(
            self.test_context.runner.docker.remove_container(
                &self.container_name,
                Some(RemoveContainerOptions {
                    force: true,
                    ..RemoveContainerOptions::default()
                }),
            ),
        );
    }
}
