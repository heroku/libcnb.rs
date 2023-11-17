use libcnb::build::{BuildContext, BuildResult, BuildResultBuilder};
use libcnb::data::store::Store;
use libcnb::detect::{DetectContext, DetectResult, DetectResultBuilder};
use libcnb::generic::{GenericMetadata, GenericPlatform};
use libcnb::{buildpack_main, Buildpack};
use std::io::Error;
use toml::toml;

// Suppress warnings due to the `unused_crate_dependencies` lint not handling integration tests well.
#[cfg(test)]
use libcnb_test as _;

pub struct TestBuildpack;

impl Buildpack for TestBuildpack {
    type Platform = GenericPlatform;
    type Metadata = GenericMetadata;
    type Error = TestBuildpackError;

    fn detect(&self, _context: DetectContext<Self>) -> libcnb::Result<DetectResult, Self::Error> {
        DetectResultBuilder::pass().build()
    }

    fn build(&self, context: BuildContext<Self>) -> libcnb::Result<BuildResult, Self::Error> {
        println!("context.store={:?}", context.store);

        BuildResultBuilder::new()
            .store(Store {
                metadata: toml! {
                    pinned_language_runtime_version = "1.2.3"
                },
            })
            .build()
    }
}

#[derive(Debug)]
pub enum TestBuildpackError {
    IOError(std::io::Error),
}

impl From<std::io::Error> for TestBuildpackError {
    fn from(io_error: Error) -> Self {
        Self::IOError(io_error)
    }
}

buildpack_main!(TestBuildpack);
