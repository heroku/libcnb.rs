use libcnb::build::{BuildContext, BuildResult, BuildResultBuilder};
use libcnb::data::store::Store;
use libcnb::detect::{DetectContext, DetectResult, DetectResultBuilder};
use libcnb::generic::{GenericMetadata, GenericPlatform};
use libcnb::{Buildpack, buildpack_main};
use toml::toml;

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
enum TestBuildpackError {}

buildpack_main!(TestBuildpack);
