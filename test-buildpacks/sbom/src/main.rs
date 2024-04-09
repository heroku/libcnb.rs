use libcnb::build::{BuildContext, BuildResult, BuildResultBuilder};
use libcnb::data::layer_name;
use libcnb::data::sbom::SbomFormat;
use libcnb::detect::{DetectContext, DetectResult, DetectResultBuilder};
use libcnb::generic::{GenericMetadata, GenericPlatform};
use libcnb::layer::{
    CachedLayerDefinition, InspectExistingAction, InvalidMetadataAction, UncachedLayerDefinition,
};
use libcnb::sbom::Sbom;
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
        let test_layer = context.cached_layer(
            layer_name!("test"),
            CachedLayerDefinition {
                build: true,
                launch: true,
                invalid_metadata: &|_| InvalidMetadataAction::DeleteLayer,
                inspect_existing: &|_: &GenericMetadata, _| InspectExistingAction::Keep,
            },
        )?;

        test_layer.replace_sboms(&[
            Sbom::from_bytes(
                SbomFormat::CycloneDxJson,
                *include_bytes!("../etc/cyclonedx_3.sbom.json"),
            ),
            Sbom::from_bytes(
                SbomFormat::SpdxJson,
                *include_bytes!("../etc/spdx_3.sbom.json"),
            ),
            Sbom::from_bytes(
                SbomFormat::SyftJson,
                *include_bytes!("../etc/syft_3.sbom.json"),
            ),
        ])?;

        let test_layer2 = context.uncached_layer(
            layer_name!("test2"),
            UncachedLayerDefinition {
                build: true,
                launch: true,
            },
        )?;

        test_layer2.replace_sboms(&[
            Sbom::from_bytes(
                SbomFormat::CycloneDxJson,
                *include_bytes!("../etc/cyclonedx_2.sbom.json"),
            ),
            Sbom::from_bytes(
                SbomFormat::SpdxJson,
                *include_bytes!("../etc/spdx_2.sbom.json"),
            ),
            Sbom::from_bytes(
                SbomFormat::SyftJson,
                *include_bytes!("../etc/syft_2.sbom.json"),
            ),
        ])?;

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
enum TestBuildpackError {}

buildpack_main!(TestBuildpack);
