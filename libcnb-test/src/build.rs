use cargo_metadata::MetadataCommand;
use libcnb_package::build::{build_buildpack_binaries, BuildBinariesError};
use libcnb_package::cross_compile::{cross_compile_assistance, CrossCompileAssistance};
use libcnb_package::{assemble_buildpack_directory, CargoProfile};
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

    let cargo_metadata = MetadataCommand::new()
        .manifest_path(&cargo_manifest_dir.join("Cargo.toml"))
        .exec()
        .map_err(PackageCrateBuildpackError::CargoMetadataError)?;

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
        &cargo_metadata,
        cargo_profile,
        &cargo_env,
        target_triple.as_ref(),
    )
    .map_err(PackageCrateBuildpackError::BuildBinariesError)?;

    assemble_buildpack_directory(
        buildpack_dir.path(),
        &cargo_manifest_dir.join("buildpack.toml"),
        &buildpack_binaries,
    )
    .map_err(PackageCrateBuildpackError::CannotAssembleBuildpackDirectory)?;

    Ok(buildpack_dir)
}

#[derive(Debug)]
pub(crate) enum PackageCrateBuildpackError {
    BuildBinariesError(BuildBinariesError),
    CannotAssembleBuildpackDirectory(std::io::Error),
    CannotCreateBuildpackTempDirectory(std::io::Error),
    CannotDetermineCrateDirectory(std::env::VarError),
    CargoMetadataError(cargo_metadata::Error),
    CrossCompileConfigurationError(String),
}
