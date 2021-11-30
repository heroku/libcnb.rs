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
INFO - Found buildpack libcnb-examples/ruby with version 0.1.0.
INFO - Building buildpack binary (x86_64-unknown-linux-musl)...
   Compiling syn v1.0.82
   Compiling bit-vec v0.6.3
   Compiling regex-syntax v0.6.25
   # Omitting further compilation output...
    Finished dev [unoptimized + debuginfo] target(s) in 19.71s
INFO - Writing buildpack directory...
INFO - Successfully wrote buildpack directory: ../../target/libcnb-examples_ruby_0.1.0_dev (103.6M)
INFO - Packaging successfully finished!
INFO - Hint: To test your buildpack locally with pack, run: pack build my-image --buildpack ../../target/libcnb-examples_ruby_0.1.0_dev --path /path/to/application
```
