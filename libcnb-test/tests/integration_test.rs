//! All integration tests are skipped by default (using the `ignore` attribute)
//! since performing builds is slow. To run them use: `cargo test -- --ignored`.
//!
//! When testing panics, prefer using `#[should_panic(expected = "...")]`, unless you need
//! to test dynamic values, in which case the only option is to use `panic::catch_unwind`
//! since `should_panic` doesn't support globs/regular expressions/compile time macros.

// Enable Clippy lints that are disabled by default.
// https://rust-lang.github.io/rust-clippy/stable/index.html
#![warn(clippy::pedantic)]

use indoc::{formatdoc, indoc};
use libcnb_data::buildpack_id;
use libcnb_test::{
    assert_contains, assert_empty, assert_not_contains, BuildConfig, BuildpackReference,
    ContainerConfig, PackResult, TestRunner,
};
use std::path::PathBuf;
use std::time::Duration;
use std::{env, fs, panic, thread};

const PROCFILE_URL: &str = "heroku/procfile";
const TEST_PORT: u16 = 12345;

// Note: Since the `libcnb-test` crate isn't a buildpack itself, we can't test a successful
// build using `BuildpackReference::CurrentCrate` here, but we still have test coverage of it
// thanks to the integration tests under `examples/` and `test-buildpacks/`.

#[test]
#[ignore = "integration test"]
fn build_other_buildpack() {
    TestRunner::default().build(
        BuildConfig::new("heroku/builder:22", "tests/fixtures/procfile")
            .buildpacks([BuildpackReference::Other(String::from(PROCFILE_URL))]),
        |context| {
            assert_empty!(context.pack_stderr);
            assert_contains!(
                context.pack_stdout,
                indoc! {"
                    [Discovering process types]
                    Procfile declares types -> web, worker, echo-args
                "}
            );
        },
    );
}

#[test]
#[ignore = "integration test"]
fn build_workspace_component_buildpack() {
    TestRunner::default().build(
        BuildConfig::new("heroku/builder:22", "tests/fixtures/empty").buildpacks([
            BuildpackReference::WorkspaceBuildpack(buildpack_id!("libcnb-test/a")),
        ]),
        |context| {
            assert_empty!(context.pack_stderr);
            assert_contains!(
                context.pack_stdout,
                indoc! {"
                    Buildpack A
                "}
            );
        },
    );
}

#[test]
#[ignore = "integration test"]
fn build_workspace_composite_buildpack() {
    TestRunner::default().build(
        BuildConfig::new("heroku/builder:22", "tests/fixtures/procfile").buildpacks([
            BuildpackReference::WorkspaceBuildpack(buildpack_id!("libcnb-test/composite")),
        ]),
        |context| {
            assert_empty!(context.pack_stderr);
            assert_contains!(
                context.pack_stdout,
                indoc! {"
                    Buildpack A
                    Buildpack B
                    
                    [Discovering process types]
                    Procfile declares types -> web, worker, echo-args
                "}
            );
        },
    );
}

#[test]
#[ignore = "integration test"]
fn build_multiple_buildpacks() {
    TestRunner::default().build(
        BuildConfig::new("heroku/builder:22", "tests/fixtures/procfile").buildpacks([
            BuildpackReference::WorkspaceBuildpack(buildpack_id!("libcnb-test/b")),
            BuildpackReference::Other(String::from(PROCFILE_URL)),
            BuildpackReference::WorkspaceBuildpack(buildpack_id!("libcnb-test/a")),
        ]),
        |context| {
            assert_empty!(context.pack_stderr);
            assert_contains!(
                context.pack_stdout,
                indoc! {"
                    Buildpack B
                    
                    [Discovering process types]
                    Procfile declares types -> web, worker, echo-args
                    Buildpack A
                "}
            );
        },
    );
}

#[test]
#[ignore = "integration test"]
fn rebuild() {
    TestRunner::default().build(
        BuildConfig::new("heroku/builder:22", "tests/fixtures/procfile")
            .buildpacks([BuildpackReference::Other(String::from(PROCFILE_URL))]),
        |context| {
            assert_empty!(context.pack_stderr);
            assert_not_contains!(context.pack_stdout, "Reusing layer");

            let config = context.config.clone();
            context.rebuild(config, |rebuild_context| {
                assert_empty!(rebuild_context.pack_stderr);
                assert_contains!(rebuild_context.pack_stdout, "Reusing layer");
            });
        },
    );
}

#[test]
#[ignore = "integration test"]
fn packaging_failure_missing_buildpack_toml() {
    let err = panic::catch_unwind(|| {
        TestRunner::default().build(BuildConfig::new("invalid!", "tests/fixtures/empty"), |_| {
            unreachable!("The test should panic prior to the TestContext being invoked.");
        });
    })
    .unwrap_err();

    assert_eq!(
        err.downcast_ref::<String>().unwrap(),
        &format!(
            "Error packaging current crate as buildpack: Couldn't find a buildpack.toml file at {}",
            env::var("CARGO_MANIFEST_DIR")
                .map(PathBuf::from)
                .unwrap()
                .join("buildpack.toml")
                .display()
        )
    );
}

#[test]
#[ignore = "integration test"]
// TODO: Fix the implementation to return the correct error message (that the buildpack.toml doesn't match the schema):
// https://github.com/heroku/libcnb.rs/issues/708
#[should_panic(
    expected = "Error packaging buildpack 'libcnb-test/invalid-buildpack-toml': Couldn't find a buildpack with ID 'libcnb-test/invalid-buildpack-toml' in the workspace at"
)]
fn packaging_failure_invalid_buildpack_toml() {
    TestRunner::default().build(
        BuildConfig::new("invalid!", "tests/fixtures/empty").buildpacks([
            BuildpackReference::WorkspaceBuildpack(buildpack_id!(
                "libcnb-test/invalid-buildpack-toml"
            )),
        ]),
        |_| {
            unreachable!("The test should panic prior to the TestContext being invoked.");
        },
    );
}

#[test]
#[ignore = "integration test"]
#[should_panic(
    expected = "Error packaging buildpack 'libcnb-test/composite-missing-package-toml': Couldn't read package.toml: I/O error while reading/writing TOML file: No such file or directory (os error 2)"
)]
fn packaging_failure_composite_buildpack_missing_package_toml() {
    TestRunner::default().build(
        BuildConfig::new("invalid!", "tests/fixtures/empty").buildpacks([
            BuildpackReference::WorkspaceBuildpack(buildpack_id!(
                "libcnb-test/composite-missing-package-toml"
            )),
        ]),
        |_| {
            unreachable!("The test should panic prior to the TestContext being invoked.");
        },
    );
}

