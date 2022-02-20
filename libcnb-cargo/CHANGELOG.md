# Changelog

## [Unreleased]

- Update cross-compile assistance on macOS to use https://github.com/messense/homebrew-macos-cross-toolchains instead of https://github.com/FiloSottile/homebrew-musl-cross. Support for the latter has not been removed, existing setups continue to work as before. ([#312](https://github.com/Malax/libcnb.rs/pull/312))
- `libcnb-cargo` now cross-compiles and packages all binary targets of the buildpack. The main buildpack binary is either the only binary target or the target with the same name as the crate. This feature allows the usage of additional binaries for i.e. execd. ([#314](https://github.com/Malax/libcnb.rs/pull/314))
- Increase minimum supported Rust version from 1.56 to 1.58 ([#318](https://github.com/Malax/libcnb.rs/pull/318)).
- Upgrade CLI to Clap v3 ([#329](https://github.com/Malax/libcnb.rs/pull/329)).

## [0.2.1] 2022-01-19

- `cargo libcnb package` now allows multiple targets, as long as there is only one binary target. To support this change, the `BuildError` variants `NoTargetsFound` and `MultipleTargetsFound` have been replaced by `NoBinTargetsFound` and `MultipleBinTargetsFound` respectively. ([#282](https://github.com/Malax/libcnb.rs/pull/282))
- `assemble_buildpack_directory()` no longer fails if the output directory already exists. ([#283](https://github.com/Malax/libcnb.rs/pull/283))

## [0.2.0] 2022-01-14

- `BuildpackData`, `assemble_buildpack_directory()` and `default_buildpack_directory_name()` have been updated for the libcnb-data replacement of `BuildpackToml` with `*BuildpackDescriptor` and rename of `*buildpack_toml` to `*buildpack_descriptor` ([#248](https://github.com/Malax/libcnb.rs/pull/248) and [#254](https://github.com/Malax/libcnb.rs/pull/254)).
- Bump external dependency versions ([#233](https://github.com/Malax/libcnb.rs/pull/233)).
- Update `libcnb-data` from `0.3.0` to `0.4.0` - see the [libcnb-data changelog](../libcnb-data/CHANGELOG.md#040-2022-01-14) ([#276](https://github.com/Malax/libcnb.rs/pull/276)).

## [0.1.0] 2021-12-08

- Add a Cargo command for cross-compiling and packaging libcnb buildpacks ([#199](https://github.com/Malax/libcnb.rs/pull/199)).
