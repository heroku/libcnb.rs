// This test buildpack uses the older trait Layer API. It will be updated to the newer API
// before the next libcnb.rs release.
#![allow(deprecated)]

mod layer;

use crate::layer::TestLayer;
use libcnb::build::{BuildContext, BuildResult, BuildResultBuilder};
use libcnb::data::layer_name;
use libcnb::detect::{DetectContext, DetectResult, DetectResultBuilder};
use libcnb::generic::{GenericMetadata, GenericPlatform};
use libcnb::{buildpack_main, Buildpack};

// Suppress warnings due to the `unused_crate_dependencies` lint not handling integration tests well.
#[cfg(test)]
use libcnb_test as _;

pub(crate) struct TestBuildpack;

impl Buildpack for TestBuildpack {
    type Platform = GenericPlatform;
    type Metadata = GenericMetadata;
    type Error = TestBuildpackError;

    fn detect(&self, _context: DetectContext<Self>) -> libcnb::Result<DetectResult, Self::Error> {
        DetectResultBuilder::pass().build()
    }

    fn build(&self, context: BuildContext<Self>) -> libcnb::Result<BuildResult, Self::Error> {
        context.handle_layer(layer_name!("test"), TestLayer)?;
        BuildResultBuilder::new().build()
    }
}

#[derive(Debug)]
enum TestBuildpackError {}

buildpack_main!(TestBuildpack);
