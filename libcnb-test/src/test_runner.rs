use crate::build::CargoEnvAddition;
use crate::docker::{DockerRemoveImageCommand, DockerRemoveVolumeCommand};
use crate::pack::{PackBuildCommand, VolumeMount};
use crate::util::CommandError;
use crate::{BuildConfig, BuildpackReference, PackResult, TestContext, app, build, util};
use libcnb_package::find_cargo_workspace_root_dir;
use std::borrow::Borrow;
use std::env;
use std::ffi::OsString;
use std::path::{Path, PathBuf};
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

        let instrumentation_enabled = config.instrumentation_enabled
            || instrumentation_enabled_via_env().unwrap_or_else(|error| panic!("{error}"));

        let instrumentation_setup = if instrumentation_enabled {
            Some(configure_instrumentation(&cargo_manifest_dir))
        } else {
            None
        };

        if let Some(ref setup) = instrumentation_setup {
            pack_command.volume(setup.volume.clone());
            pack_command.env(&setup.pack_env.0, &setup.pack_env.1);
        }

        let cargo_env_additions =
            instrumentation_setup.map_or_else(Vec::new, |setup| setup.cargo_env_additions);

        for reference in create_buildpack_references(
            config,
            &cargo_manifest_dir,
            buildpacks_target_dir.path(),
            &cargo_env_additions,
        ) {
            pack_command.buildpack(reference);
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

fn create_buildpack_references(
    config: &BuildConfig,
    cargo_manifest_dir: &Path,
    target_buildpack_dir: &Path,
    cargo_env_additions: &[CargoEnvAddition],
) -> Vec<crate::pack::BuildpackReference> {
    config
        .buildpacks
        .iter()
        .map(|buildpack| match buildpack {
            BuildpackReference::CurrentCrate => {
                let dir = build::package_crate_buildpack(
                    config.cargo_profile,
                    &config.target_triple,
                    cargo_manifest_dir,
                    target_buildpack_dir,
                    cargo_env_additions,
                )
                .unwrap_or_else(|error| {
                    panic!("Error packaging current crate as buildpack: {error}")
                });
                crate::pack::BuildpackReference::from(dir)
            }
            BuildpackReference::WorkspaceBuildpack(buildpack_id) => {
                let dir = build::package_buildpack(
                    buildpack_id,
                    config.cargo_profile,
                    &config.target_triple,
                    cargo_manifest_dir,
                    target_buildpack_dir,
                    cargo_env_additions,
                )
                .unwrap_or_else(|error| {
                    panic!("Error packaging buildpack '{buildpack_id}': {error}")
                });
                crate::pack::BuildpackReference::from(dir)
            }
            BuildpackReference::Other(id) => crate::pack::BuildpackReference::from(id.clone()),
        })
        .collect()
}

#[derive(Debug)]
struct InvalidInstrumentationEnvVar {
    value: String,
}

impl std::fmt::Display for InvalidInstrumentationEnvVar {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(
            f,
            "Invalid value for LIBCNB_INSTRUMENTATION: {:?}. Expected \"1\", \"true\", \"0\", or \"false\".",
            self.value
        )
    }
}

fn instrumentation_enabled_via_env() -> Result<bool, InvalidInstrumentationEnvVar> {
    match env::var("LIBCNB_INSTRUMENTATION") {
        Err(_) => Ok(false),
        Ok(v) => match v.to_ascii_lowercase().as_str() {
            "1" | "true" => Ok(true),
            "0" | "false" | "" => Ok(false),
            _ => Err(InvalidInstrumentationEnvVar { value: v }),
        },
    }
}

const INSTRUMENTATION_CONTAINER_DIR: &str = "/tmp/llvm-cov";

struct InstrumentationSetup {
    volume: VolumeMount,
    pack_env: (String, String),
    cargo_env_additions: Vec<CargoEnvAddition>,
}

fn configure_instrumentation(cargo_manifest_dir: &Path) -> InstrumentationSetup {
    let workspace_root = find_cargo_workspace_root_dir(cargo_manifest_dir)
        .unwrap_or_else(|error| panic!("Error finding Cargo workspace root: {error}"));

    let dir = workspace_root.join("target/coverage/profraw");
    std::fs::create_dir_all(&dir)
        .unwrap_or_else(|error| panic!("Error creating coverage output directory: {error}"));
    // The CNB lifecycle runs buildpacks as a non-root user (e.g. uid 1000) which may differ
    // from the host user, so the mounted directory must be world-writable.
    #[cfg(unix)]
    {
        use std::os::unix::fs::PermissionsExt;
        std::fs::set_permissions(&dir, std::fs::Permissions::from_mode(0o777)).unwrap_or_else(
            |error| {
                panic!("Error setting coverage directory permissions: {error}");
            },
        );
    }
    let dir = std::fs::canonicalize(&dir)
        .unwrap_or_else(|error| panic!("Error canonicalizing coverage output directory: {error}"));

    InstrumentationSetup {
        volume: VolumeMount {
            source: dir,
            target: PathBuf::from(INSTRUMENTATION_CONTAINER_DIR),
            options: Some(String::from("rw")),
        },
        // %p = PID, %m = binary signature hash — prevents clobbering across concurrent runs.
        // See: https://doc.rust-lang.org/rustc/instrument-coverage.html
        pack_env: (
            String::from("LLVM_PROFILE_FILE"),
            format!("{INSTRUMENTATION_CONTAINER_DIR}/%p-%m.profraw"),
        ),
        cargo_env_additions: vec![CargoEnvAddition {
            key: OsString::from("RUSTFLAGS"),
            value: OsString::from("-C instrument-coverage"),
            separator: OsString::from(" "),
        }],
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
