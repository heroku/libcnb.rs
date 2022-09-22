# libcnb-cargo &emsp; [![Latest Version]][crates.io] [![MSRV]][install-rust]

A Cargo command for managing buildpacks written with [libcnb.rs](https://github.com/heroku/libcnb.rs).

## Installation

```shell
$ cargo install libcnb-cargo
```

## Usage

Currently, there is only one sub-command: `package`. It allows users to package their Rust buildpack in a spec-compliant
manner and helps with cross-compilation. Using it is fairly simple, run `cargo libcnb package` inside the buildpack's
project directory:

```shell
$ cargo libcnb package
INFO - Reading buildpack metadata...
INFO - Found buildpack libcnb-examples/my-buildpack with version 0.1.0.
INFO - Determining automatic cross-compile settings...
INFO - Building binaries (x86_64-unknown-linux-musl)...
# Omitting compilation output...
    Finished dev [unoptimized + debuginfo] target(s) in 4.29s
INFO - Writing buildpack directory...
INFO - Successfully wrote buildpack directory: target/buildpack/debug/libcnb-examples_my-buildpack (3.26 MiB)
INFO - Packaging successfully finished!
INFO - Hint: To test your buildpack locally with pack, run: pack build my-image --buildpack target/buildpack/debug/libcnb-examples_my-buildpack --path /path/to/application
```

[Latest Version]: https://img.shields.io/crates/v/libcnb-cargo.svg
[crates.io]: https://crates.io/crates/libcnb-cargo
[MSRV]: https://img.shields.io/badge/MSRV-rustc_1.60+-lightgray.svg
[install-rust]: https://www.rust-lang.org/tools/install
