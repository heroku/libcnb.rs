# Changelog

## [Unreleased]

- Add `TestContext::download_sbom_files` to allow testing of SBOM logic. ([#489](https://github.com/heroku/libcnb.rs/pull/489))

## [0.6.0] 2022-07-21

- Overhaul the crate README/docs, to improve the learning/onboarding UX. ([#478](https://github.com/heroku/libcnb.rs/pull/478))
- Rename `TestRunner::run_test` to `TestRunner::build`, `TestConfig` to `BuildConfig` and `TestContext::run_test` to `TestContext::rebuild`. ([#470](https://github.com/heroku/libcnb.rs/pull/470))
- Add `TestContext::start_container`, `TestContext::run_shell_command` and `ContainerConfig`. ([#469](https://github.com/heroku/libcnb.rs/pull/469))
- Remove `TestContext::prepare_container` and `PrepareContainerContext`. To start a container use `TestContext::start_container` combined with `ContainerConfig` (or else the convenience function `TestContext::run_shell_command`) instead. ([#469](https://github.com/heroku/libcnb.rs/pull/469))
- Fix missing logs when using `ContainerContext::logs_now`. ([#471](https://github.com/heroku/libcnb.rs/pull/471))

## [0.5.0] 2022-07-14

- Add an `assert_empty!` macro, for improved UX over using `assert!` with `str::is_empty`. ([#451](https://github.com/heroku/libcnb.rs/pull/451))
- Add `TestConfig::cargo_profile` and `CargoProfile` to support compilation of buildpacks in release mode. ([#456](https://github.com/heroku/libcnb.rs/pull/456))
- Improve the error message shown when Pack CLI is not installed. ([#455](https://github.com/heroku/libcnb.rs/pull/455))
- Check that `TestConfig::app_dir` is a valid directory before applying `TestConfig::app_dir_preprocessor` or invoking Pack, so that a clearer error message can be displayed. ([#452](https://github.com/heroku/libcnb.rs/pull/452))
- `PrepareContainerContext::start_with_shell_command` and `ContainerContext::shell_exec` now use `bash` instead of `/bin/sh`. ([#464](https://github.com/heroku/libcnb.rs/pull/464))
- Use the crate's `README.md` as the root module's rustdocs, so that all of the crate's documentation can be seen in one place on `docs.rs`. ([#460](https://github.com/heroku/libcnb.rs/pull/460))
- Add rustdocs with code examples for all public APIs. ([#441](https://github.com/heroku/libcnb.rs/pull/441))
- Fix rustdocs for `LogOutput`. ([#440](https://github.com/heroku/libcnb.rs/pull/440))
- Increase minimum supported Rust version from 1.58 to 1.59. ([#445](https://github.com/heroku/libcnb.rs/pull/445))
- Update `libcnb-package` from `0.1.2` to `0.2.0`. Of note, buildpack binaries are now stripped when packaging to reduce buildpack size, which also speeds up integration tests. See the [libcnb-package changelog](../libcnb-package/CHANGELOG.md#020-2022-07-14). ([#465](https://github.com/heroku/libcnb.rs/pull/465))

## [0.4.0] 2022-06-24

- Leverage `Into` trait for `String`/`&str` arguments in `ContainerContext`. ([#412](https://github.com/heroku/libcnb.rs/pull/412))
- Pass `--trust-builder` to `pack build` to ensure the builders used in tests are always trusted. ([#409](https://github.com/heroku/libcnb.rs/pull/409))
- Add `IntegrationTest::app_dir_preprocessor`, allowing users to modify the app directory before an integration test run. ([#397](https://github.com/heroku/libcnb.rs/pull/397))
- Update `bollard` dependency from `0.12.0` to `0.13.0`. ([#419](https://github.com/heroku/libcnb.rs/pull/419))
- Update `cargo_metadata` dependency from `0.14.2` to `0.15.0`. ([#423](https://github.com/heroku/libcnb.rs/pull/423))
- Add `assert_not_contains!` macro, an inverted version of `assert_contains!`. ([#424](https://github.com/heroku/libcnb.rs/pull/424))
- Remove `IntegrationTest::run_test`, to run a test use the new `TestRunner::run_test` function. ([#422](https://github.com/heroku/libcnb.rs/pull/422))
- Rename `IntegrationTest` to `TestConfig`. ([#422](https://github.com/heroku/libcnb.rs/pull/422))
- Rename `IntegrationTestContext` to `TestContext`. ([#422](https://github.com/heroku/libcnb.rs/pull/422))
- Add `Clone` implementation for `TestConfig`, allowing it to be shared across tests. ([#422](https://github.com/heroku/libcnb.rs/pull/422))
- Add `TestContext::run_test`, allowing you to run subsequent integration tests with the image from a previous test. These functions allow testing of subsequent builds, including caching logic and buildpack behaviour when build environment variables change, stacks are upgraded and more. ([#422](https://github.com/heroku/libcnb.rs/pull/422))
- Add `TestConfig::expected_pack_result`. When set to `PackResult::Failure`, it allows testing of build failure scenarios. ([#429](https://github.com/heroku/libcnb.rs/pull/429))
- Add `TestConfig::app_dir` which is handy in cases where `TestConfig` values are shared and only the `app_dir` needs to be different. ([#430](https://github.com/heroku/libcnb.rs/pull/430))
- Remove `TestContext::app_dir` to encourage the use of `TestConfig::app_dir_preprocessor`. ([#431](https://github.com/heroku/libcnb.rs/pull/431))
- Improve performance when no `TestConfig::app_dir_preprocessor` is configured by skipping application directory copying. ([#431](https://github.com/heroku/libcnb.rs/pull/431))

## [0.3.1] 2022-04-12

- Update project URLs for the GitHub repository move to the `heroku` org. ([#388](https://github.com/heroku/libcnb.rs/pull/388))

## [0.3.0] 2022-03-08

- Add `IntegrationTest::env` and `IntegrationTest::envs`, allowing users to set environment variables for the build process. ([#346](https://github.com/heroku/libcnb.rs/pull/346))
- Replaced `IntegrationTestContext::start_container` with `IntegrationTestContext::prepare_container`, allowing users to configure the container before starting it. Ports can now be exposed via `PrepareContainerContext::expose_port`. ([#346](https://github.com/heroku/libcnb.rs/pull/346))
- Added the ability to set environment variables for the container via `PrepareContainerContext::env` and `PrepareContainerContext::envs`. ([#346](https://github.com/heroku/libcnb.rs/pull/346))
- Switch from `libcnb-cargo` to the new `libcnb-package` crate for buildpack packaging, which improves compile times due to it not including CLI-related dependencies. ([#362](https://github.com/heroku/libcnb.rs/pull/362))
- Use `--pull-policy if-not-present` when running `pack build`. ([#373](https://github.com/heroku/libcnb.rs/pull/373))
- Allow starting a container without using the default process type. Removed `PrepareContainerContext::start`, added `PrepareContainerContext::start_with_default_process`, `PrepareContainerContext::start_with_default_process_args`, `PrepareContainerContext::start_with_process`, `PrepareContainerContext::start_with_process_args`, `PrepareContainerContext::start_with_shell_command`. ([#366](https://github.com/heroku/libcnb.rs/pull/366))
- Add `ContainerContext::logs_now` and `ContainerContext::logs_wait` to access the logs of the container. Useful when used in conjunction with the new `PrepareContainerContext::start_with_shell_command` method to get the shell command output. ([#366](https://github.com/heroku/libcnb.rs/pull/366))
- Replaced `container_context::ContainerExecResult` with `log::LogOutput` which is now also returned by `ContainerContext::logs` and `ContainerContext::logs_follow`. ([#366](https://github.com/heroku/libcnb.rs/pull/366))
- Move support for connecting to the Docker daemon over TLS behind a new `remote-docker` feature flag, since remote Docker support is not fully implemented and pulls in many additional dependencies. ([#376](https://github.com/heroku/libcnb.rs/pull/376))

## [0.2.0] 2022-02-28

- `libcnb-test` now cross-compiles and packages all binary targets of the buildpack for an integration test. The main buildpack binary is either the only binary target or the target with the same name as the crate. This feature allows the usage of additional binaries for i.e. `exec.d`. ([#314](https://github.com/heroku/libcnb.rs/pull/314))
- Increase minimum supported Rust version from 1.56 to 1.58. ([#318](https://github.com/heroku/libcnb.rs/pull/318))
- Add `assert_contains!` macro for easier matching of `pack` output in integration tests. ([#322](https://github.com/heroku/libcnb.rs/pull/322))
- Fail tests early with a clearer error message, if expected cross-compilation toolchains are not found. ([#347](https://github.com/heroku/libcnb.rs/pull/347))
- Update `libcnb-cargo` from `0.2.1` to `0.3.0` - see the [libcnb-cargo changelog](../libcnb-cargo/CHANGELOG.md#030-2022-02-28). ([#361](https://github.com/heroku/libcnb.rs/pull/361))

## [0.1.1] 2022-02-04

- Use the `DOCKER_HOST` environment variable to determine the Docker connection strategy, adding support for HTTPS connections in addition to local UNIX sockets. This enables using `libcnb-test` in more complex setups like on CircleCI where the Docker daemon is on a remote machine. ([#305](https://github.com/heroku/libcnb.rs/pull/305))

## [0.1.0] 2022-02-02

- Initial release. ([#277](https://github.com/heroku/libcnb.rs/pull/277))
