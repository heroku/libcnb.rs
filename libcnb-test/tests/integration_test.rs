//! Integration tests using libcnb-test.
//!
//! All integration tests are skipped by default (using the `ignore` attribute),
//! since performing builds is slow. To run the tests use: `cargo test -- --ignored`

// Enable Clippy lints that are disabled by default.
// https://rust-lang.github.io/rust-clippy/stable/index.html
#![warn(clippy::pedantic)]

use indoc::indoc;
use libcnb_test::{
    assert_contains, assert_empty, assert_not_contains, BuildpackReference, ContainerConfig,
    PackResult, TestConfig, TestRunner,
};
use std::path::PathBuf;
use std::time::Duration;
use std::{env, fs, thread};

#[test]
#[ignore]
fn basic_build() {
    TestRunner::default().run_test(
        TestConfig::new("heroku/builder:22", "test-fixtures/procfile").buildpacks(vec![
            BuildpackReference::Other(String::from("heroku/procfile")),
        ]),
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
#[ignore]
fn rebuild() {
    TestRunner::default().run_test(
        TestConfig::new("heroku/builder:22", "test-fixtures/procfile").buildpacks(vec![
            BuildpackReference::Other(String::from("heroku/procfile")),
        ]),
        |context| {
            assert_empty!(context.pack_stderr);
            assert_not_contains!(context.pack_stdout, "Reusing layer");

            let config = context.config.clone();
            context.run_test(config, |rebuild_context| {
                assert_empty!(rebuild_context.pack_stderr);
                assert_contains!(rebuild_context.pack_stdout, "Reusing layer");
            });
        },
    );
}

#[test]
#[ignore]
fn starting_containers() {
    TestRunner::default().run_test(
        TestConfig::new("heroku/builder:22", "test-fixtures/procfile").buildpacks(vec![
            BuildpackReference::Other(String::from("heroku/procfile")),
        ]),
        |context| {
            context.start_container(
                ContainerConfig::new().env("PORT", "5678").expose_port(5678),
                |container| {
                    // Give the server time to boot up.
                    // TODO: Make requests to the server using a client that retries, and fetch logs after
                    // that, instead of sleeping. This will also allow us to test `expose_port()` etc.
                    thread::sleep(Duration::from_secs(2));

                    let log_output_until_now = container.logs_now();
                    assert_empty!(log_output_until_now.stderr);
                    assert_contains!(
                        log_output_until_now.stdout,
                        "Serving HTTP on 0.0.0.0 port 5678"
                    );

                    let exec_log_output = container.shell_exec("ps | grep python3");
                    assert_empty!(exec_log_output.stderr);
                    assert_contains!(exec_log_output.stdout, "python3");
                },
            );

            // TODO: Add a test for `start_with_default_process_args` based on the above,
            // that passes "5000" as the argument. This isn't possible at the moment,
            // since `lifecycle` seems to have a bug around passing arguments to
            // non-direct processes (and Procfile creates processes as non-direct).

            context.start_container(ContainerConfig::new().entrypoint(["worker"]), |container| {
                let all_log_output = container.logs_wait();
                assert_empty!(all_log_output.stderr);
                assert_eq!(all_log_output.stdout, "this is the worker process!\n");
            });

            context.start_container(
                ContainerConfig::new()
                    .entrypoint(["echo-args"])
                    .command(["Hello!"]),
                |container| {
                    let all_log_output = container.logs_wait();
                    assert_empty!(all_log_output.stderr);
                    assert_eq!(all_log_output.stdout, "Hello!\n");
                },
            );

            let log_output = context.run_shell_command("for i in {1..3}; do echo \"${i}\"; done");
            assert_empty!(log_output.stderr);
            assert_eq!(log_output.stdout, "1\n2\n3\n");
        },
    );
}

#[test]
#[ignore]
#[should_panic(
    expected = "Could not package current crate as buildpack: BuildBinariesError(ConfigError(NoBinTargetsFound))"
)]
fn buildpack_packaging_failure() {
    TestRunner::default().run_test(
        TestConfig::new("libcnb/invalid-builder", "test-fixtures/empty"),
        |_| {},
    );
}

#[test]
#[ignore]
#[should_panic(expected = "pack command unexpectedly failed with exit-code 1!

pack stdout:


pack stderr:
ERROR: failed to build: failed to fetch builder image 'index.docker.io/libcnb/invalid-builder:latest'")]
fn unexpected_pack_failure() {
    TestRunner::default().run_test(
        TestConfig::new("libcnb/invalid-builder", "test-fixtures/empty").buildpacks(Vec::new()),
        |_| {},
    );
}

#[test]
#[ignore]
#[should_panic(expected = "pack command unexpectedly succeeded with exit-code 0!

