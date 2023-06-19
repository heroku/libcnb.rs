#![doc = include_str!("../README.md")]
#![warn(clippy::pedantic)]
#![warn(unused_crate_dependencies)]
// This lint is too noisy and enforces a style that reduces readability in many cases.
#![allow(clippy::module_name_repetitions)]

pub mod build;
pub mod buildpack_dependency;
pub mod buildpack_package;
pub mod buildpack_package_graph;
pub mod config;
pub mod cross_compile;

use crate::build::BuildpackBinaries;
use libcnb_data::buildpack::{BuildpackDescriptor, BuildpackId};
use libcnb_data::buildpackage::Buildpackage;
use std::fs;
use std::path::{Path, PathBuf};
use toml::Table;

/// The profile to use when invoking Cargo.
///
/// <https://doc.rust-lang.org/cargo/reference/profiles.html>
#[derive(Debug, Copy, Clone, Eq, PartialEq)]
pub enum CargoProfile {
    /// Provides faster compilation times at the expense of runtime performance and binary size.
    Dev,
    /// Produces assets with optimised runtime performance and binary size, at the expense of compilation time.
    Release,
}

/// A convenient type alias to use with [`buildpack_package::BuildpackPackage`] or [`BuildpackData`] when you don't required a specialized metadata representation.
pub type GenericMetadata = Option<Table>;

/// A parsed buildpack descriptor and it's path.
#[derive(Debug)]
pub struct BuildpackData<BM> {
    pub buildpack_descriptor_path: PathBuf,
    pub buildpack_descriptor: BuildpackDescriptor<BM>,
}

/// Reads buildpack data from the given project path.
///
/// # Errors
///
/// Will return `Err` if the buildpack data could not be read successfully.
pub fn read_buildpack_data(
    project_path: impl AsRef<Path>,
) -> Result<BuildpackData<GenericMetadata>, ReadBuildpackDataError> {
    let dir = project_path.as_ref();
    let buildpack_descriptor_path = dir.join("buildpack.toml");
    fs::read_to_string(&buildpack_descriptor_path)
        .map_err(|e| ReadBuildpackDataError::ReadingBuildpack {
            path: buildpack_descriptor_path.clone(),
            source: e,
        })
        .and_then(|file_contents| {
            toml::from_str(&file_contents).map_err(|e| ReadBuildpackDataError::ParsingBuildpack {
                path: buildpack_descriptor_path.clone(),
                source: e,
            })
        })
        .map(|buildpack_descriptor| BuildpackData {
            buildpack_descriptor_path,
            buildpack_descriptor,
        })
}

/// An error from [`read_buildpack_data`]
#[derive(Debug)]
pub enum ReadBuildpackDataError {
    ReadingBuildpack {
        path: PathBuf,
        source: std::io::Error,
    },
    ParsingBuildpack {
        path: PathBuf,
        source: toml::de::Error,
    },
}

/// A parsed buildpackage descriptor and it's path.
#[derive(Debug, Clone)]
pub struct BuildpackageData {
    pub buildpackage_descriptor_path: PathBuf,
    pub buildpackage_descriptor: Buildpackage,
}

/// Reads buildpackage data from the given project path.
///
/// # Errors
///
/// Will return `Err` if the buildpackage data could not be read successfully.
pub fn read_buildpackage_data(
    project_path: impl AsRef<Path>,
) -> Result<Option<BuildpackageData>, ReadBuildpackageDataError> {
    let buildpackage_descriptor_path = project_path.as_ref().join("package.toml");

    if !buildpackage_descriptor_path.exists() {
        return Ok(None);
    }

    fs::read_to_string(&buildpackage_descriptor_path)
        .map_err(|e| ReadBuildpackageDataError::ReadingBuildpackage {
            path: buildpackage_descriptor_path.clone(),
            source: e,
        })
        .and_then(|file_contents| {
            toml::from_str(&file_contents).map_err(|e| {
                ReadBuildpackageDataError::ParsingBuildpackage {
                    path: buildpackage_descriptor_path.clone(),
                    source: e,
                }
            })
        })
        .map(|buildpackage_descriptor| {
            Some(BuildpackageData {
                buildpackage_descriptor_path,
                buildpackage_descriptor,
            })
        })
}

/// An error from [`read_buildpackage_data`]
#[derive(Debug)]
pub enum ReadBuildpackageDataError {
    ReadingBuildpackage {
        path: PathBuf,
        source: std::io::Error,
    },
    ParsingBuildpackage {
        path: PathBuf,
        source: toml::de::Error,
    },
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
#[must_use]
pub fn default_buildpack_directory_name(buildpack_id: &BuildpackId) -> String {
    buildpack_id.replace('/', "_")
}

/// Recursively walks the file system from the given `start_dir` to locate any folders containing a
/// `buildpack.toml` file.
///
/// # Errors
///
/// Will return an `Err` if any I/O errors happen while walking the file system.
pub fn find_buildpack_dirs(start_dir: &Path, ignore: &[PathBuf]) -> std::io::Result<Vec<PathBuf>> {
    fn find_buildpack_dirs_recursive(
        path: &Path,
        ignore: &[PathBuf],
        accumulator: &mut Vec<PathBuf>,
    ) -> std::io::Result<()> {
        if ignore.contains(&path.to_path_buf()) {
            return Ok(());
        }

        let metadata = path.metadata()?;
        if metadata.is_dir() {
            let entries = fs::read_dir(path)?;
            for entry in entries {
                let entry = entry?;
                let metadata = entry.metadata()?;
                if metadata.is_dir() {
                    find_buildpack_dirs_recursive(&entry.path(), ignore, accumulator)?;
                } else if let Some(file_name) = entry.path().file_name() {
                    if file_name.to_string_lossy() == "buildpack.toml" {
                        accumulator.push(path.to_path_buf());
                    }
                }
            }
        }

        Ok(())
    }

    let mut buildpack_dirs: Vec<PathBuf> = vec![];
    find_buildpack_dirs_recursive(start_dir, ignore, &mut buildpack_dirs)?;
    Ok(buildpack_dirs)
}

/// Provides a standard path to use for storing a compiled buildpack's artifacts.
#[must_use]
pub fn get_buildpack_target_dir(
    buildpack_id: &BuildpackId,
    target_dir: &Path,
    is_release: bool,
) -> PathBuf {
    target_dir
        .join("buildpack")
        .join(if is_release { "release" } else { "debug" })
        .join(default_buildpack_directory_name(buildpack_id))
}

#[cfg(test)]
mod tests {
    use crate::get_buildpack_target_dir;
    use libcnb_data::buildpack_id;
    use std::path::PathBuf;

    #[test]
    fn test_get_buildpack_target_dir() {
        let buildpack_id = buildpack_id!("some-org/with-buildpack");
        let target_dir = PathBuf::from("/target");
        assert_eq!(
            get_buildpack_target_dir(&buildpack_id, &target_dir, false),
            PathBuf::from("/target/buildpack/debug/some-org_with-buildpack")
        );
        assert_eq!(
            get_buildpack_target_dir(&buildpack_id, &target_dir, true),
            PathBuf::from("/target/buildpack/release/some-org_with-buildpack")
        );
    }
}
