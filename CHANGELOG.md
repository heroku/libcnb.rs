# Changelog

## [Unreleased]

- Add support for `default` key in `launch.toml` in `Process` struct
- Support the new `buildpack.toml` fields `description`, `keywords` and `licenses`
- Set a minumim required Rust version of 1.56 and switch to the 2021 Rust edition
- Stack id in `buildpack.toml` can now be `*` indicating "any" stack
- LayerContentMetadata values (build, cache, launch) are now under a "types" key
- Allow ProcessType to contain a dot (`.`) character
- libcnb now targets [Buildpack API 0.6](https://github.com/buildpacks/spec/releases/tag/buildpack%2Fv0.6) <https://github.com/Malax/libcnb.rs/milestone/2>
- The `data` module can now be used without the rest of the framework by depending on the `libcnb-data` crate.
- Introduced `Buildpack` trait that needs to be implemented for each buildpack
- `cnb_runtime()` now requires a `Buildpack` instead of `detect` and `build` functions.
- `ErrorHandler` has been removed. Functionality is not part of the new `Buildpack` type.

## [0.3.0] 2021/09/17
