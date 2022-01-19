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

## Quick Start Guide

This quick start guide walks you through writing a simple Cloud Native Buildpack in Rust with libcnb.rs. It will also 
demonstrate how to package your buildpack and how to build an application image with it.

### Development Environment Setup

In addition to the libcnb crate, we need some tools to compile, package and run the buildpack. These steps only need to
be carried out once and don't need to be repeated for each buildpack you will write. 

#### libcnb Cargo Command

Start by installing the libcnb Cargo command which we will later use to package our buildpack:

```shell
$ cargo install libcnb-cargo
```

#### Cross-compilation prerequisites

It is common to write and build your buildpack on a platform that is different from the platform the buildpack will
eventually run on. This means we have to cross-compile our buildpack. The `libcnb package` Cargo command tries to help you setting
up your environment depending on your host platform, but we always need the appropriate target platform for Rust which 
we can install with `rustup`:

```shell
$ rustup target add x86_64-unknown-linux-musl
```

#### Docker

If you don't have it already, we need to install Docker. Refer to the Docker documentation on how to install it for your
operating system: https://docs.docker.com/engine/install/

#### pack

To run our buildpack locally, we will use `pack`, a tool maintained by the Cloud Native Buildpacks project to support 
the use of buildpacks. It's the tool that we will eventually use to run our buildpack and create application images. 
Find their documentation about installing it here: https://buildpacks.io/docs/tools/pack/

### Project Setup

After we've successfully set up our development environment, we can move on and create a new Rust project for our 
buildpack. First, we create a new binary crate with Cargo:

```shell
$ cargo new my-buildpack
     Created binary (application) `my-buildpack` package
```

Then, modify your project's `Cargo.toml` to include the `libcnb` dependency:

```toml
[package]
name = "my-buildpack"
version = "0.1.0"
edition = "2021"

[dependencies]
libcnb = "0.5.0"
```

Since we're writing a Cloud Native Buildpack, we also need a `buildpack.toml`. Use the template below and write it to a
file named `buildpack.toml` in the root of your project, right next to `Cargo.toml`.

```toml
api = "0.6"

[buildpack]
id = "libcnb-examples/my-buildpack"
version = "0.1.0"
name = "My Buildpack"

[[stacks]]
id = "heroku-20"
```

That's all we need! We can now move on to finally write some buildpack code!


### Writing the Buildpack
As aforementioned, the buildpack we're writing will be very simple. We will just log a "Hello World" message during the build
and set the default process type to a command that will also emit "Hello World" when the application image is run. 
Find more complex example buildpacks in the [examples directory](examples).

Modify the project's `src/main.rs` file to contain the following:

