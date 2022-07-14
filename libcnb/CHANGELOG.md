# Changelog

## [Unreleased]

## [0.9.0] 2022-07-14

- Use the crate's `README.md` as the root module's rustdocs, so that all of the crate's documentation can be seen in one place on `docs.rs`. ([#460](https://github.com/heroku/libcnb.rs/pull/460))
- Increase minimum supported Rust version from 1.58 to 1.59. ([#445](https://github.com/heroku/libcnb.rs/pull/445))
- Update `libcnb-data` (which provides the types in the `data` module) from `0.7.0` to `0.8.0` - see the [libcnb-data changelog](../libcnb-data/CHANGELOG.md#080-2022-07-14). ([#465](https://github.com/heroku/libcnb.rs/pull/465))
- Update `libcnb-proc-macros` from `0.2.2` to `0.3.0` - see the [libcnb-proc-macros changelog](../libcnb-proc-macros/CHANGELOG.md#030-2022-07-14). ([#465](https://github.com/heroku/libcnb.rs/pull/465))

## [0.8.0] 2022-06-24

- Make the "Buildpack API version mismatch" check still work when `buildpack.toml` doesn't match the spec or custom buildpack type. ([#421](https://github.com/heroku/libcnb.rs/pull/421))
- Remove support for custom exit codes from `Buildpack::on_error`. Exit codes are part of the CNB spec and there are cases where some exit codes have special meaning to the CNB lifecycle. This put the burden on the buildpack author to not pick exit codes with special meanings, dependent on the currently executing phase. This makes `Buildpack::on_error` more consistent with the rest of the framework where we don't expose the interface between the buildpack and the CNB lifecycle directly but use abstractions for easier forward-compatibility and to prevent accidental misuse. ([#415](https://github.com/heroku/libcnb.rs/pull/415))
- Update `libcnb-data` (which provides the types in the `data` module) from `0.6.0` to `0.7.0` - see the [libcnb-data changelog](../libcnb-data/CHANGELOG.md#070-2022-06-24). ([#432](https://github.com/heroku/libcnb.rs/pull/432))

## [0.7.0] 2022-04-12

- Allow compilation of libcnb.rs buildpacks on Windows. Please note that this does not imply Windows container support, it's meant to allow running unit tests without cross-compiling. ([#368](https://github.com/heroku/libcnb.rs/pull/368))
- Expose `runtime::libcnb_runtime_detect`, `runtime::libcnb_runtime_build` and their related types for advanced use-cases. Buildpack authors should not use these. ([#375](https://github.com/heroku/libcnb.rs/pull/375))
- Only create layer `env`, `env.build` and `env.launch` directories when environment variables are being set within them. ([#385](https://github.com/heroku/libcnb.rs/pull/385))
- Add `WriteLayerError::MissingExecDFile` error to ease debugging when an exec.d path is missing. ([#387](https://github.com/heroku/libcnb.rs/pull/387))
- Update project URLs for the GitHub repository move to the `heroku` org. ([#388](https://github.com/heroku/libcnb.rs/pull/388))
- Update `libcnb-data` (which provides the types in the `data` module) from `0.5.0` to `0.6.0` - see the [libcnb-data changelog](../libcnb-data/CHANGELOG.md#060-2022-04-12). ([#391](https://github.com/heroku/libcnb.rs/pull/391))

## [0.6.0] 2022-02-28

- Add `#[must_use]` to `DetectResult`, `DetectResultBuilder`, `PassDetectResultBuilder`, `FailDetectResultBuilder`, `BuildResult` and `BuildResultBuilder`. ([#288](https://github.com/heroku/libcnb.rs/pull/288))
- Add `additional_buildpack_binary_path!` macro to resolve paths to additional buildpack binaries. Only works when the buildpack is packaged with `libcnb-cargo`/`libcnb-test`. ([#320](https://github.com/heroku/libcnb.rs/pull/320))
- Increase minimum supported Rust version from 1.56 to 1.58. ([#318](https://github.com/heroku/libcnb.rs/pull/318))
- Add support for exec.d programs in layers. Use `LayerResultBuilder::exec_d_program` to add exec.d programs to a layer. ([#326](https://github.com/heroku/libcnb.rs/pull/326))
- Add `libcnb::exec_d::write_exec_d_program_output` which writes `libcnb::data::exec_d::ExecDProgramOutput` in a spec conforming way. Use this to implement custom exec.d programs for your buildpack with libcnb.rs. (see [exec.d example](../examples/execd)). ([#326](https://github.com/heroku/libcnb.rs/pull/326))
- Update `libcnb-data` (which provides the types in the `data` module) from `0.4.0` to `0.5.0` - see the [libcnb-data changelog](../libcnb-data/CHANGELOG.md#050-2022-02-28). ([#361](https://github.com/heroku/libcnb.rs/pull/361))
- Update `libcnb-proc-macros` from `0.1.1` to `0.2.0` - see the [libcnb-proc-macros changelog](../libcnb-proc-macros/CHANGELOG.md#020-2022-02-28). ([#361](https://github.com/heroku/libcnb.rs/pull/361))

## [0.5.0] 2022-01-14

- Add `must_use` attributes to a number of pure public methods. ([#232](https://github.com/heroku/libcnb.rs/pull/232))
- `TargetLifecycle` has been renamed to `Scope`. ([#257](https://github.com/heroku/libcnb.rs/pull/257))
- Renamed `Buildpack::handle_error` to `Buildpack::on_error` to make it clearer that the error cannot be handled/resolved, just reacted upon. ([#266](https://github.com/heroku/libcnb.rs/pull/266))
- Add `LayerEnv::apply_to_empty`. ([#267](https://github.com/heroku/libcnb.rs/pull/267))
- Bump external dependency versions. ([#233](https://github.com/heroku/libcnb.rs/pull/233) and [#275](https://github.com/heroku/libcnb.rs/pull/275))
- Update `libcnb-data` (which provides the types in the `data` module) from `0.3.0` to `0.4.0` - see the [libcnb-data changelog](../libcnb-data/CHANGELOG.md#040-2022-01-14). ([#276](https://github.com/heroku/libcnb.rs/pull/276))

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
  point and are potential foot-guns. Implementing a `Layer` should work for all use-cases.
- The `stack_id` field in `BuildContext` and `DetectContext` is now of type `StackId` instead of `String`.
- Remove `Display` trait bound from `Buildpack::Error` type.
- Fixed file extension for delimiters when writing `LayerEnv` to disk.

## [0.3.0] 2021-09-17

- See Git log: [libcnb/v0.2.0...libcnb/v0.3.0](https://github.com/heroku/libcnb.rs/compare/libcnb/v0.2.0...libcnb/v0.3.0)

## [0.2.0] 2021-08-31

- See Git log: [libcnb/v0.1.3...libcnb/v0.2.0](https://github.com/heroku/libcnb.rs/compare/libcnb/v0.1.3...libcnb/v0.2.0)

## [0.1.3] 2021-08-06

- See Git log: [libcnb/v0.1.2...libcnb/v0.1.3](https://github.com/heroku/libcnb.rs/compare/libcnb/v0.1.2...libcnb/v0.1.3)

## [0.1.2] 2021-08-06

- See Git log: [libcnb/v0.1.1...libcnb/v0.1.2](https://github.com/heroku/libcnb.rs/compare/libcnb/v0.1.1...libcnb/v0.1.2)

## [0.1.1] 2021-05-26

- See Git log: [libcnb/v0.1.0...libcnb/v0.1.1](https://github.com/heroku/libcnb.rs/compare/libcnb/v0.1.0...libcnb/v0.1.1)

## [0.1.0] 2021-03-18

- Initial release.
