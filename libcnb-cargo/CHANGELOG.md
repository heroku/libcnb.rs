# Changelog

## [Unreleased]

## [0.2.0] 2022-01-14

- `BuildpackData`, `assemble_buildpack_directory()` and `default_buildpack_directory_name()` have been updated for the libcnb-data replacement of `BuildpackToml` with `*BuildpackDescriptor` and rename of `*buildpack_toml` to `*buildpack_descriptor` ([#248](https://github.com/Malax/libcnb.rs/pull/248) and [#254](https://github.com/Malax/libcnb.rs/pull/254)).
- Bump external dependency versions ([#233](https://github.com/Malax/libcnb.rs/pull/233)).
- Update `libcnb-data` from `0.3.0` to `0.4.0` - see the [libcnb-data changelog](../libcnb-data/CHANGELOG.md#040-2022-01-14) ([#276](https://github.com/Malax/libcnb.rs/pull/276)).

## [0.1.0] 2021-12-08

- Add a Cargo command for cross-compiling and packaging libcnb buildpacks ([#199](https://github.com/Malax/libcnb.rs/pull/199)).