#[test]
#[ignore = "integration test"]
#[should_panic(
    expected = "Error packaging buildpack 'libcnb-test/invalid-cargo-toml': Obtaining Cargo metadata failed: `cargo metadata` exited with an error: "
)]
fn packaging_failure_invalid_cargo_toml() {
    TestRunner::default().build(
        BuildConfig::new("invalid!", "tests/fixtures/empty").buildpacks([
            BuildpackReference::WorkspaceBuildpack(buildpack_id!("libcnb-test/invalid-cargo-toml")),
        ]),
        |_| {
            unreachable!("The test should panic prior to the TestContext being invoked.");
        },
    );
}

#[test]
#[ignore = "integration test"]
#[should_panic(
    expected = "Error packaging buildpack 'libcnb-test/compile-error': Building buildpack binaries failed: Failed to build binary target compile-error: Cargo unexpectedly exited with status exit status: 101"
)]
fn packaging_failure_compile_error() {
    TestRunner::default().build(
        BuildConfig::new("invalid!", "tests/fixtures/empty").buildpacks([
            BuildpackReference::WorkspaceBuildpack(buildpack_id!("libcnb-test/compile-error")),
        ]),
        |_| {
            unreachable!("The test should panic prior to the TestContext being invoked.");
        },
    );
}

#[test]
#[ignore = "integration test"]
fn packaging_failure_non_existent_workspace_buildpack() {
    let err = panic::catch_unwind(|| {
        TestRunner::default().build(
            BuildConfig::new("invalid!", "tests/fixtures/empty").buildpacks([
                BuildpackReference::WorkspaceBuildpack(buildpack_id!("non-existent")),
            ]),
            |_| {
                unreachable!("The test should panic prior to the TestContext being invoked.");
            },
        );
    })
    .unwrap_err();

    assert_eq!(
        err.downcast_ref::<String>().unwrap(),
        &format!(
            "Error packaging buildpack 'non-existent': Couldn't find a buildpack with ID 'non-existent' in the workspace at {}",
            // There is currently no env var for determining the workspace root directly:
            // https://github.com/rust-lang/cargo/issues/3946
            env::var("CARGO_MANIFEST_DIR")
                .map(PathBuf::from)
                .unwrap()
                .join("../")
                .canonicalize()
                .unwrap()
                .display()
        )
    );
}

#[test]
#[ignore = "integration test"]
#[should_panic(expected = "Error performing pack build:

