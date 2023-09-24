# Releasing

All crates are released at the same time and with the same version, even if there are no changes to a crate. This makes it
easier to gauge cross-crate compatibility.

1. Trigger the [Prepare release](https://github.com/heroku/libcnb.rs/actions/workflows/prepare-release.yml) GitHub Actions workflow on `main`, with a suitable `{patch,minor,major}` version bump.
2. Once the release preparation PR has been opened, review it (including ensuring the changelog is accurate) and then merge.
3. Trigger the [Release](https://github.com/heroku/libcnb.rs/actions/workflows/release.yml) GitHub Actions workflow on `main`, which will publish the crates to <https://crates.io> and create a GitHub Release.
