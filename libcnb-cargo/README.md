# libcnb-cargo

Cargo command for managing buildpacks written with [libcnb.rs](https://github.com/malax/libcnb.rs).

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
INFO - Building buildpack binary (x86_64-unknown-linux-musl)...
Compiling my-buildpack v0.1.0 (/Users/manuel.fuchs/projects/my-buildpack)
# Omitting compilation output...
Finished dev [unoptimized + debuginfo] target(s) in 2.67s
INFO - Writing buildpack directory...
INFO - Successfully wrote buildpack directory: target/debug/libcnb-examples_my-buildpack_0.1.0 (53.1M)
INFO - Packaging successfully finished!
INFO - Hint: To test your buildpack locally with pack, run: pack build my-image --buildpack target/debug/libcnb-examples_my-buildpack_0.1.0 --path /path/to/application
```

## Configuration

### Main buildpack binary

A buildpack crate can contain multiple binaries. To determine which binary is the main buildpack binary, the convention is:
1. If there is only one binary, use that.
2. If there are multiple, use the one that has the same name as the crate.

If this convention does not work for your buildpack, you can explicitly configure the main buildpack binary in `Cargo.toml`:

```toml
[package]
name = "my-buildpack"
version = "1.0.0"
edition = "2021"
rust-version = "1.56"

[package.metadata.libcnb]
buildpack-target = "buildpack-bin-target-name"

[dependencies]
# ...
```
