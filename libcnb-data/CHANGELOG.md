# Changelog

## [Unreleased]

- Replace builder style functions from `Launch` with a dedicated `LaunchBuilder` to be more consistent with other builders in the library. Additionally, all fields of `Launch` can now be modified via the builder pattern. ([#487](https://github.com/heroku/libcnb.rs/pull/487))
- Rename `paths` field in `launch::Slice` to `path_globs` and add docs to make it clearer that these strings are Go standard library globs. ([#487](https://github.com/heroku/libcnb.rs/pull/487))

## [0.8.0] 2022-07-14

- Disable `fancy-regex` default features (such as unused unicode support) to reduce buildpack binary size. ([#439](https://github.com/heroku/libcnb.rs/pull/439))
- Add a crate `README.md` and use it as the root module's rustdocs, so that all of the crate's documentation can be seen in one place on `docs.rs`. ([#458](https://github.com/heroku/libcnb.rs/pull/458) and [#460](https://github.com/heroku/libcnb.rs/pull/460))
- Increase minimum supported Rust version from 1.58 to 1.59. ([#445](https://github.com/heroku/libcnb.rs/pull/445))
- Update `libcnb-proc-macros` from `0.2.2` to `0.3.0` - see the [libcnb-proc-macros changelog](../libcnb-proc-macros/CHANGELOG.md#030-2022-07-14). ([#465](https://github.com/heroku/libcnb.rs/pull/465))

# [0.7.0] 2022-06-24

- Add `Launch::processes` function to add multiple `Process`es to a `Launch` value at once. ([#418](https://github.com/heroku/libcnb.rs/pull/418))
- `Launch::process`'s argument changed from `Process` to `Into<Process>`. ([#418](https://github.com/heroku/libcnb.rs/pull/418))
- Update `fancy-regex` dependency from `0.8.0` to `0.10.0`. ([#393](https://github.com/heroku/libcnb.rs/pull/393) and [#394](https://github.com/heroku/libcnb.rs/pull/394))

## [0.6.0] 2022-04-12

- Make `BuildPlan`'s `or` field public. ([#381](https://github.com/heroku/libcnb.rs/pull/381))
- Add way to construct `Require` with metadata field and integrate with `BuildPlanBuilder`. ([#382](https://github.com/heroku/libcnb.rs/pull/382))
- Add way to deserialize `Entry` metadata into a custom type. ([#382](https://github.com/heroku/libcnb.rs/pull/382))
- Update project URLs for the GitHub repository move to the `heroku` org. ([#388](https://github.com/heroku/libcnb.rs/pull/388))

## [0.5.0] 2022-02-28

- Add `#[must_use]` to `BuildPlan` and `BuildPlanBuilder`. ([#288](https://github.com/heroku/libcnb.rs/pull/288))
- Add `exec_d` module with types representing the output of an `exec.d` program. ([#324](https://github.com/heroku/libcnb.rs/pull/324))
- Increase minimum supported Rust version from 1.56 to 1.58. ([#318](https://github.com/heroku/libcnb.rs/pull/318))
- Adjust newtype generated compile-time validation macros so that they don't also perform redundant validation at runtime. In cases where only compile-time validation is being performed (for example `exec.d` scripts), this results in a significant reduction in binary size. ([#331](https://github.com/heroku/libcnb.rs/pull/331))
- Update `libcnb-proc-macros` from `0.1.1` to `0.2.0` - see the [libcnb-proc-macros changelog](../libcnb-proc-macros/CHANGELOG.md#020-2022-02-28). ([#361](https://github.com/heroku/libcnb.rs/pull/361))

## [0.4.0] 2022-01-14

- Add `must_use` attributes to a number of pure public methods. ([#232](https://github.com/heroku/libcnb.rs/pull/232))
- Remove builder-style methods from `LayerContentMetadata`. ([#235](https://github.com/heroku/libcnb.rs/pull/235))
- Make `LayerContentMetadata`'s `types` field an `Option`. ([#236](https://github.com/heroku/libcnb.rs/pull/236))
- Remove `LayerContentMetadata::Default()`. ([#236](https://github.com/heroku/libcnb.rs/pull/236))
- Switch `Buildpack.version` from type `semver::Version` to `BuildpackVersion`, in order to validate versions more strictly against the CNB spec. ([#241](https://github.com/heroku/libcnb.rs/pull/241))
- All libcnb-data struct types now reject unrecognised fields when deserializing. ([#252](https://github.com/heroku/libcnb.rs/pull/252))
- `BuildpackToml` has been replaced by `BuildpackDescriptor`, which is an enum with `Single` and `Meta` variants that wrap new `SingleBuildpackDescriptor` and `MetaBuildpackDescriptor` types. The new types now reject `buildpack.toml` files where both `stacks` and `order` are present. ([#248](https://github.com/heroku/libcnb.rs/pull/248))
- Implement `Borrow<str>` for types generated using the `libcnb_newtype!` macro (currently `BuildpackId`, `LayerName`, `ProcessType` and `StackId`), which allows them to be used with `.join()`. ([#258](https://github.com/heroku/libcnb.rs/pull/258))
- `Launch` and `Process` can now be deserialized when optional fields are missing, and omit default values when serializing. ([#243](https://github.com/heroku/libcnb.rs/pull/243) and [#265](https://github.com/heroku/libcnb.rs/pull/265))
- `Process::new` has been replaced by `ProcessBuilder`. ([#265](https://github.com/heroku/libcnb.rs/pull/265))
- Bump external dependency versions. ([#233](https://github.com/heroku/libcnb.rs/pull/233) and [#275](https://github.com/heroku/libcnb.rs/pull/275))
- Update `libcnb-proc-macros` from `0.1.0` to `0.1.1` - see the [libcnb-proc-macros changelog](../libcnb-proc-macros/CHANGELOG.md#011-2022-01-14). ([#276](https://github.com/heroku/libcnb.rs/pull/276))

## [0.3.0] 2021-12-08

- Moved `libcnb`'s data module into a new `libcnb-data` crate.
- Add `PartialEq` and `Eq` implementations for `Process`.
- Add support for `default` key in `launch.toml` in `Process` struct
- Support the new `buildpack.toml` fields `description`, `keywords` and `licenses`
- Stack id in `buildpack.toml` can now be `*` indicating "any" stack
- `LayerContentMetadata` values (build, cache, launch) are now under a `types` field
- Allow `ProcessType` to contain a dot (`.`) character
- `LayerContentTypeTable` has been renamed to `LayerTypes`.
- `StackId`, `ProcessType` and `BuildpackId` now implement `Deref<Target = String>`, `Borrow<String>`, `AsRef<String>` and `Display`.
- Add a more general `Default` implementation for `LayerContentMetadata`.
- Add `PartialEq` implementation for `LayerContentMetadata`.
- Add `PartialEq` and `Eq` implementations for `LayerTypes`.
- Add `stack_id!`, `buildpack_id!` and `process_type!` macros.
- `Process::new` no longer returns a `Result` and it's `type` argument now is of type `ProcessType`.
- Remove `defaults` module.
- `Stack` is now an enum with `Any` and `Specific` variants, rather than a struct.
- `StackId` no longer permits IDs of `*`, use `Stack::Any` instead.
- `BuildpackTomlError::InvalidStarStack` has been replaced by `BuildpackTomlError::InvalidAnyStack`.
- `BuildpackTomlError` has been split into `BuildpackApiError` and `StackError`.
- `BuildpackApi` no longer implements `FromStr`, use `BuildpackApi::try_from()` instead.
- Fixed the `group` field on `buildpack::Order` to now be public.
- `build_plan::Require` fields are now public.
- `buildpack::Buildpack` name field is now an `Option`.
