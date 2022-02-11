use crate::config::{config_from_metadata, ConfigError};
use crate::CargoProfile;
use cargo_metadata::Metadata;
use std::collections::HashMap;
use std::ffi::OsStr;
use std::path::{Path, PathBuf};
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
/// Will return `Err` if the any build did not finish successfully, the configuration cannot be
/// read or the configured main buildpack binary does not exist.
pub fn build_buildpack_binaries<
    I: IntoIterator<Item = (K, V)>,
    K: AsRef<OsStr> + Clone,
    V: AsRef<OsStr> + Clone,
>(
    project_path: impl AsRef<Path>,
    cargo_metadata: &Metadata,
    cargo_profile: CargoProfile,
    cargo_env: I,
    target_triple: impl AsRef<str>,
) -> Result<BuildpackBinaries, BuildBinariesError> {
    let binary_target_names = binary_target_names(cargo_metadata);
    let config = config_from_metadata(cargo_metadata).map_err(BuildBinariesError::ConfigError)?;

    let cargo_env: Vec<(K, V)> = cargo_env.into_iter().collect();

    let buildpack_target_binary_path = if binary_target_names.contains(&config.buildpack_target) {
        build_binary(
            project_path.as_ref(),
            cargo_metadata,
            cargo_profile,
            cargo_env.clone(),
            target_triple.as_ref(),
            &config.buildpack_target,
        )
        .map_err(|error| BuildBinariesError::BuildError(config.buildpack_target.clone(), error))
    } else {
        Err(BuildBinariesError::MissingBuildpackTarget(
            config.buildpack_target.clone(),
        ))
    }?;

    let mut additional_target_binary_paths = HashMap::new();
    for additional_binary_target_name in binary_target_names
        .iter()
        .filter(|name| *name != &config.buildpack_target)
    {
        additional_target_binary_paths.insert(
            additional_binary_target_name.clone(),
            build_binary(
                project_path.as_ref(),
                cargo_metadata,
                cargo_profile,
                cargo_env.clone(),
                target_triple.as_ref(),
                additional_binary_target_name,
            )
            .map_err(|error| {
                BuildBinariesError::BuildError(additional_binary_target_name.clone(), error)
            })?,
        );
    }

    Ok(BuildpackBinaries {
        buildpack_target_binary_path,
        additional_target_binary_paths,
    })
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
pub fn build_binary<I: IntoIterator<Item = (K, V)>, K: AsRef<OsStr>, V: AsRef<OsStr>>(
    project_path: impl AsRef<Path>,
    cargo_metadata: &Metadata,
    cargo_profile: CargoProfile,
    cargo_env: I,
    target_triple: impl AsRef<str>,
    target_name: impl AsRef<str>,
) -> Result<PathBuf, BuildError> {
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
            .join(target_name.as_ref())
            .into_std_path_buf();

        Ok(binary_path)
    } else {
        Err(BuildError::UnexpectedCargoExitStatus(exit_status))
    }
}

#[derive(Debug)]
pub struct BuildpackBinaries {
    /// The path to the main buildpack binary
    pub buildpack_target_binary_path: PathBuf,
    /// Paths to additional binaries from the buildpack
    pub additional_target_binary_paths: HashMap<String, PathBuf>,
}

#[derive(Debug)]
pub enum BuildError {
    IoError(std::io::Error),
    UnexpectedCargoExitStatus(ExitStatus),
}

#[derive(Debug)]
pub enum BuildBinariesError {
    ConfigError(ConfigError),
    BuildError(String, BuildError),
    MissingBuildpackTarget(String),
}

/// Determines the names of all binary targets from the given Cargo metadata.
fn binary_target_names(cargo_metadata: &Metadata) -> Vec<String> {
    cargo_metadata
        .root_package()
        .map(|root_package| {
            root_package
                .targets
                .iter()
                .filter_map(|target| {
                    if target.kind.contains(&String::from("bin")) {
                        Some(target.name.clone())
                    } else {
                        None
                    }
                })
                .collect()
        })
        .unwrap_or_default()
}
