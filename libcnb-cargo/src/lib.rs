// https://rust-lang.github.io/rust-clippy/stable/index.html
#![warn(clippy::pedantic)]
// This lint is too noisy and enforces a style that reduces readability in many cases.
#![allow(clippy::module_name_repetitions)]

pub mod cross_compile;

use cargo_metadata::MetadataCommand;
use libcnb_data::buildpack::SingleBuildpackDescriptor;
use std::ffi::OsStr;
use std::fs;
use std::io;
use std::path::{Path, PathBuf};
use std::process::{Command, ExitStatus};

/// Builds a buildpack binary using Cargo.
///
/// It is designed to handle cross-compilation without requiring custom configuration in the Cargo
/// manifest of the user's buildpack. The triple for the target platform is a mandatory
/// argument of this function.
///
/// Depending on the host platform, this function will try to set the required cross compilation
/// settings automatically. Please note that only selected host platforms and targets are supported.
/// For other combinations, compilation might fail, surfacing cross-compile related errors to the
/// user.
///
/// In many cases, cross-compilation requires external tools such as compilers and linkers to be
/// installed on the user's machine. When a tool is missing, a `BuildError::CrossCompileError` is
/// returned which provides additional information. Use the `cross_compile::cross_compile_help`
/// function to obtain human-readable instructions on how to setup the required tools.
///
/// This function currently only supports projects with a single binary target. If the project
/// does not contain exactly one target, the appropriate `BuildError` is returned.
///
/// This function will write Cargo's output to stdout and stderr.
///
/// # Errors
///
/// Will return `Err` if the build did not finish successfully.
pub fn build_buildpack_binary<I, K, V>(
    project_path: impl AsRef<Path>,
    cargo_profile: CargoProfile,
    target_triple: impl AsRef<str>,
    cargo_env: I,
) -> Result<PathBuf, BuildError>
where
    I: IntoIterator<Item = (K, V)>,
    K: AsRef<OsStr>,
    V: AsRef<OsStr>,
{
    let cargo_metadata = MetadataCommand::new()
        .manifest_path(project_path.as_ref().join("Cargo.toml"))
        .exec()
        .map_err(BuildError::MetadataError)?;

    let buildpack_cargo_package = cargo_metadata
        .root_package()
        .ok_or(BuildError::CouldNotFindRootPackage)?;

    let target = match buildpack_cargo_package.targets.as_slice() {
        [] => Err(BuildError::NoTargetsFound),
        [single_target] => Ok(single_target),
        _ => Err(BuildError::MultipleTargetsFound),
    }?;

    let mut cargo_args = vec!["build", "--target", target_triple.as_ref()];
    match cargo_profile {
        CargoProfile::Dev => {}
        CargoProfile::Release => cargo_args.push("--release"),
    }

    let exit_status = Command::new("cargo")
        .args(cargo_args)
        .envs(cargo_env)
        .current_dir(&project_path)
        .spawn()
        .and_then(|mut child| child.wait())
        .map_err(BuildError::IoError)?;

    if exit_status.success() {
        let binary_path = cargo_metadata
            .target_directory
            .join(target_triple.as_ref())
            .join(match cargo_profile {
                CargoProfile::Dev => "debug",
                CargoProfile::Release => "release",
            })
            .join(&target.name)
            .into_std_path_buf();

        Ok(binary_path)
    } else {
        Err(BuildError::UnexpectedExitStatus(exit_status))
    }
}

#[derive(Debug)]
pub enum BuildError {
    IoError(std::io::Error),
    UnexpectedExitStatus(ExitStatus),
    NoTargetsFound,
    MultipleTargetsFound,
    MetadataError(cargo_metadata::Error),
    CouldNotFindRootPackage,
}

#[derive(Copy, Clone)]
pub enum CargoProfile {
    Dev,
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
/// Will return `Err` if the buildpack directory already exists or could not be assembled.
pub fn assemble_buildpack_directory(
    destination_path: impl AsRef<Path>,
    buildpack_descriptor_path: impl AsRef<Path>,
    buildpack_binary_path: impl AsRef<Path>,
) -> std::io::Result<()> {
    if destination_path.as_ref().exists() {
        Err(io::Error::new(
            io::ErrorKind::AlreadyExists,
            "Destination path already exists!",
        ))
    } else {
        fs::create_dir_all(destination_path.as_ref())?;

        fs::copy(
            buildpack_descriptor_path.as_ref(),
            destination_path.as_ref().join("buildpack.toml"),
        )?;

        let bin_path = destination_path.as_ref().join("bin");
        fs::create_dir_all(&bin_path)?;

        fs::copy(buildpack_binary_path.as_ref(), bin_path.join("build"))?;
        create_file_symlink("build", bin_path.join("detect"))?;

        Ok(())
    }
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
    buildpack_descriptor.buildpack.id.replace("/", "_")
}
