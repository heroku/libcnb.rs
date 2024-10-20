use crate::docker::DockerRunCommand;
use crate::pack::PackSbomDownloadCommand;
use crate::{
    util, BuildConfig, ContainerConfig, ContainerContext, LogOutput, TemporaryDockerResources,
    TestRunner,
};
use libcnb_data::buildpack::BuildpackId;
use libcnb_data::layer::LayerName;
use libcnb_data::sbom::SbomFormat;
use std::borrow::Borrow;
use std::path::PathBuf;
use tempfile::tempdir;

/// Context for a currently executing test.
pub struct TestContext<'a> {
    /// Standard output of `pack`, interpreted as an UTF-8 string.
    pub pack_stdout: String,
    /// Standard error of `pack`, interpreted as an UTF-8 string.
    pub pack_stderr: String,
    /// The configuration used for this integration test.
    pub config: BuildConfig,

    pub(crate) docker_resources: TemporaryDockerResources,
    pub(crate) runner: &'a TestRunner,
}

impl<'a> TestContext<'a> {
    /// Starts a detached container using the provided [`ContainerConfig`].
    ///
    /// After the passed function has returned, the Docker container is removed.
    ///
    /// If you wish to run a shell command and don't need to customise the configuration, use
    /// the convenience function [`TestContext::run_shell_command`] instead.
    ///
    /// # Examples
    /// ```no_run
    /// use libcnb_test::{BuildConfig, ContainerConfig, TestRunner};
    ///
    /// TestRunner::default().build(
    ///     BuildConfig::new("heroku/builder:22", "tests/fixtures/app"),
    ///     |context| {
    ///         // Start the container using the default process-type:
    ///         // https://buildpacks.io/docs/app-developer-guide/run-an-app/#default-process-type
    ///         context.start_container(ContainerConfig::new(), |container| {
    ///             // ...
    ///         });
    ///
    ///         // Start the container using the specified process-type:
    ///         // https://buildpacks.io/docs/app-developer-guide/run-an-app/#non-default-process-type
    ///         context.start_container(ContainerConfig::new().entrypoint("worker"), |container| {
    ///             // ...
    ///         });
    ///
    ///         // Start the container using the specified process-type and additional arguments:
    ///         // https://buildpacks.io/docs/app-developer-guide/run-an-app/#non-default-process-type-with-additional-arguments
    ///         context.start_container(
    ///             ContainerConfig::new()
    ///                 .entrypoint("another-process")
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
    ///                 .entrypoint("launcher")
    ///                 .command(["for i in {1..3}; do echo \"${i}\"; done"]),
    ///             |container| {
    ///                 // ...
    ///             },
    ///         );
    ///     },
    /// );
    /// ```
    ///
    /// # Panics
    ///
    /// Panics if there was an error starting the container, such as when the specified entrypoint/command can't be found.
    ///
    /// Note: Does not panic if the container exits after starting (including if it crashes and exits non-zero).
    pub fn start_container<C: Borrow<ContainerConfig>, F: FnOnce(ContainerContext)>(
        &self,
        config: C,
        f: F,
    ) {
        let config = config.borrow();
        let container_name = util::random_docker_identifier();

        let mut docker_run_command =
            DockerRunCommand::new(&self.docker_resources.image_name, &container_name);
        docker_run_command.detach(true);
        docker_run_command.platform(self.determine_container_platform());

        if let Some(entrypoint) = &config.entrypoint {
            docker_run_command.entrypoint(entrypoint);
        }

        if let Some(command) = &config.command {
            docker_run_command.command(command);
        }

        config.env.iter().for_each(|(key, value)| {
            docker_run_command.env(key, value);
        });

        config.exposed_ports.iter().for_each(|port| {
            docker_run_command.expose_port(*port);
        });

        if let Some(volume) = &config.volumes {
            docker_run_command.volumes(volume);
        }

        // We create the ContainerContext early to ensure the cleanup in ContainerContext::drop
        // is still performed even if the Docker command panics.
        let container_context = ContainerContext {
            container_name,
            config: config.clone(),
        };

        util::run_command(docker_run_command)
            .unwrap_or_else(|command_err| panic!("Error starting container:\n\n{command_err}"));

        f(container_context);
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
    ///     BuildConfig::new("heroku/builder:22", "tests/fixtures/app"),
    ///     |context| {
    ///         // ...
    ///         let command_output =
    ///             context.run_shell_command("for i in {1..3}; do echo \"${i}\"; done");
    ///         assert_eq!(command_output.stdout, "1\n2\n3\n");
    ///     },
    /// );
    /// ```
    ///
    /// This is a convenience function for running shell commands inside the image, that is roughly equivalent to:
    /// ```no_run
    /// use libcnb_test::{BuildConfig, ContainerConfig, TestRunner};
    ///
    /// TestRunner::default().build(
    ///     BuildConfig::new("heroku/builder:22", "tests/fixtures/app"),
    ///     |context| {
    ///         // ...
    ///         context.start_container(
    ///             ContainerConfig::new()
    ///                 .entrypoint("launcher")
    ///                 .command(["for i in {1..3}; do echo \"${i}\"; done"]),
    ///             |container| {
    ///                 let log_output = container.logs_wait();
    ///                 // ...
    ///             },
    ///         );
    ///     },
    /// );
    /// ```
    ///
    /// However, in addition to requiring less boilerplate, `run_shell_command` is also able
    /// to validate the exit status of the container, so should be used instead of `start_container`
    /// where possible.
    ///
    /// # Panics
    ///
    /// Panics if there was an error starting the container, or the command exited with a non-zero
    /// exit code.
    pub fn run_shell_command(&self, command: impl Into<String>) -> LogOutput {
        let mut docker_run_command = DockerRunCommand::new(
            &self.docker_resources.image_name,
            util::random_docker_identifier(),
        );
        docker_run_command
            .remove(true)
            .platform(self.determine_container_platform())
            .entrypoint(util::CNB_LAUNCHER_BINARY)
            .command([command.into()]);

        util::run_command(docker_run_command)
            .unwrap_or_else(|command_err| panic!("Error running container:\n\n{command_err}"))
    }

    // We set an explicit platform when starting containers to prevent the Docker CLI's
    // "no specific platform was requested" warning from cluttering the captured logs.
    fn determine_container_platform(&self) -> &str {
        match self.config.target_triple.as_str() {
            "aarch64-unknown-linux-musl" => "linux/arm64",
            "x86_64-unknown-linux-musl" => "linux/amd64",
            _ => unimplemented!(
                "Unable to determine container platform from target triple '{}'. Please file a GitHub issue.",
                self.config.target_triple
            ),
        }
    }

    /// Downloads SBOM files from the built image into a temporary directory.
    ///
    /// References to the downloaded files are passed into the given function and will be cleaned-up
    /// after the function exits.
    ///
    /// # Example
    /// ```no_run
    /// use libcnb_data::buildpack_id;
    /// use libcnb_data::sbom::SbomFormat;
    /// use libcnb_test::{BuildConfig, ContainerConfig, SbomType, TestRunner};
    ///
    /// TestRunner::default().build(
    ///     BuildConfig::new("heroku/builder:22", "tests/fixtures/app"),
    ///     |context| {
    ///         context.download_sbom_files(|sbom_files| {
    ///             assert!(sbom_files
    ///                 .path_for(
    ///                     buildpack_id!("heroku/jvm"),
    ///                     SbomType::Launch,
    ///                     SbomFormat::SyftJson
    ///                 )
    ///                 .exists());
    ///         });
    ///     },
    /// );
    /// ```
    ///
    /// # Panics
    ///
    /// Panics if there was an error creating the temporary directory used to store the
    /// SBOM files, or if the Pack CLI command used to download the SBOM files failed.
    pub fn download_sbom_files<R, F: Fn(SbomFiles) -> R>(&self, f: F) -> R {
        let temp_dir = tempdir().expect("Couldn't create temporary directory for SBOM files");

        let mut command = PackSbomDownloadCommand::new(&self.docker_resources.image_name);
        command.output_dir(temp_dir.path());

        util::run_command(command)
            .unwrap_or_else(|command_err| panic!("Error downloading SBOM files:\n\n{command_err}"));

        f(SbomFiles {
            sbom_files_directory: temp_dir.path().into(),
        })
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
    ///     BuildConfig::new("heroku/builder:22", "tests/fixtures/app"),
    ///     |context| {
    ///         assert_contains!(context.pack_stdout, "---> Installing dependencies");
    ///
    ///         let config = context.config.clone();
    ///         context.rebuild(config, |context| {
    ///             assert_contains!(context.pack_stdout, "---> Using cached dependencies");
    ///         });
    ///     },
    /// );
    /// ```
    pub fn rebuild<C: Borrow<BuildConfig>, F: FnOnce(TestContext)>(self, config: C, f: F) {
        self.runner.build_internal(self.docker_resources, config, f);
    }
}

/// Downloaded SBOM files.
pub struct SbomFiles {
    sbom_files_directory: PathBuf,
}

/// The type of SBOM.
///
/// Not to be confused with [`libcnb_data::sbom::SbomFormat`].
pub enum SbomType {
    /// Launch SBOM
    Launch,
    /// Layer SBOM
    Layer(LayerName),
}

impl SbomFiles {
    /// Returns the path of a specific downloaded SBOM file.
    pub fn path_for<I: Borrow<BuildpackId>, T: Borrow<SbomType>, F: Borrow<SbomFormat>>(
        &self,
        buildpack_id: I,
        sbom_type: T,
        format: F,
    ) -> PathBuf {
        self.sbom_files_directory
            .join("layers")
            .join("sbom")
            .join("launch")
            .join(buildpack_id.borrow().replace('/', "_"))
            .join(match sbom_type.borrow() {
                SbomType::Layer(layer_name) => layer_name.to_string(),
                SbomType::Launch => String::new(),
            })
            .join(match format.borrow() {
                SbomFormat::CycloneDxJson => "sbom.cdx.json",
                SbomFormat::SpdxJson => "sbom.spdx.json",
                SbomFormat::SyftJson => "sbom.syft.json",
            })
    }
}
