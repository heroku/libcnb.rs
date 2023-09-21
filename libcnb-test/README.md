# libcnb-test &emsp; [![Docs]][docs.rs] [![Latest Version]][crates.io] [![MSRV]][install-rust]

An integration testing framework for Cloud Native Buildpacks written in Rust with [libcnb.rs](https://github.com/heroku/libcnb.rs).

The framework:
- Automatically cross-compiles and packages the buildpack under test
- Performs a build with specified configuration using `pack build`
- Supports starting containers using the resultant application image
- Supports concurrent test execution
- Handles cleanup of the test containers and images
- Provides additional test assertion macros to simplify common test scenarios (for example, `assert_contains!`)

## Dependencies

Integration tests require the following to be available on the host:

- [Docker](https://docs.docker.com/engine/install/)
- [Pack CLI](https://buildpacks.io/docs/install-pack/)
- [Cross-compilation prerequisites](https://docs.rs/libcnb/latest/libcnb/#cross-compilation-prerequisites) (however `libcnb-cargo` itself is not required)

Only local Docker daemons are fully supported. As such, if you are using Circle CI you must use the
[`machine` executor](https://circleci.com/docs/2.0/executor-types/#using-machine) rather than the
[remote docker](https://circleci.com/docs/2.0/building-docker-images/) feature.

## Examples

A basic test that performs a build with the specified builder image and app source fixture,
and then asserts against the resultant `pack build` log output:

```rust,no_run
// In $CRATE_ROOT/tests/integration_test.rs
use libcnb_test::{assert_contains, assert_empty, BuildConfig, TestRunner};

// Note: In your code you'll want to uncomment the `#[test]` annotation here.
// It's commented out in these examples so that this documentation can be
// run as a `doctest` and so checked for correctness in CI.
// #[test]
fn basic() {
    TestRunner::default().build(
        BuildConfig::new("heroku/builder:22", "tests/fixtures/app"),
        |context| {
            assert_empty!(context.pack_stderr);
            assert_contains!(context.pack_stdout, "Expected build output");
        },
    );
}
```

Performing a second build of the same image to test cache handling, using [`TestContext::rebuild`]:

```rust,no_run
use libcnb_test::{assert_contains, BuildConfig, TestRunner};

// #[test]
fn rebuild() {
    TestRunner::default().build(
        BuildConfig::new("heroku/builder:22", "tests/fixtures/app"),
        |context| {
            assert_contains!(context.pack_stdout, "Installing dependencies");

            let config = context.config.clone();
            context.rebuild(config, |rebuild_context| {
                assert_contains!(rebuild_context.pack_stdout, "Using cached dependencies");
            });
        },
    );
}
```

Testing expected buildpack failures, using [`BuildConfig::expected_pack_result`]:

```rust,no_run
use libcnb_test::{assert_contains, BuildConfig, PackResult, TestRunner};

// #[test]
fn expected_pack_failure() {
    TestRunner::default().build(
        BuildConfig::new("heroku/builder:22", "tests/fixtures/invalid-app")
            .expected_pack_result(PackResult::Failure),
        |context| {
            assert_contains!(context.pack_stderr, "ERROR: Invalid Procfile!");
        },
    );
}
```

Running a shell command against the built image, using [`TestContext::run_shell_command`]:

```rust,no_run
use libcnb_test::{assert_empty, BuildConfig, TestRunner};

// #[test]
fn run_shell_command() {
    TestRunner::default().build(
        BuildConfig::new("heroku/builder:22", "tests/fixtures/app"),
        |context| {
            // ...
            let command_output = context.run_shell_command("python --version");
            assert_empty!(command_output.stderr);
            assert_eq!(command_output.stdout, "Python 3.10.4\n");
        },
    );
}
```

Starting a container using the default process with an exposed port to test a web server, using [`TestContext::start_container`]:

```rust,no_run
use libcnb_test::{assert_contains, assert_empty, BuildConfig, ContainerConfig, TestRunner};
use std::thread;
use std::time::Duration;

const TEST_PORT: u16 = 12345;

// #[test]
fn starting_web_server_container() {
    TestRunner::default().build(
        BuildConfig::new("heroku/builder:22", "tests/fixtures/app"),
        |context| {
            // ...
            context.start_container(
                ContainerConfig::new()
                    .env("PORT", TEST_PORT.to_string())
                    .expose_port(TEST_PORT),
                |container| {
                    let address_on_host = container.address_for_port(TEST_PORT);
                    let url = format!("http://{}:{}", address_on_host.ip(), address_on_host.port());

                    // Give the server time to start.
                    thread::sleep(Duration::from_secs(2));

                    let server_log_output = container.logs_now();
                    assert_empty!(server_log_output.stderr);
                    assert_contains!(
                        server_log_output.stdout,
                        &format!("Listening on port {TEST_PORT}")
                    );

                    let response = ureq::get(&url).call().unwrap();
                    let body = response.into_string().unwrap();
                    assert_contains!(body, "Expected response substring");
                },
            );
        },
    );
}
```

Inspecting an already running container using Docker Exec, using [`ContainerContext::shell_exec`]:

```rust,no_run
use libcnb_test::{assert_contains, BuildConfig, ContainerConfig, TestRunner};

// #[test]
fn shell_exec() {
    TestRunner::default().build(
        BuildConfig::new("heroku/builder:22", "tests/fixtures/app"),
        |context| {
            // ...
            context.start_container(ContainerConfig::new(), |container| {
                // ...
                let exec_log_output = container.shell_exec("ps");
                assert_contains!(exec_log_output.stdout, "nginx");
            });
        },
    );
}
```

Dynamically modifying test fixtures during test setup, using [`BuildConfig::app_dir_preprocessor`]:

```rust,no_run
use libcnb_test::{BuildConfig, TestRunner};
use std::fs;

// #[test]
fn dynamic_fixture() {
    TestRunner::default().build(
        BuildConfig::new("heroku/builder:22", "tests/fixtures/app").app_dir_preprocessor(
            |app_dir| {
                fs::write(app_dir.join("runtime.txt"), "python-3.10").unwrap();
            },
        ),
        |context| {
            // ...
        },
    );
}
```

Building with multiple buildpacks, using [`BuildConfig::buildpacks`]:

```rust,no_run
use libcnb::data::buildpack_id;
use libcnb_test::{BuildConfig, BuildpackReference, TestRunner};

// #[test]
fn additional_buildpacks() {
    TestRunner::default().build(
        BuildConfig::new("heroku/builder:22", "tests/fixtures/app").buildpacks([
            BuildpackReference::CurrentCrate,
            BuildpackReference::WorkspaceBuildpack(buildpack_id!("my-project/buildpack")),
            BuildpackReference::Other(String::from("heroku/another-buildpack")),
        ]),
        |context| {
            // ...
        },
    );
}
```

## Tips

- Rust tests are automatically run in parallel, however only if they are in the same crate.
  For integration tests Rust compiles each file as a separate crate. As such, make sure to
  include all integration tests in a single file (either inlined or by including additional
  test modules) to ensure they run in parallel.
- If you would like to be able to more easily run your unit tests and integration tests
  separately, annotate each integration test with `#[ignore = "integration test"]`, which
  causes `cargo test` to skip them (running unit/doc tests only). The integration tests
  can then be run using `cargo test -- --ignored`, or all tests can be run at once using
  `cargo test -- --include-ignored`.
- If you wish to assert against multi-line log output, see the [indoc](https://crates.io/crates/indoc) crate.

[Docs]: https://img.shields.io/docsrs/libcnb-test
[docs.rs]: https://docs.rs/libcnb-test/latest/libcnb_test/
[Latest Version]: https://img.shields.io/crates/v/libcnb-test.svg
[crates.io]: https://crates.io/crates/libcnb-test
[MSRV]: https://img.shields.io/badge/MSRV-rustc_1.64+-lightgray.svg
[install-rust]: https://www.rust-lang.org/tools/install
