// Enable Clippy lints that are disabled by default.
// https://rust-lang.github.io/rust-clippy/stable/index.html
#![warn(clippy::pedantic)]
// This lint is too noisy and enforces a style that reduces readability in many cases.
#![allow(clippy::module_name_repetitions)]

mod test_layer;
mod test_layer_2;

use crate::test_layer::TestLayer;
use crate::test_layer_2::TestLayer2;
use libcnb::build::{BuildContext, BuildResult, BuildResultBuilder};
use libcnb::data::layer_name;
use libcnb::data::sbom::SbomFormat;
use libcnb::detect::{DetectContext, DetectResult, DetectResultBuilder};
use libcnb::generic::{GenericMetadata, GenericPlatform};
use libcnb::sbom::Sbom;
use libcnb::{buildpack_main, Buildpack};
use std::io::Error;

pub struct TestBuildpack;

impl Buildpack for TestBuildpack {
    type Platform = GenericPlatform;
    type Metadata = GenericMetadata;
    type Error = TestBuildpackError;

    fn detect(&self, _context: DetectContext<Self>) -> libcnb::Result<DetectResult, Self::Error> {
        DetectResultBuilder::pass().build()
    }

    fn build(&self, context: BuildContext<Self>) -> libcnb::Result<BuildResult, Self::Error> {
        context.handle_layer(layer_name!("test"), TestLayer)?;
        context.handle_layer(layer_name!("test2"), TestLayer2)?;

        BuildResultBuilder::new()
            .launch_sbom(Sbom::from_bytes(
                SbomFormat::CycloneDxJson,
                *include_bytes!("../etc/cyclonedx_1.sbom.json"),
            ))
            .launch_sbom(Sbom::from_bytes(
                SbomFormat::SpdxJson,
                *include_bytes!("../etc/spdx_1.sbom.json"),
            ))
            .launch_sbom(Sbom::from_bytes(
                SbomFormat::SyftJson,
                *include_bytes!("../etc/syft_1.sbom.json"),
            ))
            .build_sbom(Sbom::from_bytes(
                SbomFormat::CycloneDxJson,
                *include_bytes!("../etc/cyclonedx_2.sbom.json"),
            ))
            .build_sbom(Sbom::from_bytes(
                SbomFormat::SpdxJson,
                *include_bytes!("../etc/spdx_2.sbom.json"),
            ))
            .build_sbom(Sbom::from_bytes(
                SbomFormat::SyftJson,
                *include_bytes!("../etc/syft_2.sbom.json"),
            ))
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
