#![doc = include_str!("../README.md")]

pub mod build;
pub mod buildpack_dependency_graph;
pub mod buildpack_kind;
pub mod cargo;
pub mod cross_compile;
pub mod dependency_graph;
pub mod output;
pub mod package;
pub mod package_descriptor;
pub mod util;

use crate::build::BuildpackBinaries;
use std::fs;
use std::path::{Path, PathBuf};
use std::process::Command;

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
/// Will return `Err` if the buildpack directory couldn't be assembled.
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
/// Will return an `Err` if any I/O errors happen while walking the file system or any parsing errors
/// from reading a gitignore file.
pub fn find_buildpack_dirs(start_dir: &Path) -> Result<Vec<PathBuf>, ignore::Error> {
    ignore::Walk::new(start_dir)
        .collect::<Result<Vec<_>, _>>()
        .map(|entries| {
            entries
                .iter()
                .filter_map(|entry| {
                    if entry.path().is_dir() && entry.path().join("buildpack.toml").exists() {
                        Some(entry.path().to_path_buf())
                    } else {
                        None
                    }
                })
                .collect()
        })
}

/// Returns the path of the root workspace directory for a Rust Cargo project. This is often a useful
/// starting point for detecting buildpacks with [`find_buildpack_dirs`].
///
/// # Errors
///
/// Will return an `Err` if the root workspace directory can't be located due to:
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
    #[error("Couldn't get value of CARGO environment variable: {0}")]
    GetCargoEnv(#[source] std::env::VarError),
    #[error("Error while spawning Cargo process: {0}")]
    SpawnCommand(#[source] std::io::Error),
    #[error("Unexpected Cargo exit status ({}) while attempting to read workspace root", exit_code_or_unknown(*.0))]
    CommandFailure(std::process::ExitStatus),
    #[error("Couldn't locate a Cargo workspace within {0} or its parent directories")]
    GetParentDirectory(PathBuf),
}

fn exit_code_or_unknown(exit_status: std::process::ExitStatus) -> String {
    exit_status
        .code()
        .map_or_else(|| String::from("<unknown>"), |code| code.to_string())
}
