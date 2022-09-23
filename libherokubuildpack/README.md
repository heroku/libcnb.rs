# libherokubuildpack &emsp; [![Docs]][docs.rs] [![Latest Version]][crates.io] [![MSRV]][install-rust]

Common utilities for buildpacks written with [libcnb.rs](https://github.com/heroku/libcnb.rs). Originally designed to be
only used for official Heroku buildpacks. It was moved into the libcnb.rs repository as an incubator for utilities that
might find their way into libcnb.rs proper.

This crate is optional and not required to write buildpacks with libcnb.rs. It provides helpers that buildpack authors 
commonly need. Examples are digest generation, filesystem utilities, HTTP download helpers and tarball extraction. 

## Crate Features

It is common to not need all the helpers in this crate. To avoid including unnecessary code and dependencies, this crate
uses Cargo features to allow opt-out of certain modules if they're not needed.

The feature names line up with the modules in this crate. All features are enabled by default.

* **download** -
  Enables helpers to download files over HTTP.
* **digest** -
  Enables helpers to create checksums of files.
* **error** -
  Enables helpers to achieve consistent error logging.
* **log** -
  Enables helpers for logging.
* **tar** -
  Enables helpers for working with tarballs.
* **toml** -
  Enables helpers for working with TOML data.
* **fs** -
  Enables helpers for filesystem related tasks.

[Docs]: https://img.shields.io/docsrs/libherokubuildpack
[docs.rs]: https://docs.rs/libcnb-test/latest/libherokubuildpack/
[Latest Version]: https://img.shields.io/crates/v/libherokubuildpack.svg
[crates.io]: https://crates.io/crates/libherokubuildpack
[MSRV]: https://img.shields.io/badge/MSRV-rustc_1.64+-lightgray.svg
[install-rust]: https://www.rust-lang.org/tools/install
