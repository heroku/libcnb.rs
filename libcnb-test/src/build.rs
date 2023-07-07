use crate::pack;
use crate::pack::PackBuildpackPackageCommand;
use libcnb_data::buildpack::BuildpackDescriptor;
use libcnb_package::build::build_buildpack_binaries;
use libcnb_package::buildpack_package::{read_buildpack_package, BuildpackPackage};
use libcnb_package::cross_compile::{cross_compile_assistance, CrossCompileAssistance};
use libcnb_package::dependency_graph::{create_dependency_graph, get_dependencies, DependencyNode};
use libcnb_package::output::{
    assemble_meta_buildpack_directory, assemble_single_buildpack_directory,
    BuildpackOutputDirectoryLocator,
};
use libcnb_package::{find_buildpack_dirs, find_cargo_workspace, CargoProfile};
use std::ffi::OsString;
use std::iter::repeat_with;
use std::path::{Path, PathBuf};
use tempfile::tempdir;

/// Packages the current crate as a buildpack into a temporary directory.
pub(crate) fn package_crate_buildpack(
    cargo_profile: CargoProfile,
    target_triple: impl AsRef<str>,
) -> Result<String, PackageCrateBuildpackError> {
    let cargo_manifest_dir = std::env::var("CARGO_MANIFEST_DIR")
        .map(PathBuf::from)
        .map_err(PackageCrateBuildpackError::CannotDetermineCrateDirectory)?;
    package_buildpack(&cargo_manifest_dir, cargo_profile, target_triple)
}

pub(crate) fn package_buildpack(
    buildpack_dir: &Path,
    cargo_profile: CargoProfile,
    target_triple: impl AsRef<str>,
) -> Result<String, PackageCrateBuildpackError> {
    let buildpack_dir = if buildpack_dir.is_relative() {
        std::env::current_dir()
            .unwrap()
            .join(buildpack_dir)
            .canonicalize()
            .unwrap()
    } else {
        buildpack_dir.to_path_buf()
    };

    let cargo_build_env = match cross_compile_assistance(target_triple.as_ref()) {
        CrossCompileAssistance::HelpText(help_text) => {
            return Err(PackageCrateBuildpackError::CrossCompileConfigurationError(
                help_text,
            ));
        }
        CrossCompileAssistance::NoAssistance => Vec::new(),
        CrossCompileAssistance::Configuration { cargo_env } => cargo_env,
    };

    let workspace_dir = find_cargo_workspace(&buildpack_dir).unwrap();

    let output_dir =
        tempdir().map_err(PackageCrateBuildpackError::CannotCreateBuildpackTempDirectory)?;

    let buildpack_dirs =
        find_buildpack_dirs(&workspace_dir, &[output_dir.path().to_path_buf()]).unwrap();

    let buildpack_packages = buildpack_dirs
        .into_iter()
        .map(read_buildpack_package)
        .collect::<Result<Vec<_>, _>>()
        .unwrap();

    let buildpack_packages_graph = create_dependency_graph(buildpack_packages).unwrap();

    let buildpack_packages_requested = buildpack_packages_graph
        .node_weights()
        .filter(|buildpack_package| buildpack_package.path == buildpack_dir)
        .collect::<Vec<_>>();

    let build_order =
        get_dependencies(&buildpack_packages_graph, &buildpack_packages_requested).unwrap();

    let buildpack_output_directory_locator = BuildpackOutputDirectoryLocator::new(
        output_dir.path().to_path_buf(),
        cargo_profile,
        target_triple.as_ref().to_string(),
    );

    for buildpack_package in &build_order {
        let target_dir = buildpack_output_directory_locator.get(&buildpack_package.id());
        match buildpack_package.buildpack_data.buildpack_descriptor {
            BuildpackDescriptor::Single(_) => {
                package_single_buildpack(
                    buildpack_package,
                    &target_dir,
                    cargo_profile,
                    &cargo_build_env,
                    target_triple.as_ref(),
                );
            }
            BuildpackDescriptor::Meta(_) => {
                package_meta_buildpack(
                    buildpack_package,
                    &target_dir,
                    &buildpack_output_directory_locator,
                );
            }
        }
    }

    let target_buildpack_id = buildpack_packages_requested
        .iter()
        .map(|buildpack_package| buildpack_package.buildpack_id())
        .next()
        .unwrap();

    let output_dir = buildpack_output_directory_locator.get(target_buildpack_id);

    let buildpack_image = format!(
        "{target_buildpack_id}_{}",
        repeat_with(fastrand::lowercase)
            .take(30)
            .collect::<String>()
    );

    let buildpack_package_command =
        PackBuildpackPackageCommand::new(&buildpack_image, output_dir.join("package.toml"));

    pack::run_buildpack_package_command(buildpack_package_command);

    Ok(buildpack_image)
}

fn package_single_buildpack(
    buildpack_package: &BuildpackPackage,
    target_dir: &Path,
    cargo_profile: CargoProfile,
    cargo_build_env: &[(OsString, OsString)],
    target_triple: &str,
) {
    let buildpack_binaries = build_buildpack_binaries(
        &buildpack_package.path,
        cargo_profile,
        cargo_build_env,
        target_triple,
    )
    .unwrap();
    assemble_single_buildpack_directory(
        target_dir,
        &buildpack_package.buildpack_data.buildpack_descriptor_path,
        buildpack_package
            .buildpackage_data
            .as_ref()
            .map(|data| &data.buildpackage_descriptor),
        &buildpack_binaries,
    )
    .unwrap();
}

fn package_meta_buildpack(
    buildpack_package: &BuildpackPackage,
    target_dir: &Path,
    buildpack_output_directory_locator: &BuildpackOutputDirectoryLocator,
) {
    assemble_meta_buildpack_directory(
        target_dir,
        &buildpack_package.path,
        &buildpack_package.buildpack_data.buildpack_descriptor_path,
        buildpack_package
            .buildpackage_data
            .as_ref()
            .map(|data| &data.buildpackage_descriptor),
        buildpack_output_directory_locator,
    )
    .unwrap();
}

#[derive(Debug)]
pub(crate) enum PackageCrateBuildpackError {
    CannotCreateBuildpackTempDirectory(std::io::Error),
    CannotDetermineCrateDirectory(std::env::VarError),
    CrossCompileConfigurationError(String),
}
