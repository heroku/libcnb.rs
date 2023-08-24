use crate::cargo::{
    cargo_binary_target_names, determine_buildpack_cargo_target_name, CargoProfile,
    DetermineBuildpackCargoTargetNameError,
};
use cargo_metadata::Metadata;
use std::collections::HashMap;
use std::ffi::OsString;
use std::path::PathBuf;
use std::process::{Command, ExitStatus};

/// Builds all buildpack binary targets using Cargo.
///
/// It uses libcnb configuration metadata in the Crate's `Cargo.toml` to determine which binary is
/// the main buildpack binary and which are additional ones.
///
/// See [`build_binary`] for details around the build process.
///
/// # Errors
///
/// Will return `Err` if any build did not finish successfully, the configuration cannot be
/// read or the configured main buildpack binary does not exist.
pub fn build_buildpack_binaries(
    cargo_metadata: &Metadata,
    cargo_profile: CargoProfile,
    cargo_env: &[(OsString, OsString)],
    target_triple: impl AsRef<str>,
) -> Result<BuildpackBinaries, BuildBinariesError> {
    let binary_target_names = cargo_binary_target_names(cargo_metadata);
    let buildpack_cargo_target = determine_buildpack_cargo_target_name(cargo_metadata)
        .map_err(BuildBinariesError::CannotDetermineBuildpackCargoTargetName)?;

    let mut buildpack_binaries = BuildpackBinaries::default();

    for binary_target_name in binary_target_names {
        let binary_path = build_binary(
            cargo_metadata,
            cargo_profile,
            cargo_env.to_owned(),
            target_triple.as_ref(),
            &buildpack_cargo_target,
        )
        .map_err(|error| BuildBinariesError::BuildError(buildpack_cargo_target.clone(), error))?;

        if binary_target_name == buildpack_cargo_target {
            buildpack_binaries.buildpack_target_binary_path = binary_path;
        } else {
            buildpack_binaries
                .additional_target_binary_paths
                .insert(binary_target_name, binary_path);
        }
    }

    Ok(buildpack_binaries)
}

/// Builds a binary using Cargo.
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
/// This function will write Cargo's output to stdout and stderr.
///
/// # Errors
///
/// Will return `Err` if the build did not finish successfully.
pub fn build_binary(
    cargo_metadata: &Metadata,
    cargo_profile: CargoProfile,
    mut cargo_env: Vec<(OsString, OsString)>,
    target_triple: impl AsRef<str>,
    target_name: impl AsRef<str>,
) -> Result<PathBuf, BuildError> {
    let mut cargo_args = vec![
        "build",
        "--bin",
        target_name.as_ref(),
        "--target",
        target_triple.as_ref(),
    ];
    match cargo_profile {
        CargoProfile::Dev => {
            // We enable stripping for dev builds too, since debug builds are extremely
            // large and can otherwise take a long time to be Docker copied into the
            // ephemeral builder image created by `pack build` for local development
            // and integration testing workflows. Since we are stripping the builds,
            // we also disable debug symbols to improve performance slightly, since
            // they will only be stripped out at the end of the build anyway.
            cargo_env.append(&mut vec![
                (
                    OsString::from("CARGO_PROFILE_DEV_DEBUG"),
                    OsString::from("false"),
                ),
                (
                    OsString::from("CARGO_PROFILE_DEV_STRIP"),
                    OsString::from("true"),
                ),
            ]);
        }
        CargoProfile::Release => {
            cargo_args.push("--release");
            cargo_env.push((
                OsString::from("CARGO_PROFILE_RELEASE_STRIP"),
                OsString::from("true"),
            ));
        }
    }

    let exit_status = Command::new("cargo")
        .args(cargo_args)
        .envs(cargo_env)
        .current_dir(&cargo_metadata.workspace_root)
        .spawn()
        .and_then(|mut child| child.wait())
        .map_err(BuildError::CargoProcessIoError)?;

    if exit_status.success() {
        let binary_path = cargo_metadata
            .target_directory
            .join(target_triple.as_ref())
            .join(match cargo_profile {
                CargoProfile::Dev => "debug",
                CargoProfile::Release => "release",
            })
            .join(target_name.as_ref())
            .into_std_path_buf();

        Ok(binary_path)
    } else {
        Err(BuildError::UnexpectedCargoExitStatus(exit_status))
    }
}

#[derive(Debug, Default)]
pub struct BuildpackBinaries {
    /// The path to the main buildpack binary
    pub buildpack_target_binary_path: PathBuf,
    /// Paths to additional binaries from the buildpack
    pub additional_target_binary_paths: HashMap<String, PathBuf>,
}

#[derive(thiserror::Error, Debug)]
pub enum BuildError {
    #[error("Error while running Cargo build process: {0}")]
    CargoProcessIoError(#[source] std::io::Error),
    #[error("Cargo unexpectedly exited with status {0}")]
    UnexpectedCargoExitStatus(ExitStatus),
}

#[derive(thiserror::Error, Debug)]
pub enum BuildBinariesError {
    #[error("Failed to determine Cargo target name for buildpack: {0}")]
    CannotDetermineBuildpackCargoTargetName(#[source] DetermineBuildpackCargoTargetNameError),
    #[error("Failed to build binary target {0}: {1}")]
    BuildError(String, #[source] BuildError),
}
