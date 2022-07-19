use crate::{
    container_port_mapping, util, BuildConfig, ContainerConfig, ContainerContext, LogOutput,
    TestRunner,
};
use bollard::container::{Config, CreateContainerOptions, StartContainerOptions};
use bollard::image::RemoveImageOptions;
use std::borrow::Borrow;

/// Context for a currently executing test.
pub struct TestContext<'a> {
    /// Standard output of `pack`, interpreted as an UTF-8 string.
    pub pack_stdout: String,
    /// Standard error of `pack`, interpreted as an UTF-8 string.
    pub pack_stderr: String,
    /// The configuration used for this integration test.
    pub config: BuildConfig,

    pub(crate) image_name: String,
    pub(crate) runner: &'a TestRunner,
}

impl<'a> TestContext<'a> {
    /// Starts a container using the provided [`ContainerConfig`].
    ///
    /// If you wish to run a shell command and don't need to customise the configuration, use
    /// the convenience function [`TestContext::run_shell_command`] instead.
    ///
    /// # Examples
    /// ```no_run
    /// use libcnb_test::{BuildConfig, ContainerConfig, TestRunner};
    ///
    /// TestRunner::default().build(
    ///     BuildConfig::new("heroku/builder:22", "test-fixtures/app"),
    ///     |context| {
    ///         // Start the container using the default process-type:
    ///         // https://buildpacks.io/docs/app-developer-guide/run-an-app/#default-process-type
    ///         context.start_container(ContainerConfig::new(), |container| {
    ///             // ...
    ///         });
    ///
    ///         // Start the container using the specified process-type:
    ///         // https://buildpacks.io/docs/app-developer-guide/run-an-app/#non-default-process-type
    ///         context.start_container(ContainerConfig::new().entrypoint(["worker"]), |container| {
    ///             // ...
    ///         });
    ///
    ///         // Start the container using the specified process-type and additional arguments:
    ///         // https://buildpacks.io/docs/app-developer-guide/run-an-app/#non-default-process-type-with-additional-arguments
    ///         context.start_container(
    ///             ContainerConfig::new()
    ///                 .entrypoint(["another-process"])
    ///                 .command(["--additional-arg"]),
    ///             |container| {
    ///                 // ...
    ///             },
    ///         );
    ///
    ///         // Start the container using the provided bash script:
    ///         // https://buildpacks.io/docs/app-developer-guide/run-an-app/#user-provided-shell-process-with-bash-script
    ///         // Only use this shell command form if you need to customise the `ContainerConfig`,
    ///         // otherwise use the convenience function `TestContext::run_shell_command` instead.
    ///         context.start_container(
    ///             ContainerConfig::new()
    ///                 .entrypoint(["launcher"])
    ///                 .command(["for i in {1..3}; do echo \"${i}\"; done"]),
    ///             |container| {
    ///                 // ...
    ///             },
    ///         );
    ///     },
    /// );
    /// ```
    pub fn start_container<C: Borrow<ContainerConfig>, F: FnOnce(ContainerContext)>(
        &self,
        config: C,
        f: F,
    ) {
        let config = config.borrow();
        let container_name = util::random_docker_identifier();

        self.runner.tokio_runtime.block_on(async {
            self.runner
                .docker
                .create_container(
                    Some(CreateContainerOptions {
                        name: container_name.clone(),
                    }),
                    Config {
                        image: Some(self.image_name.clone()),
                        env: Some(config.env.iter().map(|(k, v)| format!("{k}={v}")).collect()),
                        entrypoint: config.entrypoint.clone(),
                        cmd: config.command.clone(),
                        ..container_port_mapping::port_mapped_container_config(
                            &config.exposed_ports,
                        )
                    },
                )
                .await
                .expect("Could not create container");

            self.runner
                .docker
                .start_container(&container_name, None::<StartContainerOptions<String>>)
                .await
                .expect("Could not start container");
        });

        f(ContainerContext {
            container_name,
            test_context: self,
        });
    }

    /// Run the provided shell command.
    ///
    /// The CNB launcher will run the provided command using `bash`.
    ///
    /// Note: This method will block until the container stops.
    ///
    /// # Example
    /// ```no_run
    /// use libcnb_test::{BuildConfig, ContainerConfig, TestRunner};
    ///
    /// TestRunner::default().build(
    ///     BuildConfig::new("heroku/builder:22", "test-fixtures/app"),
    ///     |context| {
    ///         // ...
    ///         let log_output = context.run_shell_command("for i in {1..3}; do echo \"${i}\"; done");
    ///         assert_eq!(log_output.stdout, "1\n2\n3\n");
    ///     },
    /// );
    /// ```
    ///
    /// This is a convenience function for running shell commands inside the image, and is equivalent to:
    /// ```no_run
    /// use libcnb_test::{BuildConfig, ContainerConfig, TestRunner};
    ///
    /// TestRunner::default().build(
    ///     BuildConfig::new("heroku/builder:22", "test-fixtures/app"),
    ///     |context| {
    ///         // ...
    ///         context.start_container(
    ///             ContainerConfig::new()
    ///                 .entrypoint(["launcher"])
    ///                 .command(["for i in {1..3}; do echo \"${i}\"; done"]),
    ///             |container| {
    ///                 let log_output = container.logs_wait();
    ///                 // ...
    ///             },
    ///         );
    ///     },
    /// );
    /// ```
    pub fn run_shell_command(&self, command: impl Into<String>) -> LogOutput {
        let mut log_output = LogOutput::default();
        self.start_container(
            ContainerConfig::new()
                .entrypoint(vec![util::CNB_LAUNCHER_BINARY])
                .command(&[command.into()]),
            |context| {
                log_output = context.logs_wait();
            },
        );
        log_output
    }

    /// Starts a subsequent integration test build.
    ///
    /// This function behaves exactly like [`TestRunner::build`], but it will reuse the OCI image
    /// from the previous test, causing the CNB lifecycle to restore any cached layers. It will use the
    /// same [`TestRunner`] as the previous test run.
    ///
    /// This function allows testing of subsequent builds, including caching logic and buildpack
    /// behaviour when build environment variables change, stacks are upgraded and more.
    ///
    /// Note that this function will consume the current context. This is because the image will
    /// be changed by the subsequent test, invalidating the context. Running a subsequent test must
    /// therefore be the last operation. You can nest subsequent runs if required.
    ///
    /// # Example
    /// ```no_run
    /// use libcnb_test::{assert_contains, BuildConfig, TestRunner};
    ///
    /// TestRunner::default().build(
    ///     BuildConfig::new("heroku/builder:22", "test-fixtures/app"),
    ///     |context| {
    ///         assert_contains!(context.pack_stdout, "---> Installing gems");
    ///
    ///         let config = context.config.clone();
    ///         context.rebuild(config, |context| {
    ///             assert_contains!(context.pack_stdout, "---> Using cached gems");
    ///         });
    ///     },
    /// );
    /// ```
    pub fn rebuild<C: Borrow<BuildConfig>, F: FnOnce(TestContext)>(self, config: C, f: F) {
        self.runner
            .build_internal(self.image_name.clone(), config, f);
    }
}

impl<'a> Drop for TestContext<'a> {
    fn drop(&mut self) {
        // We do not care if image removal succeeded or not. Panicking here would result in
        // SIGILL since this function might be called in a Tokio runtime.
        let _image_delete_result =
            self.runner
                .tokio_runtime
                .block_on(self.runner.docker.remove_image(
                    &self.image_name,
                    Some(RemoveImageOptions {
                        force: true,
                        ..RemoveImageOptions::default()
                    }),
                    None,
                ));
    }
}
