// https://rust-lang.github.io/rust-clippy/stable/index.html
#![warn(clippy::pedantic)]
// This lint is too noisy and enforces a style that reduces readability in many cases.
#![allow(clippy::module_name_repetitions)]

pub mod cross_compile;

use cargo_metadata::MetadataCommand;
use cross_compile::CrossCompileError;
use flate2::write::GzEncoder;
use flate2::Compression;
use libcnb_data::buildpack::BuildpackToml;
use std::fs;
use std::io::Write;
use std::path::{Path, PathBuf};
use std::process::{Command, ExitStatus};
use tar::{EntryType, Header};

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
/// Will return `Err` if the build did not finish successfully.
pub fn build_buildpack_binary(
    project_path: impl AsRef<Path>,
    cargo_profile: CargoProfile,
    target_triple: impl AsRef<str>,
) -> Result<PathBuf, BuildError> {
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
        .envs(
            cross_compile::cross_compile_env(target_triple.as_ref())
                .map_err(BuildError::CrossCompileError)?,
        )
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
    CrossCompileError(CrossCompileError),
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
/// Will return `Err` if the buildpack data could not be read successfully.
pub fn read_buildpack_data(
    project_path: impl AsRef<Path>,
) -> Result<BuildpackData<Option<toml::Value>>, BuildpackDataError> {
    let buildpack_toml_path = project_path.as_ref().join("buildpack.toml");

    fs::read_to_string(&buildpack_toml_path)
        .map_err(BuildpackDataError::IoError)
        .and_then(|file_contents| {
            toml::from_str(&file_contents).map_err(BuildpackDataError::DeserializationError)
        })
        .map(|buildpack_toml| BuildpackData {
            buildpack_toml_path,
            buildpack_toml,
        })
}

#[derive(Debug)]
pub enum BuildpackDataError {
    IoError(std::io::Error),
    DeserializationError(toml::de::Error),
}

pub struct BuildpackData<BM> {
    pub buildpack_toml_path: PathBuf,
    pub buildpack_toml: BuildpackToml<BM>,
}

/// Assembles a buildpack tarball and writes it to the given destination path.
///
/// Assembly of the tarball follows the constraints set by the libcnb framework. For example,
/// the buildpack binary is only stored once and symlinks are used to refer to it when the CNB
/// spec requires different file(name)s.
///
/// This function will not validate if the buildpack descriptor at the given path is valid and will
/// use it as-is.
///
/// # Errors
/// Will return `Err` if the tarball could not be assembled successfully.
pub fn assemble_buildpack_tarball(
    destination_path: impl AsRef<Path>,
    buildpack_toml_path: impl AsRef<Path>,
    buildpack_binary_path: impl AsRef<Path>,
) -> std::io::Result<()> {
    let destination_file = fs::File::create(destination_path.as_ref())?;
    let mut buildpack_toml_file = fs::File::open(buildpack_toml_path.as_ref())?;
    let mut buildpack_binary_file = fs::File::open(buildpack_binary_path.as_ref())?;

    let mut tar_builder =
        tar::Builder::new(GzEncoder::new(destination_file, Compression::default()));

    tar_builder.append_file("buildpack.toml", &mut buildpack_toml_file)?;
    tar_builder.append_file("bin/build", &mut buildpack_binary_file)?;

    // Build a symlink header to link `bin/detect` to `bin/build`
    let mut header = Header::new_gnu();
    header.set_entry_type(EntryType::Symlink);
    header.set_path("bin/detect")?;
    header.set_link_name("build")?;
    header.set_size(0);
    header.set_mtime(
        std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .unwrap_or(std::time::Duration::ZERO)
            .as_secs(),
    );
    header.set_cksum();

    tar_builder.append(&header, &[][..])?;

    tar_builder.into_inner()?.finish()?.flush()
}

/// Construct a good default filename for a buildpack tarball.
///
/// It uses the given buildpack metadata and cargo profile to construct a good default name for the
/// buildpack tarball. This function ensures the resulting filename is valid and does not contain
/// problematic characters such as `/`.
pub fn default_buildpack_tarball_filename<BM>(
    buildpack_toml: &BuildpackToml<BM>,
    cargo_profile: CargoProfile,
) -> String {
    format!(
        "{}_{}_{}.tar.gz",
        buildpack_toml.buildpack.id.replace("/", "_"),
        buildpack_toml.buildpack.version,
        match cargo_profile {
            CargoProfile::Dev => "dev",
            CargoProfile::Release => "release",
        }
    )
}
