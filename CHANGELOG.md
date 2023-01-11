# Changelog

This is the new, unified, changelog that contains changes from across all libcnb.rs crates. Before version `0.10.0`,
separate changelogs for each crate were used. If you need to refer to these old changelogs, find them named
`HISTORICAL_CHANGELOG.md` in their respective crate directories.

## [Unreleased]

## [0.11.4] 2022-01-11

### Added

- libcnb-data: Store struct now supports `clone()` and `default()`. ([#547](https://github.com/heroku/libcnb.rs/pull/547))

## [0.11.3] 2023-01-09

### Added

- libcnb: Add `store` field to `BuildContext`, exposing the contents of `store.toml` if present. ([#543](https://github.com/heroku/libcnb.rs/pull/543))

## [0.11.2] 2022-12-15

### Fixed

- libcnb-test: `TestContext::download_sbom_files` now checks the exit code of the `pack sbom download` command it runs. ([#520](https://github.com/heroku/libcnb.rs/pull/520))

### Changed

- libcnb: Drop the use of the `stacker` crate when recursively removing layer directories. ([#517](https://github.com/heroku/libcnb.rs/pull/517))
- libcnb-cargo: Updated to Clap v4. ([#511](https://github.com/heroku/libcnb.rs/pull/511))

## Added

- libherokubuildpack: Add `command` and `write` modules for working with `std::process::Command` output streams. ([#535](https://github.com/heroku/libcnb.rs/pull/535))

## [0.11.1] 2022-09-29

### Fixed

- All crates now properly include the `LICENSE` file. ([#506](https://github.com/heroku/libcnb.rs/pull/506))
- Fix `libcnb` readme file metadata which prevented vendoring `libcnb` via `cargo vendor`. ([#506](https://github.com/heroku/libcnb.rs/pull/506))

### Changed

- Improve the `libherokubuildpack` root module rustdocs. ([#503](https://github.com/heroku/libcnb.rs/pull/503))

## [0.11.0] 2022-09-23

### Changed

- Bump Minimum Supported Rust Version (MSRV) to `1.64`. ([#500](https://github.com/heroku/libcnb.rs/pull/500))
- Bump minimum external dependency versions. ([#502](https://github.com/heroku/libcnb.rs/pull/502))

### Added

- Add new crate `libherokubuildpack` with common code that can be useful when implementing buildpacks with libcnb. Originally hosted in a separate, private, repository. Code from `libherokubuildpack` might eventually find its way into libcnb.rs proper. At this point, consider it an incubator. ([#495](https://github.com/heroku/libcnb.rs/pull/495))

## [0.10.0] 2022-08-31

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
