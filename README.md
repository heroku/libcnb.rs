# libcnb.rs [![Build Status]][ci] [![Docs]][docs.rs] [![Latest Version]][crates.io] [![Rustc Version 1.56+]][rustc]

[Build Status]: https://img.shields.io/github/workflow/status/Malax/libcnb/Rust/master
[ci]: https://github.com/Malax/libcnb/actions?query=branch%3Amaster
[Docs]: https://img.shields.io/docsrs/libcnb
[docs.rs]: https://docs.rs/libcnb/*/libcnb/
[Latest Version]: https://img.shields.io/crates/v/libcnb.svg
[crates.io]: https://crates.io/crates/libcnb
[Rustc Version 1.56+]: https://img.shields.io/badge/rustc-1.56+-lightgray.svg
[rustc]: https://blog.rust-lang.org/2021/10/21/Rust-1.56.0.html

`libcnb.rs` is a Rust language binding of the [Cloud Native Buildpacks](https://buildpacks.io) [spec](https://github.com/buildpacks/spec). It is a non-opinionated implementation adding language constructs and convenience methods for working with the spec. It values strong adherence to the spec and data formats.

## Usage

Here's a [quick start template](https://github.com/Malax/rust-cnb-starter) that can be cloned.

View the [examples](./examples) for some buildpack samples.

All spec data files are implemented in the [`libcnb::data`](https://docs.rs/libcnb/*/libcnb/data/index.html) module.

[`libcnb::platform::Platform`](https://docs.rs/libcnb/*/libcnb/platform/trait.Platform.html) represents the `/platform` directory in the CNB spec.


### Example Buildpack

A basic hello world buildpack looks like:

#### Detect

For `/bin/detect`, [`libcnb::detect::cnb_runtime_detect`](https://docs.rs/libcnb/*/libcnb/detect/fn.cnb_runtime_detect.html) handles processing the arguments (made available through [`libcnb::detect::DetectContext`](https://docs.rs/libcnb/*/libcnb/detect/struct.DetectContext.html) and handling the lifecycle of the detect script (including exiting with [`libcnb::detect::DetectOutcome`](https://docs.rs/libcnb/*/libcnb/detect/enum.DetectOutcome.html)). This function will exit and write the build plan where applicable. The buildpack author is responsible for writing the `FnOnce(DetectContext<P>) -> Result<DetectOutcome, E> where E: std::fmt::Display` that `libcnb::detect::cnb_runtime_detect`] takes.


```rust
use libcnb::{
    data::build_plan::BuildPlan,
    detect::{DetectOutcome, GenericDetectContext},
};

use rust_cnb_starter::messages;

fn main() {
    libcnb::detect::cnb_runtime_detect(detect)
}

fn detect(_context: GenericDetectContext) -> Result<DetectOutcome, std::io::Error> {
    println!("/bin/detect is running!");
    Ok(DetectOutcome::Pass(BuildPlan::new()))
}
```

#### Build

For `/bin/build`, [`libcnb::build::cnb_runtime_build`](https://docs.rs/libcnb/*/libcnb/build/fn.cnb_runtime_build.html) will handle processing the arguments and exiting. Arguments and layer creation can be found on [`libcnb::build::BuildContext`](https://docs.rs/libcnb/*/libcnb/build/index.html). If an error is raised, `libcnb::build::cnb_runtime_build` will print out an error message and exit with an error status code. The buildpack author is responsible for defining a `Fn(BuildContext<P>) -> Result<(), E> where E: std::fmt::Display, P: libcnb::platform::Platform`.

```rust

use libcnb::build::GenericBuildContext;
use std::collections::HashMap;

fn main() {
    libcnb::build::cnb_runtime_build(build);
}

fn build(context: GenericBuildContext) -> Result<(), std::io::Error> {
    println!("/bin/build is running!");
    println!("App source @ {:?}", context.app_dir);

    Ok(())
}
```

## Installation
Add the following to your `Cargo.toml` file:

```toml
[dependencies]
libcnb = "0.1.0"
```

*Compiler support requires rustc 1.56+ for 2021 edition*
