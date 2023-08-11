#![doc = include_str!("../README.md")]
#![warn(clippy::pedantic)]
#![warn(unused_crate_dependencies)]
// This lint is too noisy and enforces a style that reduces readability in many cases.
#![allow(clippy::module_name_repetitions)]

pub mod build;
pub mod buildpack_dependency;
pub mod buildpack_package;
pub mod config;
pub mod cross_compile;
pub mod dependency_graph;
pub mod output;

use libcnb_data::buildpack::BuildpackDescriptor;
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
