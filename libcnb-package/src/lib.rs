// https://rust-lang.github.io/rust-clippy/stable/index.html
#![warn(clippy::pedantic)]
#![warn(unused_crate_dependencies)]
// This lint is too noisy and enforces a style that reduces readability in many cases.
#![allow(clippy::module_name_repetitions)]

pub mod build;
pub mod config;
pub mod cross_compile;

use crate::build::BuildpackBinaries;
use libcnb_data::buildpack::SingleBuildpackDescriptor;
use std::fs;
use std::path::{Path, PathBuf};

/// The profile to use when invoking Cargo.
///
/// <https://doc.rust-lang.org/cargo/reference/profiles.html>
#[derive(Copy, Clone)]
pub enum CargoProfile {
    /// Provides faster compilation times at the expense of runtime performance and binary size.
    Dev,
    /// Produces assets with optimised runtime performance and binary size, at the expense of compilation time.
    Release,
}

/// Reads buildpack data from the given project path.
///
/// # Errors
///
/// Will return `Err` if the buildpack data could not be read successfully.
pub fn read_buildpack_data(
    project_path: impl AsRef<Path>,
) -> Result<BuildpackData<Option<toml::Value>>, BuildpackDataError> {
    let buildpack_descriptor_path = project_path.as_ref().join("buildpack.toml");

    fs::read_to_string(&buildpack_descriptor_path)
        .map_err(BuildpackDataError::IoError)
        .and_then(|file_contents| {
            toml::from_str(&file_contents).map_err(BuildpackDataError::DeserializationError)
        })
        .map(|buildpack_descriptor| BuildpackData {
            buildpack_descriptor_path,
            buildpack_descriptor,
        })
}

#[derive(Debug)]
pub enum BuildpackDataError {
    IoError(std::io::Error),
    DeserializationError(toml::de::Error),
}

pub struct BuildpackData<BM> {
    pub buildpack_descriptor_path: PathBuf,
    pub buildpack_descriptor: SingleBuildpackDescriptor<BM>,
}

/// Creates a buildpack directory and copies all buildpack assets to it.
///
/// Assembly of the directory follows the constraints set by the libcnb framework. For example,
/// the buildpack binary is only copied once and symlinks are used to refer to it when the CNB
/// spec requires different file(name)s.
///
/// This function will not validate if the buildpack descriptor at the given path is valid and will
/// use it as-is.
///
/// # Errors
///
/// Will return `Err` if the buildpack directory could not be assembled.
pub fn assemble_buildpack_directory(
    destination_path: impl AsRef<Path>,
    buildpack_descriptor_path: impl AsRef<Path>,
    buildpack_binaries: &BuildpackBinaries,
) -> std::io::Result<()> {
    fs::create_dir_all(destination_path.as_ref())?;

    fs::copy(
        buildpack_descriptor_path.as_ref(),
        destination_path.as_ref().join("buildpack.toml"),
    )?;

    let bin_path = destination_path.as_ref().join("bin");
    fs::create_dir_all(&bin_path)?;

    fs::copy(
        &buildpack_binaries.buildpack_target_binary_path,
        bin_path.join("build"),
    )?;

    create_file_symlink("build", bin_path.join("detect"))?;

    if !buildpack_binaries.additional_target_binary_paths.is_empty() {
        let additional_binaries_dir = destination_path
            .as_ref()
            .join(".libcnb-cargo")
            .join("additional-bin");

        fs::create_dir_all(&additional_binaries_dir)?;

        for (binary_target_name, binary_path) in &buildpack_binaries.additional_target_binary_paths
        {
            fs::copy(
                binary_path,
                additional_binaries_dir.join(binary_target_name),
            )?;
        }
    }

    Ok(())
}

#[cfg(target_family = "unix")]
fn create_file_symlink<P: AsRef<Path>, Q: AsRef<Path>>(
    original: P,
    link: Q,
) -> std::io::Result<()> {
    std::os::unix::fs::symlink(original.as_ref(), link.as_ref())
}

#[cfg(target_family = "windows")]
fn create_file_symlink<P: AsRef<Path>, Q: AsRef<Path>>(
    original: P,
    link: Q,
) -> std::io::Result<()> {
    std::os::windows::fs::symlink_file(original.as_ref(), link.as_ref())
}

/// Construct a good default filename for a buildpack directory.
///
/// This function ensures the resulting name is valid and does not contain problematic characters
/// such as `/`.
pub fn default_buildpack_directory_name<BM>(
    buildpack_descriptor: &SingleBuildpackDescriptor<BM>,
) -> String {
    buildpack_descriptor.buildpack.id.replace('/', "_")
}
