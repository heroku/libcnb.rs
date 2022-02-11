# Changelog

## [Unreleased]

- `libcnb-test` now cross-compiles and packages all binary targets of the buildpack for an integration test. The main 
buildpack binary is either the only binary target or the target with the same name as the crate. If a user needs to explicitly
configure the main buildpack binary, `libcnb.buildpack-target` can be set in `Cargo.toml` metadata.
This feature allows the usage of additional binaries for i.e. execd. ([#314](https://github.com/Malax/libcnb.rs/pull/314))

## [0.1.1] 2022-02-04

- Use the `DOCKER_HOST` environment variable to determine the Docker connection strategy, adding support for HTTPS 
connections in addition to local UNIX sockets. This enables using `libcnb-test` in more complex setups like on CircleCI 
where the Docker deamon is on a remote machine. ([#305](https://github.com/Malax/libcnb.rs/pull/305))

## [0.1.0] 2022-02-02

- Initial release ([#277](https://github.com/Malax/libcnb.rs/pull/277)).
