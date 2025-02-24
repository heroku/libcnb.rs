use crate::docker::{
    DockerExecCommand, DockerLogsCommand, DockerPortCommand, DockerRemoveContainerCommand,
};
use crate::log::LogOutput;
use crate::util::CommandError;
use crate::{ContainerConfig, util};
use std::net::SocketAddr;

/// Context of a launched container.
pub struct ContainerContext {
    /// The randomly generated name of this container.
    pub container_name: String,
    pub(crate) config: ContainerConfig,
}

impl ContainerContext {
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
    /// use libcnb_test::{assert_contains, assert_empty, BuildConfig, ContainerConfig, TestRunner};
    ///
    /// TestRunner::default().build(
    ///     BuildConfig::new("heroku/builder:22", "tests/fixtures/app"),
    ///     |context| {
    ///         // ...
    ///         context.start_container(ContainerConfig::new(), |container| {
    ///             let log_output_until_now = container.logs_now();
    ///             assert_empty!(log_output_until_now.stderr);
    ///             assert_contains!(log_output_until_now.stdout, "Expected output");
    ///         });
    ///     },
    /// );
    /// ```
    ///
    /// # Panics
    ///
    /// Panics if there was an error retrieving the logs from the container.
    #[must_use]
    pub fn logs_now(&self) -> LogOutput {
        util::run_command(DockerLogsCommand::new(&self.container_name))
            .unwrap_or_else(|command_err| panic!("Error fetching container logs:\n\n{command_err}"))
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
    /// use libcnb_test::{assert_contains, assert_empty, BuildConfig, ContainerConfig, TestRunner};
    ///
    /// TestRunner::default().build(
    ///     BuildConfig::new("heroku/builder:22", "tests/fixtures/app"),
    ///     |context| {
    ///         // ...
    ///         context.start_container(ContainerConfig::new(), |container| {
    ///             let all_log_output = container.logs_wait();
    ///             assert_empty!(all_log_output.stderr);
    ///             assert_contains!(all_log_output.stdout, "Expected output");
    ///         });
    ///     },
    /// );
    /// ```
    ///
    /// # Panics
    ///
    /// Panics if there was an error retrieving the logs from the container.
    #[must_use]
    pub fn logs_wait(&self) -> LogOutput {
        let mut docker_logs_command = DockerLogsCommand::new(&self.container_name);
        docker_logs_command.follow(true);
        util::run_command(docker_logs_command)
            .unwrap_or_else(|command_err| panic!("Error fetching container logs:\n\n{command_err}"))
    }

    /// Returns the local address of an exposed container port.
    ///
    /// # Example
    /// ```no_run
    /// use libcnb_test::{BuildConfig, ContainerConfig, TestRunner};
    ///
    /// TestRunner::default().build(
    ///     BuildConfig::new("heroku/builder:22", "tests/fixtures/app"),
    ///     |context| {
    ///         // ...
    ///         context.start_container(
    ///             ContainerConfig::new()
    ///                 .env("PORT", "12345")
    ///                 .expose_port(12345),
    ///             |container| {
    ///                 let address_on_host = container.address_for_port(12345);
    ///                 // ...
    ///             },
    ///         );
    ///     },
    /// );
    /// ```
    ///
    /// # Panics
    ///
    /// Will panic if there was an error obtaining the container port mapping, or the specified port
    /// was not exposed using [`ContainerConfig::expose_port`](crate::ContainerConfig::expose_port).
    #[must_use]
    pub fn address_for_port(&self, port: u16) -> SocketAddr {
        assert!(
            self.config.exposed_ports.contains(&port),
            "Unknown port: Port {port} needs to be exposed first using `ContainerConfig::expose_port`"
        );

        let docker_port_command = DockerPortCommand::new(&self.container_name, port);

        match util::run_command(docker_port_command) {
            Ok(output) => output
                .stdout
                .trim()
                .parse()
                .unwrap_or_else(|error| panic!("Error parsing `docker port` output: {error}")),
            Err(CommandError::NonZeroExitCode { log_output, .. }) => {
                panic!(
                    "Error obtaining container port mapping:\n{}\nThis normally means that the container crashed. Container logs:\n\n{}",
                    log_output.stderr,
                    self.logs_now()
                );
            }
            Err(command_err) => {
                panic!("Error obtaining container port mapping:\n\n{command_err}");
            }
        }
    }

    /// Executes a shell command inside an already running container.
    ///
    /// # Example
    /// ```no_run
    /// use libcnb_test::{assert_contains, BuildConfig, ContainerConfig, TestRunner};
    ///
    /// TestRunner::default().build(
    ///     BuildConfig::new("heroku/builder:22", "tests/fixtures/app"),
    ///     |context| {
    ///         // ...
    ///         context.start_container(ContainerConfig::new(), |container| {
    ///             let log_output = container.shell_exec("ps");
    ///             assert_contains!(log_output.stdout, "gunicorn");
    ///         });
    ///     },
    /// );
    /// ```
    ///
    /// # Panics
    ///
    /// Panics if it was not possible to exec into the container, or if the command
    /// exited with a non-zero exit code.
    pub fn shell_exec(&self, command: impl AsRef<str>) -> LogOutput {
        let docker_exec_command = DockerExecCommand::new(
            &self.container_name,
            [util::CNB_LAUNCHER_BINARY, command.as_ref()],
        );
        util::run_command(docker_exec_command)
            .unwrap_or_else(|command_err| panic!("Error performing docker exec:\n\n{command_err}"))
    }
}

impl Drop for ContainerContext {
    fn drop(&mut self) {
        util::run_command(DockerRemoveContainerCommand::new(&self.container_name)).unwrap_or_else(
            |command_err| panic!("Error removing Docker container:\n\n{command_err}"),
        );
    }
}
