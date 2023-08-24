#![doc = include_str!("../README.md")]
#![warn(clippy::pedantic)]
#![warn(unused_crate_dependencies)]
// This lint is too noisy and enforces a style that reduces readability in many cases.
#![allow(clippy::module_name_repetitions)]

pub mod build;
pub mod buildpack_dependency;
pub mod buildpack_package;
pub mod cargo;
pub mod cross_compile;
pub mod dependency_graph;
pub mod output;

use crate::build::BuildpackBinaries;
use libcnb_data::buildpack::{BuildpackDescriptor, BuildpackId};
use libcnb_data::package_descriptor::PackageDescriptor;
use std::fs;
use std::path::{Path, PathBuf};
use std::process::Command;
use toml::Table;

/// A convenient type alias to use with [`buildpack_package::BuildpackPackage`] when you don't required a specialized metadata representation.
pub type GenericMetadata = Option<Table>;

/// Reads buildpack data from the given project path.
///
/// # Errors
///
/// Will return `Err` if the buildpack data could not be read successfully.
pub fn read_buildpack_descriptor(
    project_path: impl AsRef<Path>,
) -> Result<BuildpackDescriptor<GenericMetadata>, ReadBuildpackDescriptorError> {
    let buildpack_descriptor_path = project_path.as_ref().join("buildpack.toml");

    fs::read_to_string(buildpack_descriptor_path)
        .map_err(ReadBuildpackDescriptorError::Io)
        .and_then(|file_contents| {
            toml::from_str(&file_contents).map_err(ReadBuildpackDescriptorError::Parse)
        })
}

/// An error from [`read_buildpack_descriptor`]
#[derive(thiserror::Error, Debug)]
pub enum ReadBuildpackDescriptorError {
    #[error("Failed to read buildpack descriptor: {0}")]
    Io(#[source] std::io::Error),
    #[error("Failed to parse buildpack descriptor: {0}")]
    Parse(#[source] toml::de::Error),
}

/// Reads a package descriptor from the given project path.
///
/// # Errors
///
/// Will return `Err` if the package descriptor could not be read successfully.
pub fn read_package_descriptor(
    project_path: impl AsRef<Path>,
) -> Result<PackageDescriptor, ReadPackageDescriptorError> {
    fs::read_to_string(&project_path)
        .map_err(ReadPackageDescriptorError::Io)
        .and_then(|file_contents| {
            toml::from_str(&file_contents).map_err(ReadPackageDescriptorError::Parse)
        })
}

/// An error from [`read_package_descriptor`]
#[derive(thiserror::Error, Debug)]
pub enum ReadPackageDescriptorError {
    #[error("Failed to read buildpackage: {0}")]
    Io(#[source] std::io::Error),
    #[error("Failed to parse buildpackage: {0}")]
    Parse(#[source] toml::de::Error),
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
pub fn get_buildpack_package_dir(
    buildpack_id: &BuildpackId,
    package_dir: &Path,
    is_release: bool,
    target_triple: &str,
) -> PathBuf {
    package_dir
        .join(target_triple)
        .join(if is_release { "release" } else { "debug" })
        .join(output::default_buildpack_directory_name(buildpack_id))
}

/// Returns the path of the root workspace directory for a Rust Cargo project. This is often a useful
/// starting point for detecting buildpacks with [`find_buildpack_dirs`].
///
/// ## Errors
///
/// Will return an `Err` if the root workspace directory cannot be located due to:
/// - no `CARGO` environment variable with the path to the `cargo` binary
/// - executing this function with a directory that is not within a Cargo project
/// - any other file or system error that might occur
pub fn find_cargo_workspace_root_dir(
    dir_in_workspace: &Path,
) -> Result<PathBuf, FindCargoWorkspaceRootError> {
    let cargo_bin = std::env::var("CARGO")
        .map(PathBuf::from)
        .map_err(FindCargoWorkspaceRootError::GetCargoEnv)?;

    let output = Command::new(cargo_bin)
        .args(["locate-project", "--workspace", "--message-format", "plain"])
        .current_dir(dir_in_workspace)
        .output()
        .map_err(FindCargoWorkspaceRootError::SpawnCommand)?;

    let status = output.status;

    output
        .status
        .success()
        .then_some(output)
        .ok_or(FindCargoWorkspaceRootError::CommandFailure(status))
        .and_then(|output| {
            // Cargo outputs a newline after the actual path, so we have to trim.
            let root_cargo_toml = PathBuf::from(String::from_utf8_lossy(&output.stdout).trim());
            root_cargo_toml.parent().map(Path::to_path_buf).ok_or(
                FindCargoWorkspaceRootError::GetParentDirectory(root_cargo_toml),
            )
        })
}

#[derive(thiserror::Error, Debug)]
pub enum FindCargoWorkspaceRootError {
    #[error("Cannot get value of CARGO environment variable: {0}")]
    GetCargoEnv(#[source] std::env::VarError),
    #[error("Error while spawning Cargo process: {0}")]
    SpawnCommand(#[source] std::io::Error),
    #[error("Unexpected Cargo exit status ({}) while attempting to read workspace root", exit_code_or_unknown(*.0))]
    CommandFailure(std::process::ExitStatus),
    #[error("Could not locate a Cargo workspace within {0} or its parent directories")]
    GetParentDirectory(PathBuf),
}

fn exit_code_or_unknown(exit_status: std::process::ExitStatus) -> String {
    exit_status
        .code()
        .map_or_else(|| String::from("<unknown>"), |code| code.to_string())
}

#[cfg(test)]
mod tests {
    use crate::get_buildpack_package_dir;
    use libcnb_data::buildpack_id;
    use std::path::PathBuf;

    #[test]
    fn test_get_buildpack_package_dir() {
        let buildpack_id = buildpack_id!("some-org/with-buildpack");
        let package_dir = PathBuf::from("/package");
        let target_triple = "x86_64-unknown-linux-musl";

        assert_eq!(
            get_buildpack_package_dir(&buildpack_id, &package_dir, false, target_triple),
            PathBuf::from("/package/x86_64-unknown-linux-musl/debug/some-org_with-buildpack")
        );
        assert_eq!(
            get_buildpack_package_dir(&buildpack_id, &package_dir, true, target_triple),
            PathBuf::from("/package/x86_64-unknown-linux-musl/release/some-org_with-buildpack")
        );
    }
}
