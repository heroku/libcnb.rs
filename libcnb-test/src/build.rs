use fs_extra::dir::CopyOptions;
use libcnb_cargo::cross_compile::{cross_compile_assistance, CrossCompileAssistance};
use libcnb_cargo::{assemble_buildpack_directory, build_buildpack_binary, CargoProfile};
use std::path::{Path, PathBuf};
use tempfile::{tempdir, TempDir};

pub(crate) fn package_crate_buildpack(
    target_triple: impl AsRef<str>,
    app_dir: impl AsRef<Path>,
) -> std::io::Result<TempDir> {
    let cargo_manifest_dir = std::env::var("CARGO_MANIFEST_DIR")
        .map(PathBuf::from)
        .unwrap();

    let temp_app_dir = tempdir()?;

    let mut copy_options = CopyOptions::new();
    copy_options.content_only = true;

    fs_extra::dir::copy(
        &cargo_manifest_dir.join(app_dir),
        temp_app_dir.path(),
        &copy_options,
    )
    .unwrap();

    let cargo_env = match cross_compile_assistance(target_triple.as_ref()) {
        CrossCompileAssistance::Configuration { cargo_env } => cargo_env,
        _ => Default::default(),
    };

    // Package the buildpack
    let buildpack_dir = tempdir()?;
    let buildpack_binary_path = build_buildpack_binary(
        std::env::var("CARGO_MANIFEST_DIR").unwrap(),
        CargoProfile::Dev,
        target_triple.as_ref(),
        cargo_env,
    )
    .unwrap();

    assemble_buildpack_directory(
        buildpack_dir.path(),
        std::env::var("CARGO_MANIFEST_DIR")
            .map(PathBuf::from)
            .unwrap()
            .join("buildpack.toml"),
        &buildpack_binary_path,
    )
    .unwrap();

    Ok(buildpack_dir)
}