pack command failed with exit code 1!

## stderr:

ERROR: failed to build: invalid builder 'invalid!'")]
fn unexpected_pack_failure() {
    TestRunner::default().build(
        BuildConfig::new("invalid!", "tests/fixtures/empty").buildpacks(Vec::new()),
        |_| {
            unreachable!("The test should panic prior to the TestContext being invoked.");
        },
    );
}

#[test]
#[ignore = "integration test"]
#[should_panic(expected = "The pack build was expected to fail, but did not:

## stderr:


## stdout:

===> ANALYZING
")]
fn unexpected_pack_success() {
    TestRunner::default().build(
        BuildConfig::new("heroku/builder:22", "tests/fixtures/procfile")
            .buildpacks([BuildpackReference::Other(String::from(PROCFILE_URL))])
            .expected_pack_result(PackResult::Failure),
        |_| {
            unreachable!("The test should panic prior to the TestContext being invoked.");
        },
    );
}

#[test]
#[ignore = "integration test"]
fn expected_pack_failure() {
    TestRunner::default().build(
        BuildConfig::new("invalid!", "tests/fixtures/empty")
            .buildpacks(Vec::new())
            .expected_pack_result(PackResult::Failure),
        |context| {
            assert_empty!(context.pack_stdout);
            assert_contains!(
                context.pack_stderr,
                "ERROR: failed to build: invalid builder 'invalid!'"
            );
        },
    );
}

#[test]
#[ignore = "integration test"]
#[should_panic(
    expected = "Error packaging current crate as buildpack: Couldn't find a buildpack.toml file"
)]
fn expected_pack_failure_still_panics_for_non_pack_failure() {
    TestRunner::default().build(
        BuildConfig::new("invalid!", "tests/fixtures/empty")
            .expected_pack_result(PackResult::Failure),
        |_| {
            unreachable!("The test should panic prior to the TestContext being invoked.");
        },
    );
}

