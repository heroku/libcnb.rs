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
use std::path::PathBuf;
use tempfile::{tempdir, TempDir};

/// Packages the current crate as a buildpack into a temporary directory.
pub(crate) fn package_crate_buildpack(
    cargo_profile: CargoProfile,
    target_triple: impl AsRef<str>,
) -> Result<PackagedBuildpackDir, PackageTestBuildpackError> {
    let cargo_manifest_dir = std::env::var("CARGO_MANIFEST_DIR")
        .map(PathBuf::from)
        .map_err(PackageTestBuildpackError::CannotDetermineCrateDirectory)?;

    let buildpack_toml = cargo_manifest_dir.join("buildpack.toml");

    assert!(
        buildpack_toml.exists(),
        "Could not package directory as buildpack! No `buildpack.toml` file exists at {}",
        cargo_manifest_dir.display()
    );

    let buildpack_descriptor: BuildpackDescriptor =
        read_toml_file(buildpack_toml).map_err(PackageTestBuildpackError::CannotReadBuildpack)?;

    package_buildpack(
        &buildpack_descriptor.buildpack().id,
        cargo_profile,
        target_triple,
    )
}

pub(crate) fn package_buildpack(
    buildpack_id: &BuildpackId,
    cargo_profile: CargoProfile,
    target_triple: impl AsRef<str>,
) -> Result<PackagedBuildpackDir, PackageTestBuildpackError> {
    let cargo_manifest_dir = std::env::var("CARGO_MANIFEST_DIR")
        .map(PathBuf::from)
        .map_err(PackageTestBuildpackError::CannotDetermineCrateDirectory)?;

    let cargo_build_env = match cross_compile_assistance(target_triple.as_ref()) {
        CrossCompileAssistance::HelpText(help_text) => {
            return Err(PackageTestBuildpackError::CrossCompileConfigurationError(
                help_text,
            ));
        }
        CrossCompileAssistance::NoAssistance => Vec::new(),
        CrossCompileAssistance::Configuration { cargo_env } => cargo_env,
    };

    let workspace_root_path = find_cargo_workspace_root_dir(&cargo_manifest_dir)
        .map_err(PackageTestBuildpackError::FindCargoWorkspace)?;

    let package_dir =
        tempdir().map_err(PackageTestBuildpackError::CannotCreateBuildpackTempDirectory)?;

    let buildpack_dir_resolver = create_packaged_buildpack_dir_resolver(
        package_dir.as_ref(),
        cargo_profile,
        target_triple.as_ref(),
    );

    // TODO: this could accidentally detect a packaged meta-buildpack twice. how should we ignore directories
    //       containing packaged buildpacks that might get detected now that the user controls --package-dir?
    //       - support .gitignore or some other persistent configuration?
    //       - during packaging should we create some type of file that indicates the output is a packaged buildpack?
    let buildpack_dependency_graph =
        build_libcnb_buildpacks_dependency_graph(&workspace_root_path, &[])
            .map_err(PackageTestBuildpackError::CreateBuildpackDependencyGraph)?;

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

    Ok(PackagedBuildpackDir {
        _root: package_dir,
        path: buildpack_dir_resolver(buildpack_id),
    })
}

pub(crate) struct PackagedBuildpackDir {
    _root: TempDir,
    pub(crate) path: PathBuf,
}

#[derive(Debug)]
pub(crate) enum PackageTestBuildpackError {
    CannotCreateBuildpackTempDirectory(std::io::Error),
    CannotDetermineCrateDirectory(std::env::VarError),
    CannotReadBuildpack(TomlFileError),
    CreateBuildpackDependencyGraph(BuildBuildpackDependencyGraphError),
    CrossCompileConfigurationError(String),
    FindCargoWorkspace(FindCargoWorkspaceRootError),
    GetDependencies(GetDependenciesError<BuildpackId>),
    PackageBuildpack(PackageBuildpackError),
}
