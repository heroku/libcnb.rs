use libcnb_package::build::{build_buildpack_binaries, BuildBinariesError};
use libcnb_package::cross_compile::{cross_compile_assistance, CrossCompileAssistance};
use libcnb_package::output::{
    assemble_single_buildpack_directory, AssembleBuildpackDirectoryError,
};
use libcnb_package::CargoProfile;
use std::path::PathBuf;
use tempfile::{tempdir, TempDir};

/// Packages the current crate as a buildpack into a temporary directory.
pub(crate) fn package_crate_buildpack(
    cargo_profile: CargoProfile,
    target_triple: impl AsRef<str>,
) -> Result<TempDir, PackageCrateBuildpackError> {
    let cargo_manifest_dir = std::env::var("CARGO_MANIFEST_DIR")
        .map(PathBuf::from)
        .map_err(PackageCrateBuildpackError::CannotDetermineCrateDirectory)?;

    let cargo_env = match cross_compile_assistance(target_triple.as_ref()) {
        CrossCompileAssistance::HelpText(help_text) => {
            return Err(PackageCrateBuildpackError::CrossCompileConfigurationError(
                help_text,
            ));
        }
        CrossCompileAssistance::NoAssistance => Vec::new(),
        CrossCompileAssistance::Configuration { cargo_env } => cargo_env,
    };

    let buildpack_dir =
        tempdir().map_err(PackageCrateBuildpackError::CannotCreateBuildpackTempDirectory)?;

    let buildpack_binaries = build_buildpack_binaries(
        &cargo_manifest_dir,
        cargo_profile,
        &cargo_env,
        target_triple.as_ref(),
    )
    .map_err(PackageCrateBuildpackError::BuildBinariesError)?;

    assemble_single_buildpack_directory(
        buildpack_dir.path(),
        cargo_manifest_dir.join("buildpack.toml"),
        None,
        &buildpack_binaries,
    )
    .map_err(PackageCrateBuildpackError::AssembleBuildpackDirectory)?;

    Ok(buildpack_dir)
}

#[derive(Debug)]
pub(crate) enum PackageCrateBuildpackError {
    AssembleBuildpackDirectory(AssembleBuildpackDirectoryError),
    BuildBinariesError(BuildBinariesError),
    CannotCreateBuildpackTempDirectory(std::io::Error),
    CannotDetermineCrateDirectory(std::env::VarError),
    CrossCompileConfigurationError(String),
}
