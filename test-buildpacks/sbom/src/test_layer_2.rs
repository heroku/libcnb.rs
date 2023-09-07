use crate::{SbomFormat, TestBuildpack};
use libcnb::build::BuildContext;
use libcnb::data::layer_content_metadata::LayerTypes;
use libcnb::generic::GenericMetadata;
use libcnb::layer::{ExistingLayerStrategy, Layer, LayerData, LayerResult, LayerResultBuilder};
use libcnb::sbom::Sbom;
use libcnb::Buildpack;
use std::path::Path;

pub(crate) struct TestLayer2;

impl Layer for TestLayer2 {
    type Buildpack = TestBuildpack;
    type Metadata = GenericMetadata;

    fn types(&self) -> LayerTypes {
        LayerTypes {
            launch: true,
            build: true,
            cache: true,
        }
    }

    fn create(
        &mut self,
        _context: &BuildContext<Self::Buildpack>,
        _layer_path: &Path,
    ) -> Result<LayerResult<Self::Metadata>, <Self::Buildpack as Buildpack>::Error> {
        LayerResultBuilder::new(GenericMetadata::default())
            .sbom(Sbom::from_bytes(
                SbomFormat::CycloneDxJson,
                *include_bytes!("../etc/cyclonedx_2.sbom.json"),
            ))
            .sbom(Sbom::from_bytes(
                SbomFormat::SpdxJson,
                *include_bytes!("../etc/spdx_2.sbom.json"),
            ))
            .sbom(Sbom::from_bytes(
                SbomFormat::SyftJson,
                *include_bytes!("../etc/syft_2.sbom.json"),
            ))
            .build()
    }

    fn existing_layer_strategy(
        &mut self,
        _context: &BuildContext<Self::Buildpack>,
        _layer_data: &LayerData<Self::Metadata>,
    ) -> Result<ExistingLayerStrategy, <Self::Buildpack as Buildpack>::Error> {
        Ok(ExistingLayerStrategy::Keep)
    }
}
