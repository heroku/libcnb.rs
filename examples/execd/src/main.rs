// Enable Clippy lints that are disabled by default.
// https://rust-lang.github.io/rust-clippy/stable/index.html
#![warn(clippy::pedantic)]
// This lint is too noisy and enforces a style that reduces readability in many cases.
#![allow(clippy::module_name_repetitions)]

mod layer;

use crate::layer::ExecDLayer;
use libcnb::build::{BuildContext, BuildResult, BuildResultBuilder};
use libcnb::data::launch::{Launch, ProcessBuilder};
use libcnb::data::layer_name;
use libcnb::data::process_type;
use libcnb::detect::{DetectContext, DetectResult, DetectResultBuilder};
use libcnb::generic::{GenericError, GenericMetadata, GenericPlatform};
use libcnb::{buildpack_main, Buildpack};

pub struct ExecDBuildpack;

impl Buildpack for ExecDBuildpack {
    type Platform = GenericPlatform;
    type Metadata = GenericMetadata;
    type Error = GenericError;

    fn detect(&self, _context: DetectContext<Self>) -> libcnb::Result<DetectResult, Self::Error> {
        DetectResultBuilder::pass().build()
    }

    fn build(&self, context: BuildContext<Self>) -> libcnb::Result<BuildResult, Self::Error> {
        context.handle_layer(layer_name!("layer_name"), ExecDLayer)?;

        BuildResultBuilder::new()
            .launch(
                Launch::new().process(
                    ProcessBuilder::new(process_type!("web"), "sleep 3600")
                        .default(true)
                        .build(),
                ),
            )
            .build()
    }
}

buildpack_main!(ExecDBuildpack);
