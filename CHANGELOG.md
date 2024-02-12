# Changelog

This is the new, unified, changelog that contains changes from across all libcnb.rs crates. Before version `0.10.0`,
separate changelogs for each crate were used. If you need to refer to these old changelogs, find them named
`HISTORICAL_CHANGELOG.md` in their respective crate directories.

The format is based on [Keep a Changelog](https://keepachangelog.com/en/1.1.0/),
and this project adheres to [Semantic Versioning](https://semver.org/spec/v2.0.0.html).

## [Unreleased]


## [0.18.0] - 2024-02-12

### Changed

- Now targets [Buildpack API 0.10](https://github.com/buildpacks/spec/releases/tag/buildpack%2Fv0.10). Buildpacks need to upgrade the `api` key to `0.10` in their `buildpack.toml`. ([#773](https://github.com/heroku/libcnb.rs/pull/773))
- Improved the consistency of cross-compilation assistance provided across all supported `target_triple` and host OS/architecture combinations. [#769](https://github.com/heroku/libcnb.rs/pull/769)
- Added cross-compilation assistance for `aarch64-unknown-linux-musl` (on macOS and ARM64 Linux) and `x86_64-unknown-linux-musl` (on ARM64 Linux). [#769](https://github.com/heroku/libcnb.rs/pull/769)
- Raised Minimum Supported Rust Version (MSRV) to `1.76`. ([#774](https://github.com/heroku/libcnb.rs/pull/774))
- `libcnb`:
  - Changed `Layer` interface from `&self` to `&mut self`. ([#669](https://github.com/heroku/libcnb.rs/pull/669))

### Added

- `libherokubuildpack`:
  - `MappedWrite::unwrap` for getting the wrapped `Write` back out. ([#765](https://github.com/heroku/libcnb.rs/pull/765))

### Removed

- Types, errors, macros and functions related to stacks. The concept of stacks has been removed from the CNB spec. Use `Target` instead. ([#773](https://github.com/heroku/libcnb.rs/pull/773))

## [0.17.0] - 2023-12-06

### Added

- `libcnb`:
  - An optional `trace` feature has been added that emits OpenTelemetry tracing
    data to a [File Export](https://opentelemetry.io/docs/specs/otel/protocol/file-exporter/). ([#723](https://github.com/heroku/libcnb.rs/pull/723))

## [0.16.0] - 2023-11-17

### Changed

- Raised Minimum Supported Rust Version (MSRV) to `1.74`. ([#747](https://github.com/heroku/libcnb.rs/pull/747))
- Improved the consistency of all user-facing libcnb.rs error message wordings. ([#722](https://github.com/heroku/libcnb.rs/pull/722))
- The assistance error message shown when the necessary cross-compilation tools are not found now also includes the `rustup target add` step. ([#729](https://github.com/heroku/libcnb.rs/pull/729))
- Updated the documentation for `TestRunner::build` and `TestContext::start_container` to mention when Docker resource teardown occurs. ([#743](https://github.com/heroku/libcnb.rs/pull/743))

### Fixed

- `libcnb-test`:
  - Fixed incorrect error messages being shown for buildpack compilation/packaging failures. ([#720](https://github.com/heroku/libcnb.rs/pull/720))
  - The Docker volumes created by Pack for the build and launch layer caches are now cleaned up after each test. ([#741](https://github.com/heroku/libcnb.rs/pull/741))
  - The Docker image cleanup process no longer makes duplicate attempts to remove images when using `TestContext::rebuild`. ([#741](https://github.com/heroku/libcnb.rs/pull/741))
  - Test failures due to the Docker daemon not being installed or started no longer cause a non-unwinding panic abort with noisy traceback. ([#741](https://github.com/heroku/libcnb.rs/pull/741))
  - Containers created by `TestContext::start_container` are now correctly cleaned up if the container failed to start. ([#742](https://github.com/heroku/libcnb.rs/pull/742))

## [0.15.0] - 2023-09-25

### Added

- `libcnb`:
  - `LayerTypes` now implements `Copy` and `Clone`. ([#670](https://github.com/heroku/libcnb.rs/pull/670)).
- `libcnb-data`:
  - `ExecDProgramOutputKey`, `ProcessType`, `LayerName`, `BuildpackId` and `StackId` now implement `Ord` and `PartialOrd`. ([#658](https://github.com/heroku/libcnb.rs/pull/658))
  - Added `generic::GenericMetadata` as a generic metadata type. Also makes it the default for `BuildpackDescriptor`, `SingleBuildpackDescriptor`, `CompositeBuildpackDescriptor` and `LayerContentMetadata`. ([#664](https://github.com/heroku/libcnb.rs/pull/664))
- `libcnb-test`:
  - Added the `BuildpackReference::WorkspaceBuildpack` enum variant. This allows for the testing of any libcnb.rs or composite buildpack in the Cargo workspace, instead of only the buildpack of the current crate. **Note: The testing of composite buildpacks requires `pack` CLI version `>=0.30`.** ([#666](https://github.com/heroku/libcnb.rs/pull/666))

### Changed

- `libcnb-data`:
  - Renamed the `buildpackage` module to `package_descriptor`, and the `Buildpackage*` types within it to `PackageDescriptor*`. ([#656](https://github.com/heroku/libcnb.rs/pull/656))
  - Renamed multiple types to match the new composite vs component buildpack [upstream terminology](https://github.com/buildpacks/spec/blob/main/buildpack.md#cnb-terminology). Renamed `SingleBuildpackDescriptor` to `ComponentBuildpackDescriptor`, `MetaBuildpackDescriptor` to `CompositeBuildpackDescriptor` and `BuildpackDescriptor::{Single,Meta}` to `BuildpackDescriptor::{Component,Composite}`. ([#682](https://github.com/heroku/libcnb.rs/pull/682))
- `libcnb-cargo`:
  - No longer outputs paths for non-libcnb.rs and non-meta buildpacks. ([#657](https://github.com/heroku/libcnb.rs/pull/657))
  - Build output for humans changed slightly, output intended for machines/scripting didn't change. ([#657](https://github.com/heroku/libcnb.rs/pull/657))
  - When performing buildpack detection, standard ignore files (`.ignore` and `.gitignore`) will be respected. ([#673](https://github.com/heroku/libcnb.rs/pull/673))
- `libcnb-test`:
  - Renamed `BuildpackReference::Crate` to `BuildpackReference::CurrentCrate`. ([#666](https://github.com/heroku/libcnb.rs/pull/666))

## [0.14.0] - 2023-08-18

### Added

- `libcnb-package`: Added cross-compilation assistance for Linux `aarch64-unknown-linux-musl`. ([#577](https://github.com/heroku/libcnb.rs/pull/577))
- `libcnb-cargo`: Added `--package-dir` command line option to control where packaged buildpacks are written. ([#583](https://github.com/heroku/libcnb.rs/pull/583))
- `libcnb-test`:
  - `LogOutput` now implements `std::fmt::Display`. ([#635](https://github.com/heroku/libcnb.rs/pull/635))
  - `ContainerConfig` now implements `Clone`. ([#636](https://github.com/heroku/libcnb.rs/pull/636))

### Changed

- `libcnb-cargo`: Moved the default location for packaged buildpacks from Cargo's `target/` directory to `packaged/` in the Cargo workspace root. This simplifies the path and stops modification of the `target/` directory which previously might have caching implications when other tools didn't expect non-Cargo output in that directory. Users that implicitly rely on the output directory need to adapt. The output of `cargo libcnb package` will refer to the new locations. ([#583](https://github.com/heroku/libcnb.rs/pull/583))
- `libcnb-package`:
  - buildpack target directory now contains the target triple. Users that implicitly rely on the output directory need to adapt. The output of `cargo libcnb package` will refer to the new locations. ([#580](https://github.com/heroku/libcnb.rs/pull/580))
  - `get_buildpack_target_dir` was renamed to `get_buildpack_package_dir` ([#583](https://github.com/heroku/libcnb.rs/pull/583))
- `libcnb-test`:
  - `ContainerContext::address_for_port` will now panic for all failure modes rather than just some, and so now returns `SocketAddr` directly instead of `Option<SocketAddr>`. This reduces test boilerplate due to the caller no longer needing to `.unwrap()` and improves debugging UX when containers crash after startup. ([#605](https://github.com/heroku/libcnb.rs/pull/605) and [#636](https://github.com/heroku/libcnb.rs/pull/636))
  - Docker commands are now run using the Docker CLI instead of Bollard and the Docker daemon API. ([#620](https://github.com/heroku/libcnb.rs/pull/620))
  - `ContainerConfig::entrypoint` now accepts a String rather than a vector of strings. Any arguments to the entrypoint should be moved to `ContainerConfig::command`. ([#620](https://github.com/heroku/libcnb.rs/pull/620))
  - Removed `TestRunner::new` since its only purpose was for advanced configuration that's no longer applicable. Use `TestRunner::default` instead. ([#620](https://github.com/heroku/libcnb.rs/pull/620))
  - Removed `stdout_raw` and `stderr_raw` from `LogOutput`. ([#607](https://github.com/heroku/libcnb.rs/pull/607))
  - Improved wording of panic error messages. ([#619](https://github.com/heroku/libcnb.rs/pull/619) and [#620](https://github.com/heroku/libcnb.rs/pull/620))
- `libherokubuildpack`: Changed the `flate2` decompression backend from `miniz_oxide` to `zlib`. ([#593](https://github.com/heroku/libcnb.rs/pull/593))

### Fixed

- `libcnb-test`:
  - `TestContext::run_shell_command` and `ContainerContext::shell_exec` now validate the exit code of the spawned commands and panic if they are non-zero. ([#620](https://github.com/heroku/libcnb.rs/pull/620))
  - `ContainerContext::expose_port` now only exposes the port to localhost. ([#610](https://github.com/heroku/libcnb.rs/pull/610))
  - If a test with an expected result of `PackResult::Failure` unexpectedly succeeds, the built app image is now correctly cleaned up. ([#625](https://github.com/heroku/libcnb.rs/pull/625))

## [0.13.0] - 2023-06-21

The highlight of this release is the `cargo libcnb package` changes to support compilation of both buildpacks and meta-buildpacks.

### Changed

- `libcnb-cargo`: The `cargo libcnb package` command now supports compiling buildpacks and meta-buildpacks ([#575](https://github.com/heroku/libcnb.rs/pull/575)):
  - When used in a buildpack directory it will compile only that buildpack.
  - When used in a workspace directory it will compile all buildpacks found in subdirectories.
- `libcnb-package`: Changed `default_buildpack_directory_name` to accept a `BuildpackId` ([#575](https://github.com/heroku/libcnb.rs/pull/575))

### Added

- `libcnb-cargo`
  - Buildpacks can reference other buildpacks within a workspace by using `uri = "libcnb:{buildpack_id}"` as a dependency entry in the buildpack's [package.toml](https://buildpacks.io/docs/reference/config/package-config/) file. ([#575](https://github.com/heroku/libcnb.rs/pull/575))
- `libcnb-data`
  - Serialization / deserialization of [package.toml](https://buildpacks.io/docs/reference/config/package-config/) files supported with the `Buildpackage` struct. ([#575](https://github.com/heroku/libcnb.rs/pull/575))
- `libcnb-package`
  - Added
    `read_buildpackage_data`,
    `find_buildpack_dirs`,
    `get_buildpack_target_dir`
    to support packaging operations. ([#575](https://github.com/heroku/libcnb.rs/pull/575))
  - Added
    `buildpack_dependency::BuildpackDependency`,
    `buildpack_dependency::get_local_buildpackage_dependencies`,
    `buildpack_dependency::rewrite_buildpackage_local_dependencies`,
    `buildpack_dependency::rewrite_buildpackage_relative_path_dependencies_to_absolute`
    to support Buildpack dependency handling and packaging operations. ([#575](https://github.com/heroku/libcnb.rs/pull/575))
  - Added
    `buildpack_package::BuildpackPackage`,
    `buildpack_package::read_buildpack_package`
    to support libcnb.rs-based Rust packages. ([#575](https://github.com/heroku/libcnb.rs/pull/575))
  - Added
    `dependency_graph::DependencyNode`,
    `dependency_graph::create_dependency_graph`,
    `dependency_graph::get_dependencies`
    to support dependency ordering and resolution in libcnb.rs-based Rust packages. ([#575](https://github.com/heroku/libcnb.rs/pull/575))

## [0.12.0] - 2023-04-28

Highlight of this release is the bump to [Buildpack API 0.9](https://github.com/buildpacks/spec/releases/tag/buildpack%2Fv0.9). This release contains breaking changes, please refer to the items below for migration advice.

### Changed

- libcnb.rs now targets [Buildpack API 0.9](https://github.com/buildpacks/spec/releases/tag/buildpack%2Fv0.9). Buildpacks need to upgrade the `api` key to `0.9` in their `buildpack.toml`. ([#567](https://github.com/heroku/libcnb.rs/pull/567))
  - `Process` no longer supports the `direct` flag. All processes are now `direct`. Processes that need to use bash can use bash explicitly in the command. ([#567](https://github.com/heroku/libcnb.rs/pull/567))
  - `Process::command` has been changed to a sequence of values where the first one is the executable and any additional values are arguments to the executable. The already existing `args` field behaves slightly different now as its contents can now be overridden by the user. See the [upstream CNB specification](https://github.com/buildpacks/spec/blob/buildpack/v0.9/buildpack.md#launchtoml-toml) for details. ([#567](https://github.com/heroku/libcnb.rs/pull/567))
- `Env::get` now returns `Option<&OsString>` instead of `Option<OsString>`. This is more in line with expectations users have when dealing with a collection type. This is a breaking change, compile errors can be fixed by adding a [`Option::cloned`](https://doc.rust-lang.org/std/option/enum.Option.html#method.cloned-1) call after `Env::get` to get the old behaviour. In some cases, cloning might not be necessary, slightly improving the code that uses `Env::get`. ([#565](https://github.com/heroku/libcnb.rs/pull/565))

### Added

- `Env::get_string_lossy` as a convenience method to work with environment variables directly. Getting a value out of an `Env` and treating its contents as unicode is a common case. Using this new method can simplify buildpack code. ([#565](https://github.com/heroku/libcnb.rs/pull/565))
- `Clone` implementation for `libcnb::layer_env::Scope`. ([#566](https://github.com/heroku/libcnb.rs/pull/566))

## [0.11.5] - 2023-02-07

### Changed

- Update `toml` to `0.7.1`. If your buildpack interacts with TOML data directly, you probably want to bump
the `toml` version in your buildpack as well. ([#556](https://github.com/heroku/libcnb.rs/pull/556))

## [0.11.4] - 2023-01-11

### Added

- libcnb-data: Store struct now supports `clone()` and `default()`. ([#547](https://github.com/heroku/libcnb.rs/pull/547))

## [0.11.3] - 2023-01-09

### Added

- libcnb: Add `store` field to `BuildContext`, exposing the contents of `store.toml` if present. ([#543](https://github.com/heroku/libcnb.rs/pull/543))

## [0.11.2] - 2022-12-15

### Fixed

- libcnb-test: `TestContext::download_sbom_files` now checks the exit code of the `pack sbom download` command it runs. ([#520](https://github.com/heroku/libcnb.rs/pull/520))

### Changed

- libcnb: Drop the use of the `stacker` crate when recursively removing layer directories. ([#517](https://github.com/heroku/libcnb.rs/pull/517))
- libcnb-cargo: Updated to Clap v4. ([#511](https://github.com/heroku/libcnb.rs/pull/511))

## Added

- libherokubuildpack: Add `command` and `write` modules for working with `std::process::Command` output streams. ([#535](https://github.com/heroku/libcnb.rs/pull/535))

## [0.11.1] - 2022-09-29

### Fixed

- All crates now properly include the `LICENSE` file. ([#506](https://github.com/heroku/libcnb.rs/pull/506))
- Fix `libcnb` readme file metadata which prevented vendoring `libcnb` via `cargo vendor`. ([#506](https://github.com/heroku/libcnb.rs/pull/506))

### Changed

- Improve the `libherokubuildpack` root module rustdocs. ([#503](https://github.com/heroku/libcnb.rs/pull/503))

## [0.11.0] - 2022-09-23

### Changed

- Bump Minimum Supported Rust Version (MSRV) to `1.64`. ([#500](https://github.com/heroku/libcnb.rs/pull/500))
- Bump minimum external dependency versions. ([#502](https://github.com/heroku/libcnb.rs/pull/502))

### Added

- Add new crate `libherokubuildpack` with common code that can be useful when implementing buildpacks with libcnb. Originally hosted in a separate, private, repository. Code from `libherokubuildpack` might eventually find its way into libcnb.rs proper. At this point, consider it an incubator. ([#495](https://github.com/heroku/libcnb.rs/pull/495))

## [0.10.0] - 2022-08-31

Highlight of this release is the bump to
[Buildpack API 0.8](https://github.com/buildpacks/spec/releases/tag/buildpack%2Fv0.8) which brings support for SBOM to
libcnb.rs. This is also the first release where all libcnb.rs crates are released at the same time and with the same
version number. See the changelog below for other changes.

### Changed

- libcnb.rs now targets [Buildpack API 0.8](https://github.com/buildpacks/spec/releases/tag/buildpack%2Fv0.8). Buildpacks need to upgrade the `api` key to `0.8` in their `buildpack.toml`. ([#489](https://github.com/heroku/libcnb.rs/pull/489))
- In accordance to the CNB specification `>=0.7`, `BuildpackId` no longer permits `sbom` as a buildpack id. ([#489](https://github.com/heroku/libcnb.rs/pull/489))
- Replace builder style functions from `Launch` with a dedicated `LaunchBuilder` to be more consistent with other builders in the library. Additionally, all fields of `Launch` can now be modified via the builder pattern. ([#487](https://github.com/heroku/libcnb.rs/pull/487))
- Rename `paths` field in `launch::Slice` to `path_globs` and add docs to make it clearer that these strings are Go standard library globs. ([#487](https://github.com/heroku/libcnb.rs/pull/487))
- Add explicit `DeleteLayerError` to provide more context when debugging layer handling problems. ([#488](https://github.com/heroku/libcnb.rs/pull/488))

### Fixed

- Fix `BuildpackApi` to use `u64` instead of `u32` for major and minor version parts. ([#489](https://github.com/heroku/libcnb.rs/pull/489))
- Fix permission issues during layer handling when the layer contains read-only directories. ([#488](https://github.com/heroku/libcnb.rs/pull/488))

### Added

- Add `BuildResultBuilder::build_sbom`, `BuildResultBuilder::launch_sbom` and `LayerResultBuilder::sbom` to enable buildpack authors to attach SBOM data for layers and launch. ([#489](https://github.com/heroku/libcnb.rs/pull/489))
- Add `sbom::SbomFormat`, describing supported SBOM formats. ([#489](https://github.com/heroku/libcnb.rs/pull/489))
- Add `Buildpack::sbom_formats` field. ([#489](https://github.com/heroku/libcnb.rs/pull/489))
- Add support for setting a working directory for launch processes. ([#489](https://github.com/heroku/libcnb.rs/pull/489))
- Add `TestContext::download_sbom_files` to allow testing of SBOM logic. ([#489](https://github.com/heroku/libcnb.rs/pull/489))

### Removed

- Remove support for legacy BOM. Remove `Launch::bom`, `Build::bom`, `bom::Bom`, `bom::Entry`. ([#489](https://github.com/heroku/libcnb.rs/pull/489))

[unreleased]: https://github.com/heroku/libcnb.rs/compare/v0.18.0...HEAD
[0.18.0]: https://github.com/heroku/libcnb.rs/compare/v0.17.0...v0.18.0
[0.17.0]: https://github.com/heroku/libcnb.rs/compare/v0.16.0...v0.17.0
[0.16.0]: https://github.com/heroku/libcnb.rs/compare/v0.15.0...v0.16.0
[0.15.0]: https://github.com/heroku/libcnb.rs/compare/v0.14.0...v0.15.0
[0.14.0]: https://github.com/heroku/libcnb.rs/compare/v0.13.0...v0.14.0
[0.13.0]: https://github.com/heroku/libcnb.rs/compare/v0.12.0...v0.13.0
[0.12.0]: https://github.com/heroku/libcnb.rs/compare/v0.11.5...v0.12.0
[0.11.5]: https://github.com/heroku/libcnb.rs/compare/v0.11.4...v0.11.5
[0.11.4]: https://github.com/heroku/libcnb.rs/compare/v0.11.3...v0.11.4
[0.11.3]: https://github.com/heroku/libcnb.rs/compare/v0.11.2...v0.11.3
[0.11.2]: https://github.com/heroku/libcnb.rs/compare/v0.11.1...v0.11.2
[0.11.1]: https://github.com/heroku/libcnb.rs/compare/v0.11.0...v0.11.1
[0.11.0]: https://github.com/heroku/libcnb.rs/compare/v0.10.0...v0.11.0
[0.10.0]: https://github.com/heroku/libcnb.rs/compare/libcnb/v0.9.0...v0.10.0
