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
- `ErrorHandler` has been removed. Functionality is now part of the new `Buildpack` trait.
- `build` now returns `Result<BuildResult, E>` instead of `Result<(), E>`. Construct `BuildResult` values by using the new `BuildResultBuilder`.
- `detect` now returns `DetectResult` instead of a `DetectOutcome` enum. Construct `DetectResult` values by using the new `DetectResultBuilder`.
- `BuildContext#write_launch` was removed. Return a `Launch` value from `build` via `BuildResult` instead.
- `cnb_runtime` was renamed to `libcnb_runtime`.
- Introduced `buildpack_main` macro to initialize the framework.
- Switch to BSD 3-Clause License.
- `Generic*` implementations moved to the `generic` module.
- `LayerContentTypeTable` has been renamed to `LayerTypes`.
- Remove `PlatformEnv` and replaced it with the already existing `Env`.
- `StackId`, `ProcessType` and `BuildpackId` now implement `Deref<Target = String>`, `Borrow<String>`, `AsRef<String>` and `Display`.
- Add a more general `Default` implementation for `LayerContentMetadata`.
- Add `PartialEq` implementation for `LayerContentMetadata`.
- Add `PartialEq` and `Eq` implementations for `LayerTypes`.
- Add `LayerEnv::chainable_insert`
- `LayerEnv` and `ModificationBehavior` now implement `Clone`.
- Add `stack_id!`, `buildpack_id!` and `process_type!` macros.
- `Process::new` no longer returns a `Result` and it's `type` argument now is of type `ProcessType`. 

## [0.3.0] 2021/09/17
