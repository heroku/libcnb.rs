use libcnb_common::toml_file::{read_toml_file, TomlFileError};
use libcnb_data::buildpack::{BuildpackDescriptor, BuildpackId};
use libcnb_package::buildpack_dependency_graph::{
    build_libcnb_buildpacks_dependency_graph, BuildBuildpackDependencyGraphError,
};
use libcnb_package::cross_compile::{cross_compile_assistance, CrossCompileAssistance};
use libcnb_package::dependency_graph::{get_dependencies, GetDependenciesError};
use libcnb_package::output::create_packaged_buildpack_dir_resolver;
use libcnb_package::package::PackageBuildpackError;
use libcnb_package::{find_cargo_workspace_root_dir, CargoProfile, FindCargoWorkspaceRootError};
use std::collections::BTreeMap;
use std::fs;
use std::path::{Path, PathBuf};

/// Packages the current crate as a buildpack into a temporary directory.
pub(crate) fn package_crate_buildpack(
    cargo_profile: CargoProfile,
    target_triple: impl AsRef<str>,
    cargo_manifest_dir: &Path,
    target_buildpack_dir: &Path,
) -> Result<PathBuf, PackageTestBuildpackError> {
    let buildpack_toml = cargo_manifest_dir.join("buildpack.toml");

    assert!(
        buildpack_toml.exists(),
        "Could not package directory as buildpack! No `buildpack.toml` file exists at {}",
        cargo_manifest_dir.display()
    );

    let buildpack_descriptor: BuildpackDescriptor = read_toml_file(buildpack_toml)
        .map_err(PackageTestBuildpackError::CannotReadBuildpackDescriptor)?;

    package_buildpack(
        &buildpack_descriptor.buildpack().id,
        cargo_profile,
        target_triple,
        cargo_manifest_dir,
        target_buildpack_dir,
    )
}

pub(crate) fn package_buildpack(
    buildpack_id: &BuildpackId,
    cargo_profile: CargoProfile,
    target_triple: impl AsRef<str>,
    cargo_manifest_dir: &Path,
    target_buildpack_dir: &Path,
) -> Result<PathBuf, PackageTestBuildpackError> {
    let cargo_build_env = match cross_compile_assistance(target_triple.as_ref()) {
        CrossCompileAssistance::HelpText(help_text) => {
            return Err(PackageTestBuildpackError::CrossCompileConfigurationError(
                help_text,
            ));
        }
        CrossCompileAssistance::NoAssistance => Vec::new(),
        CrossCompileAssistance::Configuration { cargo_env } => cargo_env,
    };

    let workspace_root_path = find_cargo_workspace_root_dir(cargo_manifest_dir)
        .map_err(PackageTestBuildpackError::FindCargoWorkspaceRoot)?;

    let buildpack_dir_resolver = create_packaged_buildpack_dir_resolver(
        target_buildpack_dir,
        cargo_profile,
        target_triple.as_ref(),
    );

    let buildpack_dependency_graph = build_libcnb_buildpacks_dependency_graph(&workspace_root_path)
        .map_err(PackageTestBuildpackError::BuildBuildpackDependencyGraph)?;

    let root_node = buildpack_dependency_graph
        .node_weights()
        .find(|node| node.buildpack_id == buildpack_id.clone());

    assert!(
        root_node.is_some(),
        "Could not package directory as buildpack! No buildpack with id `{buildpack_id}` exists in the workspace at {}",
        workspace_root_path.display()
    );

    let build_order = get_dependencies(
        &buildpack_dependency_graph,
        &[root_node.expect("The root node should exist")],
    )
    .map_err(PackageTestBuildpackError::GetDependencies)?;

    let mut packaged_buildpack_dirs = BTreeMap::new();
    for node in &build_order {
        let buildpack_destination_dir = buildpack_dir_resolver(&node.buildpack_id);

        fs::create_dir_all(&buildpack_destination_dir).unwrap();

        libcnb_package::package::package_buildpack(
            &node.path,
            cargo_profile,
            target_triple.as_ref(),
            &cargo_build_env,
            &buildpack_destination_dir,
            &packaged_buildpack_dirs,
        )
        .map_err(PackageTestBuildpackError::PackageBuildpack)?;

        packaged_buildpack_dirs.insert(node.buildpack_id.clone(), buildpack_destination_dir);
    }

    Ok(buildpack_dir_resolver(buildpack_id))
}

#[derive(Debug)]
pub(crate) enum PackageTestBuildpackError {
    CannotReadBuildpackDescriptor(TomlFileError),
    BuildBuildpackDependencyGraph(BuildBuildpackDependencyGraphError),
    CrossCompileConfigurationError(String),
    FindCargoWorkspaceRoot(FindCargoWorkspaceRootError),
    GetDependencies(GetDependenciesError<BuildpackId>),
    PackageBuildpack(PackageBuildpackError),
}
