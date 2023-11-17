use libcnb_common::toml_file::{read_toml_file, TomlFileError};
use libcnb_data::buildpack::{BuildpackDescriptor, BuildpackId};
use libcnb_package::buildpack_dependency_graph::{
    build_libcnb_buildpacks_dependency_graph, BuildBuildpackDependencyGraphError,
};
use libcnb_package::cross_compile::{cross_compile_assistance, CrossCompileAssistance};
use libcnb_package::dependency_graph::{get_dependencies, GetDependenciesError};
use libcnb_package::output::create_packaged_buildpack_dir_resolver;
use libcnb_package::{find_cargo_workspace_root_dir, CargoProfile, FindCargoWorkspaceRootError};
use std::collections::BTreeMap;
use std::path::{Path, PathBuf};
use std::{fs, io};

/// Packages the current crate as a buildpack into the provided directory.
pub(crate) fn package_crate_buildpack(
    cargo_profile: CargoProfile,
    target_triple: impl AsRef<str>,
    cargo_manifest_dir: &Path,
    target_buildpack_dir: &Path,
) -> Result<PathBuf, PackageBuildpackError> {
    let buildpack_toml = cargo_manifest_dir.join("buildpack.toml");

    if !buildpack_toml.exists() {
        return Err(PackageBuildpackError::BuildpackDescriptorNotFound(
            buildpack_toml,
        ));
    }

    let buildpack_descriptor: BuildpackDescriptor = read_toml_file(buildpack_toml)
        .map_err(PackageBuildpackError::CannotReadBuildpackDescriptor)?;

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
) -> Result<PathBuf, PackageBuildpackError> {
    let cargo_build_env = match cross_compile_assistance(target_triple.as_ref()) {
        CrossCompileAssistance::HelpText(help_text) => {
            return Err(PackageBuildpackError::CrossCompileToolchainNotFound(
                help_text,
            ));
        }
        CrossCompileAssistance::NoAssistance => Vec::new(),
        CrossCompileAssistance::Configuration { cargo_env } => cargo_env,
    };

    let workspace_root_path = find_cargo_workspace_root_dir(cargo_manifest_dir)
        .map_err(PackageBuildpackError::FindCargoWorkspaceRoot)?;

    let buildpack_dir_resolver = create_packaged_buildpack_dir_resolver(
        target_buildpack_dir,
        cargo_profile,
        target_triple.as_ref(),
    );

    let buildpack_dependency_graph = build_libcnb_buildpacks_dependency_graph(&workspace_root_path)
        .map_err(PackageBuildpackError::BuildBuildpackDependencyGraph)?;

    let root_node = buildpack_dependency_graph
        .node_weights()
        .find(|node| &node.buildpack_id == buildpack_id)
        .ok_or_else(|| {
            PackageBuildpackError::BuildpackIdNotFound(buildpack_id.clone(), workspace_root_path)
        })?;

    let build_order = get_dependencies(&buildpack_dependency_graph, &[root_node])
        .map_err(PackageBuildpackError::GetDependencies)?;

    let mut packaged_buildpack_dirs = BTreeMap::new();
    for node in &build_order {
        let buildpack_destination_dir = buildpack_dir_resolver(&node.buildpack_id);

        fs::create_dir_all(&buildpack_destination_dir).map_err(|error| {
            PackageBuildpackError::CannotCreateDirectory(buildpack_destination_dir.clone(), error)
        })?;

        libcnb_package::package::package_buildpack(
            &node.path,
            cargo_profile,
            target_triple.as_ref(),
            &cargo_build_env,
            &buildpack_destination_dir,
            &packaged_buildpack_dirs,
        )
        .map_err(PackageBuildpackError::PackageBuildpack)?;

        packaged_buildpack_dirs.insert(node.buildpack_id.clone(), buildpack_destination_dir);
    }

    Ok(buildpack_dir_resolver(buildpack_id))
}

#[derive(thiserror::Error, Debug)]
pub(crate) enum PackageBuildpackError {
    #[error("Couldn't find a buildpack.toml file at {0}")]
    BuildpackDescriptorNotFound(PathBuf),
    #[error("Couldn't find a buildpack with ID '{0}' in the workspace at {1}")]
    BuildpackIdNotFound(BuildpackId, PathBuf),
    #[error("Couldn't create directory {0}: {1}")]
    CannotCreateDirectory(PathBuf, io::Error),
    #[error("Couldn't read buildpack.toml: {0}")]
    CannotReadBuildpackDescriptor(TomlFileError),
    #[error("Couldn't calculate buildpack dependency graph: {0}")]
    BuildBuildpackDependencyGraph(BuildBuildpackDependencyGraphError),
    #[error("Couldn't find cross-compilation toolchain.\n\n{0}")]
    CrossCompileToolchainNotFound(String),
    #[error("Couldn't find Cargo workspace root: {0}")]
    FindCargoWorkspaceRoot(FindCargoWorkspaceRootError),
    #[error("Couldn't get buildpack dependencies: {0}")]
    GetDependencies(GetDependenciesError<BuildpackId>),
    #[error(transparent)]
    PackageBuildpack(libcnb_package::package::PackageBuildpackError),
}
