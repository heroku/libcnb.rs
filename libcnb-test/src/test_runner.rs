use crate::docker::{DockerRemoveImageCommand, DockerRemoveVolumeCommand};
use crate::pack::PackBuildCommand;
use crate::util::CommandError;
use crate::{app, build, util, BuildConfig, BuildpackReference, PackResult, TestContext};
use std::borrow::Borrow;
use std::env;
use std::path::PathBuf;
use tempfile::tempdir;

/// Runner for libcnb integration tests.
///
/// # Example
/// ```no_run
/// use libcnb_test::{assert_contains, assert_empty, BuildConfig, TestRunner};
///
/// TestRunner::default().build(
///     BuildConfig::new("heroku/builder:22", "tests/fixtures/app"),
///     |context| {
///         assert_empty!(context.pack_stderr);
///         assert_contains!(context.pack_stdout, "Expected build output");
///     },
/// )
/// ```
#[derive(Default)]
pub struct TestRunner {}

impl TestRunner {
    /// Starts a new integration test build.
    ///
    /// This function copies the application to a temporary directory (if necessary), cross-compiles the current
    /// crate, packages it as a buildpack and then invokes [pack](https://buildpacks.io/docs/tools/pack/)
    /// to build a new Docker image with the buildpacks specified by the passed [`BuildConfig`].
    ///
    /// After the passed test function has returned, the Docker image and volumes created by Pack are removed.
    ///
    /// Since this function is supposed to only be used in integration tests, failures are not
    /// signalled via [`Result`] values. Instead, this function panics whenever an unexpected error
    /// occurred to simplify testing code.
    ///
    /// # Example
    /// ```no_run
    /// use libcnb_test::{assert_contains, assert_empty, BuildConfig, TestRunner};
    ///
    /// TestRunner::default().build(
    ///     BuildConfig::new("heroku/builder:22", "tests/fixtures/app"),
    ///     |context| {
    ///         assert_empty!(context.pack_stderr);
    ///         assert_contains!(context.pack_stdout, "Expected build output");
    ///     },
    /// )
    /// ```
    pub fn build<C: Borrow<BuildConfig>, F: FnOnce(TestContext)>(&self, config: C, f: F) {
        let image_name = util::random_docker_identifier();
        let docker_resources = TemporaryDockerResources {
            build_cache_volume_name: format!("{image_name}.build-cache"),
            launch_cache_volume_name: format!("{image_name}.launch-cache"),
            image_name,
        };
        self.build_internal(docker_resources, config, f);
    }

    pub(crate) fn build_internal<C: Borrow<BuildConfig>, F: FnOnce(TestContext)>(
        &self,
        docker_resources: TemporaryDockerResources,
        config: C,
        f: F,
    ) {
        let config = config.borrow();

        let cargo_manifest_dir = env::var("CARGO_MANIFEST_DIR").map_or_else(
            |error| panic!("Error determining Cargo manifest directory: {error}"),
            PathBuf::from,
        );

        let app_dir = {
            let normalized_app_dir_path = if config.app_dir.is_relative() {
                cargo_manifest_dir.join(&config.app_dir)
            } else {
                config.app_dir.clone()
            };

            assert!(
                normalized_app_dir_path.is_dir(),
                "App dir is not a valid directory: {}",
                normalized_app_dir_path.display()
            );

            // Copy the app to a temporary directory if an app_dir_preprocessor is specified and run the
            // preprocessor. Skip app copying if no changes to the app will be made.
            if let Some(app_dir_preprocessor) = &config.app_dir_preprocessor {
                let temporary_app_dir = app::copy_app(&normalized_app_dir_path)
                    .expect("Error copying app fixture to temporary location");

                (app_dir_preprocessor)(temporary_app_dir.as_path().to_owned());

                temporary_app_dir
            } else {
                normalized_app_dir_path.into()
            }
        };

        let buildpacks_target_dir =
            tempdir().expect("Error creating temporary directory for compiled buildpacks");

        let mut pack_command = PackBuildCommand::new(
            &config.builder_name,
            &app_dir,
            &docker_resources.image_name,
            &docker_resources.build_cache_volume_name,
            &docker_resources.launch_cache_volume_name,
        );

        config.env.iter().for_each(|(key, value)| {
            pack_command.env(key, value);
        });

        for buildpack in &config.buildpacks {
            match buildpack {
                BuildpackReference::CurrentCrate => {
                    let crate_buildpack_dir = build::package_crate_buildpack(
                        config.cargo_profile,
                        &config.target_triple,
                        &cargo_manifest_dir,
                        buildpacks_target_dir.path(),
                    )
                    .unwrap_or_else(|error| {
                        panic!("Error packaging current crate as buildpack: {error}")
                    });
                    pack_command.buildpack(crate_buildpack_dir);
                }

                BuildpackReference::WorkspaceBuildpack(buildpack_id) => {
                    let buildpack_dir = build::package_buildpack(
                        buildpack_id,
                        config.cargo_profile,
                        &config.target_triple,
                        &cargo_manifest_dir,
                        buildpacks_target_dir.path(),
                    )
                    .unwrap_or_else(|error| {
                        panic!("Error packaging buildpack '{buildpack_id}': {error}")
                    });
                    pack_command.buildpack(buildpack_dir);
                }

                BuildpackReference::Other(id) => {
                    pack_command.buildpack(id.clone());
                }
            };
        }

        let pack_result = util::run_command(pack_command);

        let output = match (&config.expected_pack_result, pack_result) {
            (PackResult::Success, Ok(output)) => output,
            (PackResult::Failure, Err(CommandError::NonZeroExitCode { log_output, .. })) => {
                log_output
            }
            (PackResult::Failure, Ok(log_output)) => {
                panic!("The pack build was expected to fail, but did not:\n\n{log_output}");
            }
            (_, Err(command_err)) => {
                panic!("Error performing pack build:\n\n{command_err}");
            }
        };

        let test_context = TestContext {
            pack_stdout: output.stdout,
            pack_stderr: output.stderr,
            docker_resources,
            config: config.clone(),
            runner: self,
        };

        f(test_context);
    }
}

#[allow(clippy::struct_field_names)]
pub(crate) struct TemporaryDockerResources {
    pub(crate) build_cache_volume_name: String,
    pub(crate) image_name: String,
    pub(crate) launch_cache_volume_name: String,
}

impl Drop for TemporaryDockerResources {
    fn drop(&mut self) {
        // Ignoring errors here since we don't want to panic inside Drop.
        // We don't emit a warning to stderr since that gets too noisy in some common
        // cases (such as running a test suite when Docker isn't started) where the tests
        // themselves will also report the same error message.
        let _ = util::run_command(DockerRemoveImageCommand::new(&self.image_name));
        let _ = util::run_command(DockerRemoveVolumeCommand::new([
            &self.build_cache_volume_name,
            &self.launch_cache_volume_name,
        ]));
    }
}
