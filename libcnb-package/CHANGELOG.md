# Changelog

## [Unreleased]

- Strip buildpack binaries when packaging, to reduce buildpack size. This not only reduces the size of production builder images, but also speeds up workflows that involve non-release builds (such as integration tests). ([#445](https://github.com/heroku/libcnb.rs/pull/445))
- The type of the `cargo_env` argument to `build::build_buildpack_binaries` and `build::build_binary` has changed. ([#445](https://github.com/heroku/libcnb.rs/pull/445))
- Use the crate's `README.md` as the root module's rustdocs, so that all of the crate's documentation can be seen in one place on `docs.rs`. ([#460](https://github.com/heroku/libcnb.rs/pull/460))
- Increase minimum supported Rust version from 1.58 to 1.59. ([#445](https://github.com/heroku/libcnb.rs/pull/445))

## [0.1.2] 2022-06-24

- Only create `.libcnb-cargo/additional-bin` if there are additional binaries to bundle. ([#413](https://github.com/heroku/libcnb.rs/pull/413))
- Update `cargo_metadata` dependency from `0.14.2` to `0.15.0`. ([#423](https://github.com/heroku/libcnb.rs/pull/423))

## [0.1.1] 2022-04-12

- Update project URLs for the GitHub repository move to the `heroku` org. ([#388](https://github.com/heroku/libcnb.rs/pull/388))

## [0.1.0] 2022-03-08

- Initial release, containing the packaging functionality extracted from `libcnb-cargo`. ([#362](https://github.com/heroku/libcnb.rs/pull/362))
