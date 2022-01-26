use fs_extra::dir::CopyOptions;
use libcnb_cargo::cross_compile::{cross_compile_assistance, CrossCompileAssistance};
use libcnb_cargo::{
    assemble_buildpack_directory, build_buildpack_binary, BuildError, CargoProfile,
};
use std::path::{Path, PathBuf};
use tempfile::{tempdir, TempDir};

/// Packages the current crate as a buildpack into a temporary directory.
pub(crate) fn package_crate_buildpack(
    target_triple: impl AsRef<str>,
    app_dir: impl AsRef<Path>,
) -> Result<TempDir, PackageCrateBuildpackError> {
    let cargo_manifest_dir = std::env::var("CARGO_MANIFEST_DIR")
        .map(PathBuf::from)
        .map_err(PackageCrateBuildpackError::CannotDetermineCrateDirectory)?;

    let temp_app_dir =
        tempdir().map_err(PackageCrateBuildpackError::CannotCreateAppTempDirectory)?;

    let mut copy_options = CopyOptions::new();
    copy_options.content_only = true;

    fs_extra::dir::copy(
        &cargo_manifest_dir.join(app_dir),
        temp_app_dir.path(),
        &copy_options,
    )
    .map_err(PackageCrateBuildpackError::CannotCopyAppToTempDirectory)?;

    let cargo_env = match cross_compile_assistance(target_triple.as_ref()) {
        CrossCompileAssistance::Configuration { cargo_env } => cargo_env,
        _ => Default::default(),
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
    CannotCreateAppTempDirectory(std::io::Error),
    CannotCreateBuildpackTempDirectory(std::io::Error),
    BuildError(BuildError),
    CannotCopyAppToTempDirectory(fs_extra::error::Error),
    CannotAssembleBuildpackDirectory(std::io::Error),
    CannotDetermineCrateDirectory(std::env::VarError),
}