#[test]
#[ignore = "integration test"]
fn app_dir_preprocessor() {
    TestRunner::default().build(
        BuildConfig::new("heroku/builder:22", "tests/fixtures/nested_dirs")
            .buildpacks([BuildpackReference::Other(String::from(PROCFILE_URL))])
            .app_dir_preprocessor(|app_dir| {
                assert!(app_dir.join("file1.txt").exists());
                fs::remove_file(app_dir.join("file1.txt")).unwrap();
                assert!(!app_dir.join("Procfile").exists());
                fs::write(app_dir.join("Procfile"), "web: true").unwrap();
            }),
        |context| {
            let expected_directory_listing = indoc! {"
                .
                ./Procfile
                ./subdir1
                ./subdir1/file2.txt
                ./subdir1/subdir2
                ./subdir1/subdir2/subdir3
                ./subdir1/subdir2/subdir3/file3.txt
            "};

            // The cache path exclusion is required since when using Rosetta on macOS
            // a cache directory is created at `$HOME/.cache/rosetta`.
            let log_output = context.run_shell_command("find . -not -path './.cache*' | sort");
            assert_empty!(log_output.stderr);
            assert_eq!(log_output.stdout, expected_directory_listing);

            // Check that rebuilds get a new/clean ephemeral fixture directory.
            let config = context.config.clone();
            context.rebuild(config, |context| {
                let log_output = context.run_shell_command("find . -not -path './.cache*' | sort");
                assert_empty!(log_output.stderr);
                assert_eq!(log_output.stdout, expected_directory_listing);
            });
        },
    );

    // Check that the original fixture was left untouched.
    let fixture_dir = env::var("CARGO_MANIFEST_DIR")
        .map(PathBuf::from)
        .unwrap()
        .join("tests/fixtures/nested_dirs");
    assert!(fixture_dir.join("file1.txt").exists());
    assert!(!fixture_dir.join("Procfile").exists());
}

#[test]
#[ignore = "integration test"]
fn app_dir_absolute_path() {
    let absolute_app_dir = env::var("CARGO_MANIFEST_DIR")
        .map(PathBuf::from)
        .unwrap()
        .join("tests/fixtures/procfile")
        .canonicalize()
        .unwrap();

    TestRunner::default().build(
        BuildConfig::new("heroku/builder:22", absolute_app_dir)
            .buildpacks([BuildpackReference::Other(String::from(PROCFILE_URL))]),
        |_| {},
    );
}

#[test]
#[ignore = "integration test"]
fn app_dir_invalid_path() {
    let err = panic::catch_unwind(|| {
        TestRunner::default().build(
            BuildConfig::new("invalid!", "tests/fixtures/non-existent-fixture")
                .buildpacks(Vec::new())
                .app_dir_preprocessor(|_| {
                    unreachable!("The app dir should be validated before the preprocessor is run.");
                }),
            |_| {},
        );
    })
    .unwrap_err();

    assert_eq!(
        err.downcast_ref::<String>().unwrap(),
        &format!(
            "App dir is not a valid directory: {}",
            env::var("CARGO_MANIFEST_DIR")
                .map(PathBuf::from)
                .unwrap()
                .join("tests/fixtures/non-existent-fixture")
                .display()
        )
    );
}

#[test]
#[ignore = "integration test"]
#[should_panic(expected = "Error downloading SBOM files:

pack command failed with exit code 1!

## stderr:

ERROR: image 'libcnbtest_")]
fn download_sbom_files_failure() {
    TestRunner::default().build(
        BuildConfig::new("invalid!", "tests/fixtures/empty")
            .buildpacks(Vec::new())
            .expected_pack_result(PackResult::Failure),
        |context| {
            context.download_sbom_files(|_| {});
        },
    );
}

#[test]
#[ignore = "integration test"]
fn starting_containers() {
    TestRunner::default().build(
        BuildConfig::new("heroku/builder:22", "tests/fixtures/procfile")
            .buildpacks([BuildpackReference::Other(String::from(PROCFILE_URL))]),
        |context| {
            // Using the default entrypoint and command.
            context.start_container(
                ContainerConfig::new()
                    .env("PORT", TEST_PORT.to_string())
                    .expose_port(TEST_PORT),
                |container| {
                    let address_on_host = container.address_for_port(TEST_PORT);
                    let url = format!("http://{}:{}", address_on_host.ip(), address_on_host.port());

                    // Retries needed since the server takes a moment to start up.
                    let mut attempts_remaining = 5;
                    let response = loop {
                        let response = ureq::get(&url).call();
                        if response.is_ok() || attempts_remaining == 0 {
                            break response;
                        }
                        attempts_remaining -= 1;
                        thread::sleep(Duration::from_secs(1));
                    }
                    .unwrap();

                    let body = response.into_string().unwrap();
                    assert_contains!(body, "Directory listing for /");

                    let server_log_output = container.logs_now();
                    assert_contains!(
                        server_log_output.stdout,
                        &format!("Serving HTTP on 0.0.0.0 port {TEST_PORT}")
                    );
                    assert_contains!(server_log_output.stderr, "GET /");

                    let exec_log_output = container.shell_exec("ps | grep python3");
                    assert_empty!(exec_log_output.stderr);
                    assert_contains!(exec_log_output.stdout, "python3");
                },
            );

            // Overriding the default entrypoint, but using the default command.
            context.start_container(ContainerConfig::new().entrypoint("worker"), |container| {
                let all_log_output = container.logs_wait();
                assert_empty!(all_log_output.stderr);
                assert_eq!(all_log_output.stdout, "this is the worker process!\n");
            });

            // Overriding both the entrypoint and command.
            context.start_container(
                ContainerConfig::new()
                    .entrypoint("echo-args")
                    .command(["$GREETING", "$DESIGNATION"])
                    .envs([("GREETING", "Hello"), ("DESIGNATION", "World")]),
                |container| {
                    let all_log_output = container.logs_wait();
                    assert_empty!(all_log_output.stderr);
                    assert_eq!(all_log_output.stdout, "Hello World\n");
                },
            );

            let command_output =
                context.run_shell_command("for i in {1..3}; do echo \"${i}\"; done");
            assert_empty!(command_output.stderr);
            assert_eq!(command_output.stdout, "1\n2\n3\n");
        },
    );
}

#[test]
#[ignore = "integration test"]
#[should_panic(expected = "Error starting container:

docker command failed with exit code 127!

## stderr:

docker: Error response from daemon:")]
fn start_container_spawn_failure() {
    TestRunner::default().build(
        BuildConfig::new("heroku/builder:22", "tests/fixtures/procfile")
            .buildpacks([BuildpackReference::Other(String::from(PROCFILE_URL))]),
        |context| {
            context.start_container(
                ContainerConfig::new().entrypoint("nonexistent-command"),
                |_| {
                    unreachable!("The test should fail before the ContainerContext is invoked.");
                },
            );
        },
    );
}

#[test]
#[ignore = "integration test"]
#[should_panic(expected = "Error performing docker exec:

docker command failed with exit code 1!

## stderr:

Error response from daemon:")]
fn shell_exec_when_container_has_crashed() {
    TestRunner::default().build(
        BuildConfig::new("heroku/builder:22", "tests/fixtures/procfile")
            .buildpacks([BuildpackReference::Other(String::from(PROCFILE_URL))]),
        |context| {
            context.start_container(
                ContainerConfig::new()
                    .entrypoint("launcher")
                    .command(["exit 1"]),
                |container| {
                    thread::sleep(Duration::from_secs(1));
                    container.shell_exec("ps");
                },
            );
        },
    );
}

#[test]
#[ignore = "integration test"]
#[should_panic(expected = "Error performing docker exec:

docker command failed with exit code 1!

## stderr:

some stderr

## stdout:

some stdout
")]
fn shell_exec_nonzero_exit_status() {
    TestRunner::default().build(
        BuildConfig::new("heroku/builder:22", "tests/fixtures/procfile")
            .buildpacks([BuildpackReference::Other(String::from(PROCFILE_URL))]),
        |context| {
            context.start_container(ContainerConfig::new(), |container| {
                thread::sleep(Duration::from_secs(1));
                container.shell_exec("echo 'some stdout'; echo 'some stderr' >&2; exit 1");
            });
        },
    );
}

#[test]
#[ignore = "integration test"]
#[should_panic(expected = "Error running container:

docker command failed with exit code 1!

## stderr:

some stderr

## stdout:

some stdout
")]
fn run_shell_command_nonzero_exit_status() {
    TestRunner::default().build(
        BuildConfig::new("heroku/builder:22", "tests/fixtures/procfile")
            .buildpacks([BuildpackReference::Other(String::from(PROCFILE_URL))]),
        |context| {
            context.run_shell_command("echo 'some stdout'; echo 'some stderr' >&2; exit 1");
        },
    );
}

#[test]
#[ignore = "integration test"]
fn logs_work_after_container_crashed() {
    TestRunner::default().build(
        BuildConfig::new("heroku/builder:22", "tests/fixtures/procfile")
            .buildpacks([BuildpackReference::Other(String::from(PROCFILE_URL))]),
        |context| {
            context.start_container(
                ContainerConfig::new()
                    .entrypoint("launcher")
                    .command(["echo 'some stdout'; echo 'some stderr' >&2; exit 1"]),
                |container| {
                    thread::sleep(Duration::from_secs(1));
                    let server_log_output = container.logs_now();
                    assert_eq!(server_log_output.stdout, "some stdout\n");
                    assert_eq!(server_log_output.stderr, "some stderr\n");
                },
            );
        },
    );
}

#[test]
#[ignore = "integration test"]
#[should_panic(
    expected = "Unknown port: Port 12345 needs to be exposed first using `ContainerConfig::expose_port`"
)]
fn address_for_port_when_port_not_exposed() {
    TestRunner::default().build(
        BuildConfig::new("heroku/builder:22", "tests/fixtures/procfile")
            .buildpacks([BuildpackReference::Other(String::from(PROCFILE_URL))]),
        |context| {
            context.start_container(ContainerConfig::new(), |container| {
                let _ = container.address_for_port(TEST_PORT);
            });
        },
    );
}

