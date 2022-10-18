//! Software Bill of Materials (SBOM) support.

use libcnb_data::sbom::SbomFormat;
use std::fs;
use std::path::{Path, PathBuf};

/// A software bill of materials (SBOM).
///
/// Since the CNB specification supports multiple SBOM formats and there are many ways to generate
/// and represent them, libcnb.rs treats them as raw bytes with attached metadata about the format.
///
/// This means there is no direct support for generating an SBOM file with libcnb.rs. Leverage
/// external tooling such as build tool plugins or Rust crates to generate SBOM files. libcnb.rs
/// offers [`From`]/[`TryFrom`] implementations for common Rust SBOM libraries. Enable the
/// corresponding features to gain access to them.
#[derive(Debug, Clone)]
pub struct Sbom {
    pub format: SbomFormat,
    pub data: Vec<u8>,
}

impl Sbom {
    /// Constructs an `Sbom` from the given path, treating it as the SBOM format specified.
    ///
    /// Note that there is no validation performed by libcnb.rs, the CNB lifecycle will error at
    /// runtime should the SBOM be invalid.
    pub fn from_path<P: AsRef<Path>>(format: SbomFormat, path: P) -> std::io::Result<Self> {
        fs::read(path.as_ref()).map(|data| Self { format, data })
    }

    /// Constructs an `Sbom` from the given bytes, treating it as the SBOM format specified.
    ///
    /// Note that there is no validation performed by libcnb.rs, the CNB lifecycle will error at
    /// runtime should the SBOM be invalid.
    pub fn from_bytes<D: Into<Vec<u8>>>(format: SbomFormat, data: D) -> Self {
        Self {
            format,
            data: data.into(),
        }
    }
}

#[cfg(feature = "cyclonedx-bom")]
impl TryFrom<cyclonedx_bom::models::bom::Bom> for Sbom {
    type Error = cyclonedx_bom::errors::JsonWriteError;

    fn try_from(cyclonedx_bom: cyclonedx_bom::models::bom::Bom) -> Result<Self, Self::Error> {
        let mut data = vec![];

        cyclonedx_bom.output_as_json_v1_3(&mut data)?;

        Ok(Self {
            format: SbomFormat::CycloneDxJson,
            data,
        })
    }
}

pub(crate) fn cnb_sbom_path<P: AsRef<Path>>(
    sbom_format: &SbomFormat,
    base_directory: P,
    base_name: &str,
) -> PathBuf {
    let suffix = match sbom_format {
        SbomFormat::CycloneDxJson => "cdx.json",
        SbomFormat::SpdxJson => "spdx.json",
        SbomFormat::SyftJson => "syft.json",
    };

    base_directory
        .as_ref()
        .join(format!("{base_name}.sbom.{suffix}"))
}
