use crate::build::build_buildpack_binaries;
use crate::buildpack_kind::{determine_buildpack_kind, BuildpackKind};
use crate::package_descriptor::{normalize_package_descriptor, NormalizePackageDescriptorError};
use crate::{assemble_buildpack_directory, CargoProfile};
use cargo_metadata::MetadataCommand;
use libcnb_common::toml_file::{read_toml_file, write_toml_file, TomlFileError};
use libcnb_data::buildpack::BuildpackId;
use libcnb_data::package_descriptor::PackageDescriptor;
use std::collections::BTreeMap;
use std::ffi::OsString;
use std::fs;
use std::path::{Path, PathBuf};

/// Packages either a libcnb.rs or a composite buildpack.
///
/// # Errors
///
/// Returns `Err` if packaging failed or the given buildpack directory is unsupported.
pub fn package_buildpack(
    buildpack_directory: &Path,
    cargo_profile: CargoProfile,
    target_triple: &str,
    cargo_build_env: &[(OsString, OsString)],
    destination: &Path,
    dependencies: &BTreeMap<BuildpackId, PathBuf>,
) -> Result<(), PackageBuildpackError> {
    match determine_buildpack_kind(buildpack_directory) {
        Some(BuildpackKind::LibCnbRs) => package_libcnb_buildpack(
            buildpack_directory,
            cargo_profile,
            target_triple,
            cargo_build_env,
            destination,
        )
        .map_err(PackageBuildpackError::PackageLibcnbBuildpackError),
        Some(BuildpackKind::Composite) => {
            package_composite_buildpack(buildpack_directory, destination, dependencies)
                .map_err(PackageBuildpackError::PackageCompositeBuildpackError)
        }
        _ => Err(PackageBuildpackError::UnsupportedBuildpack),
    }
}

#[derive(thiserror::Error, Debug)]
pub enum PackageBuildpackError {
    #[error("{0}")]
    PackageCompositeBuildpackError(PackageCompositeBuildpackError),
    #[error("{0}")]
    PackageLibcnbBuildpackError(PackageLibcnbBuildpackError),
    #[error("Buildpack is not supported to be packaged")]
    UnsupportedBuildpack,
}

/// Packages a libcnb.rs buildpack after (cross-) compiling.
///
/// # Errors
///
/// Returns `Err` if compilation or packaging failed.
pub fn package_libcnb_buildpack(
    buildpack_directory: &Path,
    cargo_profile: CargoProfile,
    target_triple: &str,
    cargo_build_env: &[(OsString, OsString)],
    destination: &Path,
) -> Result<(), PackageLibcnbBuildpackError> {
    let cargo_metadata = MetadataCommand::new()
        .manifest_path(&buildpack_directory.join("Cargo.toml"))
        .exec()
        .map_err(PackageLibcnbBuildpackError::CargoMetadataError)?;

    let buildpack_binaries = build_buildpack_binaries(
        buildpack_directory,
        &cargo_metadata,
        cargo_profile,
        cargo_build_env,
        target_triple,
    )
    .map_err(PackageLibcnbBuildpackError::BuildBinariesError)?;

    assemble_buildpack_directory(
        destination,
        buildpack_directory.join("buildpack.toml"),
        &buildpack_binaries,
    )
    .map_err(PackageLibcnbBuildpackError::AssembleBuildpackDirectory)?;

    fs::write(
        destination.join("package.toml"),
        "[buildpack]\nuri = \".\"\n",
    )
    .map_err(PackageLibcnbBuildpackError::WritePackageDescriptor)
}

#[derive(thiserror::Error, Debug)]
pub enum PackageLibcnbBuildpackError {
    #[error("Assembling buildpack directory failed: {0}")]
    AssembleBuildpackDirectory(std::io::Error),
    #[error("IO error while writing package descriptor: {0}")]
    WritePackageDescriptor(std::io::Error),
    #[error("Building buildpack binaries failed: {0}")]
    BuildBinariesError(crate::build::BuildBinariesError),
    #[error("Obtaining Cargo metadata failed: {0}")]
    CargoMetadataError(cargo_metadata::Error),
}

/// Packages a composite buildpack.
///
/// Packaging consists of copying `buildpack.toml` as well as `package.toml` to the given
/// destination path.
///
/// In addition, references to libcnb.rs buildpacks in the form of `libcnb:` URIs are resolved and
/// local paths are absolutized so the `package.toml` stays correct after being moved to a
/// different location.
///
/// # Errors
///
/// Returns `Err` if a `libcnb:` URI refers to a buildpack not in `buildpack_paths` or packaging
/// otherwise failed (i.e. IO errors).
pub fn package_composite_buildpack(
    buildpack_directory: &Path,
    destination: &Path,
    buildpack_paths: &BTreeMap<BuildpackId, PathBuf>,
) -> Result<(), PackageCompositeBuildpackError> {
    fs::copy(
        buildpack_directory.join("buildpack.toml"),
        destination.join("buildpack.toml"),
    )
    .map_err(PackageCompositeBuildpackError::CouldNotCopyBuildpackToml)?;

    let package_descriptor_path = buildpack_directory.join("package.toml");

    let normalized_package_descriptor =
        read_toml_file::<PackageDescriptor>(&package_descriptor_path)
            .map_err(PackageCompositeBuildpackError::CouldNotReadPackageDescriptor)
            .and_then(|package_descriptor| {
                normalize_package_descriptor(
                    &package_descriptor,
                    &package_descriptor_path,
                    buildpack_paths,
                )
                .map_err(PackageCompositeBuildpackError::NormalizePackageDescriptorError)
            })?;

    write_toml_file(
        &normalized_package_descriptor,
        destination.join("package.toml"),
    )
    .map_err(PackageCompositeBuildpackError::CouldNotWritePackageDescriptor)
}

#[derive(thiserror::Error, Debug)]
pub enum PackageCompositeBuildpackError {
    #[error("Could not copy buildpack.toml: {0}")]
    CouldNotCopyBuildpackToml(std::io::Error),
    #[error("Could not read package.toml: {0}")]
    CouldNotReadPackageDescriptor(TomlFileError),
    #[error("Error while normalizing package.toml: {0}")]
    NormalizePackageDescriptorError(NormalizePackageDescriptorError),
    #[error("Could not write package descriptor: {0}")]
    CouldNotWritePackageDescriptor(TomlFileError),
}
