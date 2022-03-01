# Changelog

## [Unreleased]

- Add `IntegrationTest::env` and `IntegrationTest::envs`, allowing users to set environment variables for the build process. ([#346](https://github.com/Malax/libcnb.rs/pull/346))
- Replaced `IntegrationTestContext::start_container` with `IntegrationTestContext::prepare_container`, allowing users to configure the container before starting it. Ports can now be exposed via `PrepareContainerContext::expose_port`. ([#346](https://github.com/Malax/libcnb.rs/pull/346))
- Added the ability to set environment variables for the container via `PrepareContainerContext::env` and `PrepareContainerContext::envs`. ([#346](https://github.com/Malax/libcnb.rs/pull/346))

## [0.2.0] 2022-02-28

- `libcnb-test` now cross-compiles and packages all binary targets of the buildpack for an integration test. The main buildpack binary is either the only binary target or the target with the same name as the crate. This feature allows the usage of additional binaries for i.e. execd. ([#314](https://github.com/Malax/libcnb.rs/pull/314))
- Increase minimum supported Rust version from 1.56 to 1.58 ([#318](https://github.com/Malax/libcnb.rs/pull/318)).
- Add `assert_contains!` macro for easier matching of `pack` output in integration tests. ([#322](https://github.com/Malax/libcnb.rs/pull/322))
- Fail tests early with a clearer error message, if expected cross-compilation toolchains are not found ([#347](https://github.com/Malax/libcnb.rs/pull/347).
- Update `libcnb-cargo` from `0.2.1` to `0.3.0` - see the [libcnb-cargo changelog](../libcnb-cargo/CHANGELOG.md#030-2022-02-28). ([#361](https://github.com/Malax/libcnb.rs/pull/361))

## [0.1.1] 2022-02-04

- Use the `DOCKER_HOST` environment variable to determine the Docker connection strategy, adding support for HTTPS 
connections in addition to local UNIX sockets. This enables using `libcnb-test` in more complex setups like on CircleCI 
where the Docker daemon is on a remote machine. ([#305](https://github.com/Malax/libcnb.rs/pull/305))

## [0.1.0] 2022-02-02

- Initial release ([#277](https://github.com/Malax/libcnb.rs/pull/277)).
