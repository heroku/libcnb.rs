# libcnb.rs [![Build Status]][ci] [![Docs]][docs.rs] [![Latest Version]][crates.io] [![Rustc Version 1.56+]][rustc]

[Build Status]: https://img.shields.io/github/workflow/status/Malax/libcnb/Rust/master
[ci]: https://github.com/Malax/libcnb/actions?query=branch%3Amaster
[Docs]: https://img.shields.io/docsrs/libcnb
[docs.rs]: https://docs.rs/libcnb/*/libcnb/
[Latest Version]: https://img.shields.io/crates/v/libcnb.svg
[crates.io]: https://crates.io/crates/libcnb
[Rustc Version 1.56+]: https://img.shields.io/badge/rustc-1.56+-lightgray.svg
[rustc]: https://blog.rust-lang.org/2021/10/21/Rust-1.56.0.html

`libcnb.rs` is a Rust framework for writing [Cloud Native Buildpacks](https://buildpacks.io) in Rust. It is an opinionated implementation adding language constructs and convenience methods for working with the spec. It values strong adherence to the spec and data formats.

It currently targets version `0.6` of the CNB spec.


## Installation
Add the following to your `Cargo.toml` file:

```toml
[dependencies]
libcnb = "0.5.0"
```

*Compiler support requires rustc 1.56+ for 2021 edition*

## Usage
View the [examples](./libcnb/examples) for some buildpack samples.

All spec data files are implemented in the [`libcnb-data`](https://docs.rs/libcnb-data) crate and
can be used without the framework to implement Cloud Native Buildpacks tooling in Rust.

### Hello World Buildpack

A basic hello world buildpack looks like this:

```no_run
use libcnb::{
    cnb_runtime, data::build_plan::BuildPlan, BuildContext, Buildpack, DetectContext,
    DetectOutcome, GenericError, GenericMetadata, GenericPlatform,
};

struct HelloWorldBuildpack;

impl Buildpack for HelloWorldBuildpack {
    // The CNB platform this buildpack targets, usually GenericPlatform. See the CNB spec for more information
    // about platforms: https://github.com/buildpacks/spec/blob/main/buildpack.md
    type Platform = GenericPlatform;
    
    // The type for the metadata of the buildpack itself. This is the data found in the `[metadata]` section
    // of your buildpack's buildpack.toml. The framework will automatically try to parse it into the specified type.
    // This example buildpack uses GenericMetadata which provides low-level access to the TOML table.
    type Metadata = GenericMetadata;
    
    // The error type for this buildpack. Buildpack authors usually implement an enum with specific errors that can
    // happen during buildpack execution. This error type should only contain error specific to this buildpack, such
    // as CouldNotExecuteMaven or InvalidGemfileLock. This example buildpack uses GenericError which means this 
    // buildpack does not specify any errors.
    //
    // More generic errors that happen during buildpack execution such as I/O errors while writing CNB TOML files are
    // handled by libcnb.rs itself.
    type Error = GenericError;

    // This method will be called when the CNB lifecycle calls detect. Use the DetectContext to access CNB data such as
    // the stack this buildpack is currently executed on, the app directory and similar things. When using libcnb.rs, 
    // you never have to read environment variables or read/write files to disk to interact with the CNB lifecycle.
    //
    // One example of this is the return type of this method. DetectOutcome encapsulates the required exit code as well
    // as the data written to the build plan. libcnb.rs will, according to the returned value, handle both writing the 
    // build plan and exiting with the correct status code for you.
    fn detect(&self, context: DetectContext<Self>) -> libcnb::Result<DetectOutcome, Self::Error> {
        Ok(DetectOutcome::Pass(BuildPlan::new()))
    }

    // Similar to detect, this method will be called when the CNB lifecycle executes the build phase.
    fn build(&self, context: BuildContext<Self>) -> libcnb::Result<(), Self::Error> {
        println!("Hello World!");
        println!("Build runs on stack {}!", context.stack_id);
        Ok(())
    }
}

fn main() {
    // This kicks of the framework for the given buildpack.
    cnb_runtime(HelloWorldBuildpack);
}
```