```rust,no_run
use libcnb::build::{BuildContext, BuildResult, BuildResultBuilder};
use libcnb::data::launch::{Launch, ProcessBuilder};
use libcnb::data::process_type;
use libcnb::detect::{DetectContext, DetectResult, DetectResultBuilder};
use libcnb::generic::{GenericError, GenericMetadata, GenericPlatform};
use libcnb::{buildpack_main, Buildpack};

struct HelloWorldBuildpack;

impl Buildpack for HelloWorldBuildpack {
    // The CNB platform this buildpack targets, usually `GenericPlatform`. See the CNB spec for
    // more information about platforms:
    // https://github.com/buildpacks/spec/blob/main/buildpack.md
    type Platform = GenericPlatform;

    // The type for the metadata of the buildpack itself. This is the data found in the
    // `[metadata]` section of your buildpack's `buildpack.toml`. The framework will automatically
    // try to parse it into the specified type. This example buildpack uses GenericMetadata which
    // provides low-level access to the TOML table.
    type Metadata = GenericMetadata;

    // The error type for this buildpack. Buildpack authors usually implement an enum with
    // specific errors that can happen during buildpack execution. This error type should
    // only contain error specific to this buildpack, such as `CouldNotExecuteMaven` or
    // `InvalidGemfileLock`. This example buildpack uses `GenericError` which means this buildpack
    // does not specify any errors.
    //
    // Common errors that happen during buildpack execution such as I/O errors while
    // writing CNB TOML files are handled by libcnb.rs itself.
    type Error = GenericError;

    // This method will be called when the CNB lifecycle executes the detect phase (`bin/detect`).
    // Use the `DetectContext` to access CNB data such as the stack this buildpack is currently
    // executed on, the app directory and similar things. When using libcnb.rs, you never have
    // to read environment variables or read/write files to disk to interact with the CNB lifecycle.
    //
    // One example of this is the return type of this method. `DetectResult` encapsulates the
    // required exit code as well as the data written to the build plan. libcnb.rs will,
    // according to the returned value, handle both writing the build plan and exiting with
    // the correct status code for you.
    fn detect(&self, _context: DetectContext<Self>) -> libcnb::Result<DetectResult, Self::Error> {
        DetectResultBuilder::pass().build()
    }

    // Similar to detect, this method will be called when the CNB lifecycle executes the
    // build phase (`bin/build`).
    fn build(&self, context: BuildContext<Self>) -> libcnb::Result<BuildResult, Self::Error> {
        println!("Hello World!");
        println!("Build runs on stack {}!", context.stack_id);

        BuildResultBuilder::new()
            .launch(
                Launch::new().process(
                    ProcessBuilder::new(process_type!("web"), "echo")
                        .arg("Hello World!")
                        .default(true)
                        .build(),
                ),
            )
            .build()
    }
}

// Implements the main function and wires up the framework for the given buildpack.
buildpack_main!(HelloWorldBuildpack);
```

### Packaging the Buildpack

Now that our buildpack is written, it's time to package it, so that it can be run. If you followed the steps to setup
your development environment, you have access to the `libcnb` Cargo command that will handle packaging for you.

In your project directory, run `cargo libcnb package` to start packaging:

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

If you get errors with hints about how to install required tools to cross-compile from your host platform to the
target platform, follow them and try again. These differ from platform to platform, which is why they're not part of this
quick start guide.

### Running the Buildpack

You might have seen in the output that we're now ready to run our buildpack locally 
with `pack`. Before we can do this, we need an application to build. Since our buildpack does not interact with the
application code at all, we just create an empty directory and use that as our application:

```shell
$ mkdir bogus-app
$ pack build my-image --buildpack target/debug/libcnb-examples_my-buildpack_0.1.0 --path bogus-app --builder heroku/buildpacks:20
20: Pulling from heroku/buildpacks
Digest: sha256:04e8ea7a1f482f289d432d9518edcfaaf9f3a10432cd1b624e58225f22e7c416
Status: Image is up to date for heroku/buildpacks:20
20: Pulling from heroku/pack
Digest: sha256:21ea4b85bc937b47e017bf43136a5295ec08e093e3c210b20e69e9c1b0f2bd57
Status: Image is up to date for heroku/pack:20
===> DETECTING
libcnb-examples/my-buildpack 0.1.0
===> ANALYZING
Skipping buildpack layer analysis
===> BUILDING
Hello World!
Build runs on stack heroku-20!
===> EXPORTING
Reusing 1/1 app layer(s)
Reusing layer 'launcher'
Reusing layer 'config'
Reusing layer 'process-types'
Adding label 'io.buildpacks.lifecycle.metadata'
Adding label 'io.buildpacks.build.metadata'
Adding label 'io.buildpacks.project.metadata'
Setting default process type 'web'
Saving my-image...
*** Images (d4f67a828236):
      my-image
Successfully built image my-image
```

### Running the image
The newly created Docker image can be run in the same way as you would a Docker image created via `docker build`.
If all went well, you should see our "Hello World!" message in your terminal:

```shell
$ docker run my-image
Hello World!
```

### Next Steps
While the buildpack we've written in this quick start guide is not very useful, it can serve as a starting point for a 
more useful buildpack. To discover more of the libcnb API, browse the [examples directory](example) and the 
[documentation on docs.rs][docs.rs].
