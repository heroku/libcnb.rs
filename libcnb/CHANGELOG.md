# Changelog

## [Unreleased]

- Add `must_use` attributes to a number of pure public methods ([#232](https://github.com/Malax/libcnb.rs/pull/232)).
- `TargetLifecycle` has been renamed to `Scope` ([#257](https://github.com/Malax/libcnb.rs/pull/257)).
- Renamed `Buildpack::handle_error` to `Buildpack::on_error` to make it clearer that the error cannot be handled/resolved, just reacted upon ([#266](https://github.com/Malax/libcnb.rs/pull/266)).
- Add `LayerEnv::apply_to_empty` ([#267](https://github.com/Malax/libcnb.rs/pull/267)).

## [0.4.0] 2021-12-08

- Set a minimum required Rust version of 1.56 and switch to the 2021 Rust edition.
- libcnb now targets [Buildpack API 0.6](https://github.com/buildpacks/spec/releases/tag/buildpack%2Fv0.6).
- The `data` module can now be used without the rest of the framework by depending on the `libcnb-data` crate.
- For changes to the `data` module in this release, see the [libcnb-data changelog](../libcnb-data/CHANGELOG.md#030-2021-12-08).
- Introduced `Buildpack` trait that needs to be implemented for each buildpack.
- `cnb_runtime()` now requires a `Buildpack` instead of `detect` and `build` functions.
- `ErrorHandler` has been removed. Functionality is now part of the new `Buildpack` trait.
- `build` now returns `Result<BuildResult, E>` instead of `Result<(), E>`. Construct `BuildResult` values by using the new `BuildResultBuilder`.
- `detect` now returns `DetectResult` instead of a `DetectOutcome` enum. Construct `DetectResult` values by using the new `DetectResultBuilder`.
- `BuildContext#write_launch` was removed. Return a `Launch` value from `build` via `BuildResult` instead.
- `cnb_runtime` was renamed to `libcnb_runtime`.
- Introduced `buildpack_main` macro to initialize the framework.
- Switch to BSD 3-Clause License.
- `Generic*` implementations moved to the `generic` module.
- Remove `PlatformEnv` and replaced it with the already existing `Env`.
- Add `LayerEnv::chainable_insert`
- `LayerEnv` and `ModificationBehavior` now implement `Clone`.
- Made it easier to work with buildpack errors during all phases of a `LayerLifecycle`.
- `LayerEnv` was integrated into the `LayerLifecycle`, allowing buildpack authors to both write environment variables
  in a declarative way and use them between different layers without explicit IO.
- Introduce `LayerName` for layer names to enforce layer name constraints in the CNB specification.
- Layer types are no longer part of create/update in `LayerLifecycle`. They moved up to the layer itself, allowing the
  implementation of implicit layer handling when no update or crate happens.
- New trait design for `LayerLifecycle` which also was renamed to `Layer`.
- Removed low-level layer functions from `BuildContext`. They don't fit well with the design of the library at this
  point and are potential footguns. Implementing a `Layer` should work for all use-cases.
- The `stack_id` field in `BuildContext` and `DetectContext` is now of type `StackId` instead of `String`.
- Remove `Display` trait bound from `Buildpack::Error` type.
- Fixed file extension for delimiters when writing `LayerEnv` to disk.

## [0.3.0] 2021-09-17

- See Git log: [v0.2.0...v0.3.0](https://github.com/Malax/libcnb.rs/compare/v0.2.0...v0.3.0)

## [0.2.0] 2021-08-31

- See Git log: [v0.1.3...v0.2.0](https://github.com/Malax/libcnb.rs/compare/v0.1.3...v0.2.0)

## [0.1.3] 2021-08-06

- See Git log: [v0.1.2...v0.1.3](https://github.com/Malax/libcnb.rs/compare/v0.1.2...v0.1.3)

## [0.1.2] 2021-08-06

- See Git log: [v0.1.1...v0.1.2](https://github.com/Malax/libcnb.rs/compare/v0.1.1...v0.1.2)

## [0.1.1] 2021-05-26

- See Git log: [v0.1.0...v0.1.1](https://github.com/Malax/libcnb.rs/compare/v0.1.0...v0.1.1)

## [0.1.0] 2021-03-18

- Initial release.
