# libcnb-cargo &emsp; [![Latest Version]][crates.io] [![MSRV]][install-rust]

A Cargo command for managing buildpacks written with [libcnb.rs](https://github.com/heroku/libcnb.rs).

## Installation

```shell
cargo install --locked libcnb-cargo
```

## Usage

Currently, there is only one sub-command: `package`. It allows users to package their
Rust buildpack in a spec-compliant manner and helps with cross-compilation.

```console
$ cargo libcnb package --help
Packages a libcnb.rs Cargo project as a Cloud Native Buildpack

Usage: cargo libcnb package [OPTIONS]

Options:
      --no-cross-compile-assistance  Disable cross-compile assistance
      --release                      Build in release mode, with optimizations
      --target <TARGET>              Build for the target triple [default: x86_64-unknown-linux-musl]
      --package-dir <PACKAGE_DIR>    Directory for packaged buildpacks, defaults to 'packaged' in Cargo workspace root
  -h, --help                         Print help
```

Using it is fairly simple, run `cargo libcnb package` inside the buildpack's
project directory:

```console
$ cargo libcnb package
🚚 Preparing package directory...
🖥️ Gathering Cargo configuration (for x86_64-unknown-linux-musl)
🏗️ Building buildpack dependency graph...
🔀 Determining build order...
🚚 Building 1 buildpacks...
📦 [1/1] Building libcnb-examples/my-buildpack (./)
# Omitting compilation output...
    Finished dev [unoptimized] target(s) in 8.24s
Successfully wrote buildpack directory: packaged/x86_64-unknown-linux-musl/debug/libcnb-examples_my-buildpack (4.09 MiB)
✨ Packaging successfully finished!

💡 To test your buildpack locally with pack, run:
pack build my-image-name \
  --buildpack packaged/x86_64-unknown-linux-musl/debug/libcnb-examples_my-buildpack \
  --trust-extra-buildpacks \
  --path /path/to/application

/Users/example/src/my-buildpack/packaged/x86_64-unknown-linux-musl/debug/libcnb-examples_my-buildpack
```

[Latest Version]: https://img.shields.io/crates/v/libcnb-cargo.svg
[crates.io]: https://crates.io/crates/libcnb-cargo
[MSRV]: https://img.shields.io/badge/MSRV-rustc_1.76+-lightgray.svg
[install-rust]: https://www.rust-lang.org/tools/install