pack stdout:
")]
fn unexpected_pack_success() {
    TestRunner::default().run_test(
        TestConfig::new("heroku/builder:22", "test-fixtures/procfile")
            .buildpacks(vec![BuildpackReference::Other(String::from(
                "heroku/procfile",
            ))])
            .expected_pack_result(PackResult::Failure),
        |_| {},
    );
}

#[test]
#[ignore]
fn expected_pack_failure() {
    TestRunner::default().run_test(
        TestConfig::new("libcnb/invalid-builder", "test-fixtures/empty")
            .buildpacks(Vec::new())
            .expected_pack_result(PackResult::Failure),
        |context| {
            assert_empty!(context.pack_stdout);
            assert_contains!(
                context.pack_stderr,
                "ERROR: failed to build: failed to fetch builder image 'index.docker.io/libcnb/invalid-builder:latest'"
            );
        },
    );
}

#[test]
#[ignore]
#[should_panic(
    expected = "Could not package current crate as buildpack: BuildBinariesError(ConfigError(NoBinTargetsFound))"
)]
fn expected_pack_failure_still_panics_for_non_pack_failure() {
    TestRunner::default().run_test(
        TestConfig::new("libcnb/invalid-builder", "test-fixtures/empty")
            .expected_pack_result(PackResult::Failure),
        |_| {},
    );
}

#[test]
#[ignore]
fn app_dir_preprocessor() {
    TestRunner::default().run_test(
        TestConfig::new("heroku/builder:22", "test-fixtures/nested_dirs")
            .buildpacks(vec![BuildpackReference::Other(String::from(
                "heroku/procfile",
            ))])
            .app_dir_preprocessor(|app_dir| {
                assert!(app_dir.join("file1.txt").exists());
                fs::write(app_dir.join("Procfile"), "list-files: find . | sort").unwrap();
            }),
        |context| {
            context.start_container(ContainerConfig::new(), |container| {
                let log_output = container.logs_wait();
                assert_contains!(
                    log_output.stdout,
                    indoc! {"
                            ./Procfile
                            ./file1.txt
                            ./subdir1
                            ./subdir1/file2.txt
                            ./subdir1/subdir2
                            ./subdir1/subdir2/subdir3
                            ./subdir1/subdir2/subdir3/file3.txt
                        "}
                );
            });
        },
    );

    // Check that the original fixture was left untouched.
    let fixture_dir = env::var("CARGO_MANIFEST_DIR")
        .map(PathBuf::from)
        .unwrap()
        .join("test-fixtures/nested_dirs");
    assert!(fixture_dir.join("file1.txt").exists());
    assert!(!fixture_dir.join("Procfile").exists());
}

#[test]
#[ignore]
fn app_dir_absolute_path() {
    let absolute_app_dir = env::var("CARGO_MANIFEST_DIR")
        .map(PathBuf::from)
        .unwrap()
        .join("test-fixtures/procfile")
        .canonicalize()
        .unwrap();

    TestRunner::default().run_test(
        TestConfig::new("heroku/builder:22", absolute_app_dir).buildpacks(vec![
            BuildpackReference::Other(String::from("heroku/procfile")),
        ]),
        |_| {},
    );
}

#[test]
#[ignore]
// The actual panic message looks like this:
// `"App dir is not a valid directory: /.../libcnb-test/test-fixtures/non-existent-fixture"`
// It's intentionally an absolute path to make debugging failures easier when a relative path
// has been passed (the common case). However, since the absolute path is system/environment
// dependent, we would need to construct the expected string dynamically in `should_panic`,
// but cannot due to: https://github.com/rust-lang/rust/issues/88430
// As such we test the most important part, the fact that the error message lists the non-existent
// fixture directory path. We intentionally include the `libcnb-test/` crate directory prefix,
// since that only appears in the absolute path, not the relative path passed to `TestConfig::new`.
#[should_panic(expected = "libcnb-test/test-fixtures/non-existent-fixture")]
fn app_dir_invalid_path() {
    TestRunner::default().run_test(
        TestConfig::new("heroku/builder:22", "test-fixtures/non-existent-fixture")
            .buildpacks(Vec::new()),
        |_| {},
    );
}

#[test]
#[ignore]
// The actual panic message looks like this:
// `"App dir is not a valid directory: /.../libcnb-test/test-fixtures/non-existent-fixture"`
// See above for why we only test this substring.
#[should_panic(expected = "libcnb-test/test-fixtures/non-existent-fixture")]
fn app_dir_invalid_path_checked_before_applying_preprocessor() {
    TestRunner::default().run_test(
        TestConfig::new("heroku/builder:22", "test-fixtures/non-existent-fixture")
            .buildpacks(Vec::new())
            .app_dir_preprocessor(|_| {}),
        |_| {},
    );
}
