# libcnb.rs &emsp; [![Build Status]][ci] [![Docs]][docs.rs] [![Latest Version]][crates.io] [![MSRV]][install-rust]

[Build Status]: https://img.shields.io/github/actions/workflow/status/heroku/libcnb.rs/ci.yml?branch=main
[ci]: https://github.com/heroku/libcnb.rs/actions/workflows/ci.yml?query=branch%3Amain
[Docs]: https://img.shields.io/docsrs/libcnb
[docs.rs]: https://docs.rs/libcnb/latest/libcnb/
[Latest Version]: https://img.shields.io/crates/v/libcnb.svg
[crates.io]: https://crates.io/crates/libcnb
[MSRV]: https://img.shields.io/badge/MSRV-rustc_1.64+-lightgray.svg
[install-rust]: https://www.rust-lang.org/tools/install

`libcnb.rs` is a framework for writing [Cloud Native Buildpacks](https://buildpacks.io) in Rust.
It is an opinionated implementation adding language constructs and convenience methods for working
with the spec. It values strong adherence to the spec and data formats.

It currently targets version `0.9` of the CNB [Buildpack API specification](https://github.com/buildpacks/spec/blob/buildpack/0.9/buildpack.md).

## Quick Start Guide

This quick start guide walks you through writing a simple Cloud Native Buildpack in Rust with libcnb.rs. It will also 
demonstrate how to package your buildpack and how to build an application image with it.

### Development Environment Setup

In addition to the libcnb crate, we need some tools to compile, package and run the buildpack. These steps only need to
be carried out once and don't need to be repeated for each buildpack you will write.

#### libcnb Cargo Command

Start by installing [libcnb-cargo](https://crates.io/crates/libcnb-cargo), which provides the `libcnb` Cargo command
that we will use later to package our buildpack:

```shell
cargo install --locked libcnb-cargo
```

#### Cross-compilation prerequisites

It is common to write and build your buildpack on a platform that is different from the platform on which the buildpack will
eventually run. This means we have to cross-compile our buildpack. The `libcnb package` Cargo command tries to help you set
up your environment depending on your host platform, but we always need the appropriate target platform for Rust, which
we can install with `rustup`:

```shell
rustup target add x86_64-unknown-linux-musl
```

#### Docker

If you don't have it already, we need to install Docker. Refer to the Docker documentation on how to install it for your
operating system: <https://docs.docker.com/engine/install/>

#### pack

To run our buildpack locally, we will use `pack`, a tool maintained by the Cloud Native Buildpacks project to support 
the use of buildpacks. It's the tool that we will eventually use to run our buildpack and create application images. 
Find their documentation about installing it here: <https://buildpacks.io/docs/install-pack/>

### Project Setup

After we've successfully set up our development environment, we can move on and create a new Rust project for our 
buildpack. First, we create a new binary crate with Cargo:

```shell
cargo new my-buildpack
```

Then, add the `libcnb` dependency to your project's `Cargo.toml`:

```shell
cargo add libcnb
```

Note: If you get an error about `cargo add` not being a supported command, make sure you are
using Rust 1.62+, or else install [cargo-edit](https://github.com/killercup/cargo-edit).

Since we're writing a Cloud Native Buildpack, we also need a `buildpack.toml`. Use the template below and write it to a
file named `buildpack.toml` in the root of your project, right next to `Cargo.toml`.

```toml
api = "0.9"

[buildpack]
id = "libcnb-examples/my-buildpack"
version = "0.1.0"
name = "My Buildpack"

[[stacks]]
id = "*"
```

That's all we need! We can now move on to finally write some buildpack code!

### Writing the Buildpack

The buildpack we're writing will be very simple. We will just log a "Hello World" message during the build
and set the default process type to a command that will also emit "Hello World" when the application image is run.
Examples of more complex buildpacks can be found in the [examples directory](https://github.com/heroku/libcnb.rs/tree/main/examples).

Modify the project's `src/main.rs` file to contain the following:

```rust,no_run
use libcnb::build::{BuildContext, BuildResult, BuildResultBuilder};
use libcnb::data::launch::{LaunchBuilder, ProcessBuilder};
use libcnb::data::process_type;
use libcnb::detect::{DetectContext, DetectResult, DetectResultBuilder};
use libcnb::generic::{GenericError, GenericMetadata, GenericPlatform};
use libcnb::{buildpack_main, Buildpack};

pub(crate) struct HelloWorldBuildpack;

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
                LaunchBuilder::new()
                    .process(
                        ProcessBuilder::new(process_type!("web"), ["echo"])
                            .arg("Hello World!")
                            .default(true)
                            .build(),
                    )
                    .build(),
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
  --path /path/to/application

/Users/example/src/my-buildpack/packaged/x86_64-unknown-linux-musl/debug/libcnb-examples_my-buildpack
```

If you get errors with hints about how to install required tools to cross-compile from your host platform to the
target platform, follow them and try again. These differ from platform to platform, which is why they're not part of this
quick start guide.

### Running the Buildpack

You might have seen in the output that we're now ready to run our buildpack locally 
with `pack`. Before we can do this, we need an application to build. Since our buildpack does not interact with the
application code at all, we just create an empty directory and use that as our application:

```console
$ mkdir bogus-app
$ pack build my-image --buildpack packaged/x86_64-unknown-linux-musl/debug/libcnb-examples_my-buildpack --path bogus-app --builder heroku/builder:22
...
===> ANALYZING
Image with name "my-image" not found
===> DETECTING
libcnb-examples/my-buildpack 0.1.0
===> RESTORING
===> BUILDING
Hello World!
Build runs on stack heroku-22!
===> EXPORTING
Adding layer 'buildpacksio/lifecycle:launch.sbom'
Adding 1/1 app layer(s)
Adding layer 'buildpacksio/lifecycle:launcher'
Adding layer 'buildpacksio/lifecycle:config'
Adding layer 'buildpacksio/lifecycle:process-types'
Adding label 'io.buildpacks.lifecycle.metadata'
Adding label 'io.buildpacks.build.metadata'
Adding label 'io.buildpacks.project.metadata'
Setting default process type 'web'
Saving my-image...
*** Images (aa4695184718):
      my-image
Successfully built image my-image
```

### Running the image

The newly created Docker image can be run in the same way as you would a Docker image created via `docker build`.
If all went well, you should see our "Hello World!" message in your terminal:

```console
$ docker run my-image
Hello World!
```

### Next Steps

While the buildpack we've written in this quick start guide is not very useful, it can
serve as a starting point for a more useful buildpack. To discover more of the libcnb API,
browse the [examples directory](https://github.com/heroku/libcnb.rs/tree/main/examples)
and the [documentation on docs.rs][docs.rs].

Later, when you are ready to write integration tests for your buildpack, see the [libcnb-test documentation](https://docs.rs/libcnb-test/).
