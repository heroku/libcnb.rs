# Changelog

## [Unreleased]

## [0.5.0] 2022-07-14

- Fix the packaged buildpack size reported by `cargo libcnb package`. ([#442](https://github.com/heroku/libcnb.rs/pull/442))
- Reduce number of dependencies to improve installation time. ([#442](https://github.com/heroku/libcnb.rs/pull/442) and [#443](https://github.com/heroku/libcnb.rs/pull/443))
- Use the crate's `README.md` as the root module's rustdocs, so that all of the crate's documentation can be seen in one place on `docs.rs`. ([#460](https://github.com/heroku/libcnb.rs/pull/460))
- Increase minimum supported Rust version from 1.58 to 1.59. ([#445](https://github.com/heroku/libcnb.rs/pull/445))
- Update `libcnb-package` from `0.1.2` to `0.2.0`. Of note, buildpack binaries are now stripped when packaging to reduce buildpack size. See the [libcnb-package changelog](../libcnb-package/CHANGELOG.md#020-2022-07-14). ([#465](https://github.com/heroku/libcnb.rs/pull/465))

## [0.4.1] 2022-06-24

- Update `cargo_metadata` dependency from `0.14.2` to `0.15.0`. ([#423](https://github.com/heroku/libcnb.rs/pull/423))

## [0.4.0] 2022-04-12

- Move the packaging library parts of `libcnb-cargo` to a new `libcnb-package` crate. Consumers of the library should substitute all `libcnb-cargo` references with `libcnb-package` for equivalent functionality. ([#362](https://github.com/heroku/libcnb.rs/pull/362))
- Update project URLs for the GitHub repository move to the `heroku` org. ([#388](https://github.com/heroku/libcnb.rs/pull/388))

## [0.3.0] 2022-02-28

- Update cross-compile assistance on macOS to use https://github.com/messense/homebrew-macos-cross-toolchains instead of https://github.com/FiloSottile/homebrew-musl-cross. Support for the latter has not been removed, existing setups continue to work as before. ([#312](https://github.com/heroku/libcnb.rs/pull/312))
- `libcnb-cargo` now cross-compiles and packages all binary targets of the buildpack. The main buildpack binary is either the only binary target or the target with the same name as the crate. This feature allows the usage of additional binaries for i.e. `exec.d`. ([#314](https://github.com/heroku/libcnb.rs/pull/314))
- Increase minimum supported Rust version from 1.56 to 1.58. ([#318](https://github.com/heroku/libcnb.rs/pull/318))
- Upgrade CLI to Clap v3. ([#329](https://github.com/heroku/libcnb.rs/pull/329))
- Update `libcnb-data` from `0.4.0` to `0.5.0` - see the [libcnb-data changelog](../libcnb-data/CHANGELOG.md#050-2022-02-28). ([#361](https://github.com/heroku/libcnb.rs/pull/361))

## [0.2.1] 2022-01-19

- `cargo libcnb package` now allows multiple targets, as long as there is only one binary target. To support this change, the `BuildError` variants `NoTargetsFound` and `MultipleTargetsFound` have been replaced by `NoBinTargetsFound` and `MultipleBinTargetsFound` respectively. ([#282](https://github.com/heroku/libcnb.rs/pull/282))
- `assemble_buildpack_directory()` no longer fails if the output directory already exists. ([#283](https://github.com/heroku/libcnb.rs/pull/283))

## [0.2.0] 2022-01-14

- `BuildpackData`, `assemble_buildpack_directory()` and `default_buildpack_directory_name()` have been updated for the libcnb-data replacement of `BuildpackToml` with `*BuildpackDescriptor` and rename of `*buildpack_toml` to `*buildpack_descriptor`. ([#248](https://github.com/heroku/libcnb.rs/pull/248) and [#254](https://github.com/heroku/libcnb.rs/pull/254))
- Bump external dependency versions. ([#233](https://github.com/heroku/libcnb.rs/pull/233))
- Update `libcnb-data` from `0.3.0` to `0.4.0` - see the [libcnb-data changelog](../libcnb-data/CHANGELOG.md#040-2022-01-14). ([#276](https://github.com/heroku/libcnb.rs/pull/276))

## [0.1.0] 2021-12-08

- Add a Cargo command for cross-compiling and packaging libcnb buildpacks. ([#199](https://github.com/heroku/libcnb.rs/pull/199))
