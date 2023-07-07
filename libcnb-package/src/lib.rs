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
use std::process::Command;
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
        .map_err(|e| ReadBuildpackDataError::ReadingBuildpack(buildpack_descriptor_path.clone(), e))
        .and_then(|file_contents| {
            toml::from_str(&file_contents).map_err(|e| {
                ReadBuildpackDataError::ParsingBuildpack(buildpack_descriptor_path.clone(), e)
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
    ReadingBuildpack(PathBuf, std::io::Error),
    ParsingBuildpack(PathBuf, toml::de::Error),
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
        .map_err(|e| {
            ReadBuildpackageDataError::ReadingBuildpackage(buildpackage_descriptor_path.clone(), e)
        })
        .and_then(|file_contents| {
            toml::from_str(&file_contents).map_err(|e| {
                ReadBuildpackageDataError::ParsingBuildpackage(
                    buildpackage_descriptor_path.clone(),
                    e,
                )
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
    ReadingBuildpackage(PathBuf, std::io::Error),
    ParsingBuildpackage(PathBuf, toml::de::Error),
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

/// TODO
/// ## Errors
pub fn find_cargo_workspace(dir_in_workspace: &Path) -> Result<PathBuf, FindCargoWorkspaceError> {
    let cargo_bin = std::env::var("CARGO")
        .map(PathBuf::from)
        .map_err(FindCargoWorkspaceError::GetCargoEnv)?;

    let output = Command::new(cargo_bin)
        .args(["locate-project", "--workspace", "--message-format", "plain"])
        .current_dir(dir_in_workspace)
        .output()
        .map_err(FindCargoWorkspaceError::SpawnCommand)?;

    let status = output.status;

    output
        .status
        .success()
        .then_some(output)
        .ok_or(FindCargoWorkspaceError::CommandFailure(status))
        .and_then(|output| {
            let root_cargo_toml = PathBuf::from(String::from_utf8_lossy(&output.stdout).trim());
            root_cargo_toml
                .parent()
                .map(Path::to_path_buf)
                .ok_or(FindCargoWorkspaceError::GetParentDirectory(root_cargo_toml))
        })
}

#[derive(Debug)]
pub enum FindCargoWorkspaceError {
    GetCargoEnv(std::env::VarError),
    SpawnCommand(std::io::Error),
    CommandFailure(std::process::ExitStatus),
    GetParentDirectory(PathBuf),
}