#[test]
#[ignore = "integration test"]
fn address_for_port_when_container_crashed() {
    let mut container_name = String::new();

    // AssertUnwindSafe is required so that `container_name`` can be mutated across the unwind boundary.
    let err = panic::catch_unwind(panic::AssertUnwindSafe(|| {
        TestRunner::default().build(
            BuildConfig::new("heroku/builder:22", "tests/fixtures/procfile")
                .buildpacks([BuildpackReference::Other(String::from(PROCFILE_URL))]),
            |context| {
                context.start_container(
                    ContainerConfig::new()
                        .entrypoint("launcher")
                        .command(["echo 'some stdout'; echo 'some stderr' >&2; exit 1"])
                        .expose_port(TEST_PORT),
                    |container| {
                        container_name = container.container_name.clone();
                        // Wait for the container to actually exit, otherwise `address_for_port()` will succeed.
                        thread::sleep(Duration::from_secs(1));
                        let _ = container.address_for_port(TEST_PORT);
                    },
                );
            },
        );
    }))
    .unwrap_err();

    assert_eq!(
        err.downcast_ref::<String>().unwrap(),
        &formatdoc! {"
            Error obtaining container port mapping:
            Error: No public port '12345' published for {container_name}

            This normally means that the container crashed. Container logs:
            
            ## stderr:
            
            some stderr
            
            ## stdout:
            
            some stdout
            
        "}
    );
}
