use libcnb_cargo::cross_compile::{cross_compile_assistance, CrossCompileAssistance};
use libcnb_cargo::{
    assemble_buildpack_directory, build_buildpack_binary, BuildError, CargoProfile,
};
use std::path::PathBuf;
use tempfile::{tempdir, TempDir};

/// Packages the current crate as a buildpack into a temporary directory.
pub(crate) fn package_crate_buildpack(
    target_triple: impl AsRef<str>,
) -> Result<TempDir, PackageCrateBuildpackError> {
    let cargo_manifest_dir = std::env::var("CARGO_MANIFEST_DIR")
        .map(PathBuf::from)
        .map_err(PackageCrateBuildpackError::CannotDetermineCrateDirectory)?;

    let cargo_env = match cross_compile_assistance(target_triple.as_ref()) {
        CrossCompileAssistance::Configuration { cargo_env } => cargo_env,
        _ => vec![],
    };

    let buildpack_dir =
        tempdir().map_err(PackageCrateBuildpackError::CannotCreateBuildpackTempDirectory)?;

    let buildpack_binary_path = build_buildpack_binary(
        &cargo_manifest_dir,
        CargoProfile::Dev,
        target_triple.as_ref(),
        cargo_env,
    )
    .map_err(PackageCrateBuildpackError::BuildError)?;

    assemble_buildpack_directory(
        buildpack_dir.path(),
        &cargo_manifest_dir.join("buildpack.toml"),
        &buildpack_binary_path,
    )
    .map_err(PackageCrateBuildpackError::CannotAssembleBuildpackDirectory)?;

    Ok(buildpack_dir)
}

#[derive(Debug)]
pub(crate) enum PackageCrateBuildpackError {
    CannotCreateBuildpackTempDirectory(std::io::Error),
    BuildError(BuildError),
    CannotAssembleBuildpackDirectory(std::io::Error),
    CannotDetermineCrateDirectory(std::env::VarError),
}
