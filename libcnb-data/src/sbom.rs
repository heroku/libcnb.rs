use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Eq, PartialEq, Hash, Deserialize, Serialize)]
pub enum SbomFormat {
    /// Cyclone DX (JSON)
    ///
    /// See: <https://cyclonedx.org/>
    #[serde(rename = "application/vnd.cyclonedx+json")]
    CycloneDxJson,

    /// SPDX (JSON)
    ///
    /// See: <https://spdx.dev/>
    #[serde(rename = "application/spdx+json")]
    SpdxJson,

    /// Syft (JSON)
    ///
    /// See: <https://github.com/anchore/syft>
    #[serde(rename = "application/vnd.syft+json")]
    SyftJson,
}

/// All currently supported SBOM formats.
pub const SBOM_FORMATS: &[SbomFormat] = &[
    SbomFormat::CycloneDxJson,
    SbomFormat::SpdxJson,
    SbomFormat::SyftJson,
];
