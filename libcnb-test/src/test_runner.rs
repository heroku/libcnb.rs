use crate::pack::PackBuildCommand;
use crate::util::CommandError;
use crate::{
    app, build, util, BuildConfig, BuildpackReference, LogOutput, PackResult, TestContext,
};
use std::borrow::Borrow;
use std::env;
use std::path::PathBuf;

/// Runner for libcnb integration tests.
///
/// # Example
/// ```no_run
/// use libcnb_test::{assert_contains, assert_empty, BuildConfig, TestRunner};
///
/// TestRunner::default().build(
///     BuildConfig::new("heroku/builder:22", "test-fixtures/app"),
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
    /// Since this function is supposed to only be used in integration tests, failures are not
    /// signalled via [`Result`](Result) values. Instead, this function panics whenever an unexpected error
    /// occurred to simplify testing code.
    ///
    /// # Example
    /// ```no_run
    /// use libcnb_test::{assert_contains, assert_empty, BuildConfig, TestRunner};
    ///
    /// TestRunner::default().build(
    ///     BuildConfig::new("heroku/builder:22", "test-fixtures/app"),
    ///     |context| {
    ///         assert_empty!(context.pack_stderr);
    ///         assert_contains!(context.pack_stdout, "Expected build output");
    ///     },
    /// )
    /// ```
    pub fn build<C: Borrow<BuildConfig>, F: FnOnce(TestContext)>(&self, config: C, f: F) {
        self.build_internal(util::random_docker_identifier(), config, f);
    }

    pub(crate) fn build_internal<C: Borrow<BuildConfig>, F: FnOnce(TestContext)>(
        &self,
        image_name: String,
        config: C,
        f: F,
    ) {
        let config = config.borrow();

        let app_dir = {
            let normalized_app_dir_path = if config.app_dir.is_relative() {
                env::var("CARGO_MANIFEST_DIR")
                    .map(PathBuf::from)
                    .expect("Could not determine Cargo manifest directory")
                    .join(&config.app_dir)
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
                    .expect("Could not copy app to temporary location");

                (app_dir_preprocessor)(temporary_app_dir.as_path().to_owned());

                temporary_app_dir
            } else {
                normalized_app_dir_path.into()
            }
        };

        let temp_crate_buildpack_dir =
            config
                .buildpacks
                .contains(&BuildpackReference::Crate)
                .then(|| {
                    build::package_crate_buildpack(config.cargo_profile, &config.target_triple)
                        .expect("Could not package current crate as buildpack")
                });

        let mut pack_command = PackBuildCommand::new(&config.builder_name, &app_dir, &image_name);

        config.env.iter().for_each(|(key, value)| {
            pack_command.env(key, value);
        });

        for buildpack in &config.buildpacks {
            match buildpack {
                BuildpackReference::Crate => {
                    pack_command.buildpack(temp_crate_buildpack_dir.as_ref()
                        .expect("Test references crate buildpack, but crate wasn't packaged as a buildpack. This is an internal libcnb-test error, please report any occurrences."))
                }
                BuildpackReference::Other(id) => pack_command.buildpack(id.clone()),
            };
        }

        let output = match (
            &config.expected_pack_result,
            util::run_command(pack_command),
        ) {
            (PackResult::Success, Ok(output)) => output,
            (PackResult::Failure, Err(CommandError::NonZeroExitCode { stdout, stderr, .. })) => {
                LogOutput { stdout, stderr }
            }
            (PackResult::Failure, Ok(LogOutput { stdout, stderr })) => {
                panic!("The pack build was expected to fail, but did not:\n\n## stderr:\n\n{stderr}\n## stdout:\n\n{stdout}\n");
            }
            (_, Err(command_err)) => {
                panic!("Error performing pack build:\n\n{command_err}");
            }
        };

        let test_context = TestContext {
            pack_stdout: output.stdout,
            pack_stderr: output.stderr,
            image_name,
            config: config.clone(),
            runner: self,
        };

        f(test_context);
    }
}
