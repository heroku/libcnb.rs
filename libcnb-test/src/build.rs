use crate::pack;
use crate::pack::PackBuildpackPackageCommand;
use libcnb_data::buildpack::{BuildpackDescriptor, BuildpackId, BuildpackIdError};
use libcnb_package::build::{build_buildpack_binaries, BuildBinariesError};
use libcnb_package::buildpack_package::{
    read_buildpack_package, BuildpackPackage, ReadBuildpackPackageError,
};
use libcnb_package::cross_compile::{cross_compile_assistance, CrossCompileAssistance};
use libcnb_package::dependency_graph::{
    create_dependency_graph, get_dependencies, CreateDependencyGraphError, DependencyNode,
    GetDependenciesError,
};
use libcnb_package::output::{
    assemble_meta_buildpack_directory, assemble_single_buildpack_directory,
    AssembleBuildpackDirectoryError, BuildpackOutputDirectoryLocator,
};
use libcnb_package::{
    find_buildpack_dirs, find_cargo_workspace, CargoProfile, FindCargoWorkspaceError,
    ReadBuildpackDataError, ReadBuildpackageDataError,
};
use std::env::current_dir;
use std::ffi::OsString;
use std::iter::repeat_with;
use std::path::{Path, PathBuf};
use tempfile::tempdir;

/// Packages the current crate as a buildpack into a temporary directory.
pub(crate) fn package_crate_buildpack(
    cargo_profile: CargoProfile,
    target_triple: impl AsRef<str>,
) -> Result<String, PackageBuildpackError> {
    let cargo_manifest_dir = std::env::var("CARGO_MANIFEST_DIR")
        .map(PathBuf::from)
        .map_err(PackageBuildpackError::CannotDetermineCrateDirectory)?;
    package_buildpack(&cargo_manifest_dir, cargo_profile, target_triple)
}

pub(crate) fn package_buildpack(
    buildpack_dir: &Path,
    cargo_profile: CargoProfile,
    target_triple: impl AsRef<str>,
) -> Result<String, PackageBuildpackError> {
    let buildpack_dir = if buildpack_dir.is_relative() {
        current_dir()
            .and_then(|current_dir| current_dir.join(buildpack_dir).canonicalize())
            .map_err(PackageBuildpackError::CannotGetCurrentDirectory)?
    } else {
        buildpack_dir.to_path_buf()
    };

    let cargo_build_env = match cross_compile_assistance(target_triple.as_ref()) {
        CrossCompileAssistance::HelpText(help_text) => {
            return Err(PackageBuildpackError::CrossCompileConfigurationError(
                help_text,
            ));
        }
        CrossCompileAssistance::NoAssistance => Vec::new(),
        CrossCompileAssistance::Configuration { cargo_env } => cargo_env,
    };

    let workspace_dir =
        find_cargo_workspace(&buildpack_dir).map_err(PackageBuildpackError::FindCargoWorkspace)?;

    let buildpack_dirs = find_buildpack_dirs(&workspace_dir, &[workspace_dir.join("target")])
        .map_err(PackageBuildpackError::FindBuildpackDirs)?;

    let buildpack_packages = buildpack_dirs
        .into_iter()
        .map(read_buildpack_package)
        .collect::<Result<Vec<_>, _>>()
        .map_err(|error| match error {
            ReadBuildpackPackageError::ReadBuildpackDataError(e) => {
                PackageBuildpackError::ReadBuildpackData(e)
            }
            ReadBuildpackPackageError::ReadBuildpackageDataError(e) => {
                PackageBuildpackError::ReadBuildpackageData(e)
            }
        })?;

    let buildpack_packages_graph = create_dependency_graph(buildpack_packages)
        .map_err(PackageBuildpackError::CreateDependencyGraph)?;

    let buildpack_packages_requested = buildpack_packages_graph
        .node_weights()
        .filter(|buildpack_package| buildpack_package.path == buildpack_dir)
        .collect::<Vec<_>>();

    assert!(
        !buildpack_packages_requested.is_empty(),
        "Could not package directory as buildpack: {}",
        buildpack_dir.display()
    );

    let build_order = get_dependencies(&buildpack_packages_graph, &buildpack_packages_requested)
        .map_err(PackageBuildpackError::GetDependencies)?;

    let root_output_dir =
        tempdir().map_err(PackageBuildpackError::CannotCreateBuildpackTempDirectory)?;

    let buildpack_output_directory_locator = BuildpackOutputDirectoryLocator::new(
        root_output_dir.path().to_path_buf(),
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
                )?;
            }
            BuildpackDescriptor::Meta(_) => {
                package_meta_buildpack(
                    buildpack_package,
                    &target_dir,
                    &buildpack_output_directory_locator,
                )?;
            }
        }
    }

    let target_buildpack_id = buildpack_packages_requested
        .iter()
        .map(|buildpack_package| buildpack_package.buildpack_id())
        .next()
        .expect("The list of requested buildpacks should only contain a single item (i.e.; the buildpack being tested)");

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
) -> Result<(), PackageBuildpackError> {
    let buildpack_binaries = build_buildpack_binaries(
        &buildpack_package.path,
        cargo_profile,
        cargo_build_env,
        target_triple,
    )
    .map_err(PackageBuildpackError::BuildBuildpackBinaries)?;

    assemble_single_buildpack_directory(
        target_dir,
        &buildpack_package.buildpack_data.buildpack_descriptor_path,
        buildpack_package
            .buildpackage_data
            .as_ref()
            .map(|data| &data.buildpackage_descriptor),
        &buildpack_binaries,
    )
    .map_err(PackageBuildpackError::AssembleBuildpackDirectory)
}

fn package_meta_buildpack(
    buildpack_package: &BuildpackPackage,
    target_dir: &Path,
    buildpack_output_directory_locator: &BuildpackOutputDirectoryLocator,
) -> Result<(), PackageBuildpackError> {
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
    .map_err(PackageBuildpackError::AssembleBuildpackDirectory)
}

#[derive(Debug)]
pub(crate) enum PackageBuildpackError {
    CannotCreateBuildpackTempDirectory(std::io::Error),
    CannotDetermineCrateDirectory(std::env::VarError),
    CrossCompileConfigurationError(String),
    CannotGetCurrentDirectory(std::io::Error),
    FindCargoWorkspace(FindCargoWorkspaceError),
    FindBuildpackDirs(std::io::Error),
    ReadBuildpackData(ReadBuildpackDataError),
    ReadBuildpackageData(ReadBuildpackageDataError),
    CreateDependencyGraph(CreateDependencyGraphError<BuildpackId, BuildpackIdError>),
    GetDependencies(GetDependenciesError<BuildpackId>),
    BuildBuildpackBinaries(BuildBinariesError),
    AssembleBuildpackDirectory(AssembleBuildpackDirectoryError),
}
