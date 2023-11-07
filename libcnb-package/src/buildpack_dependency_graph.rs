use crate::buildpack_kind::determine_buildpack_kind;
use crate::buildpack_kind::BuildpackKind;
use crate::dependency_graph::{
    create_dependency_graph, CreateDependencyGraphError, DependencyNode,
};
use crate::find_buildpack_dirs;
use crate::package_descriptor::buildpack_id_from_libcnb_dependency;
use libcnb_common::toml_file::{read_toml_file, TomlFileError};
use libcnb_data::buildpack::{BuildpackDescriptor, BuildpackId, BuildpackIdError};
use libcnb_data::package_descriptor::PackageDescriptor;
use petgraph::Graph;
use std::convert::Infallible;
use std::path::{Path, PathBuf};

/// Creates a dependency graph of libcnb.rs and composite buildpacks in a directory.
///
/// Buildpacks that aren't implemented with libcnb.rs or aren't composite buildpacks will not be part
/// of the dependency graph. Examples buildpacks that are not included are docker image URLs or
/// directories containing CNBs written in bash.
///
/// Likewise, the only dependency edges in the resulting graph are dependencies declared via
/// `libcnb:` URIs.
///
/// # Errors
///
/// Returns `Err` if a buildpack declares an invalid dependency, has an invalid buildpack.toml or
/// package.toml or an I/O error occurred while traversing the given directory.
pub fn build_libcnb_buildpacks_dependency_graph(
    cargo_workspace_root: &Path,
) -> Result<Graph<BuildpackDependencyGraphNode, ()>, BuildBuildpackDependencyGraphError> {
    find_buildpack_dirs(cargo_workspace_root)
        .map_err(BuildBuildpackDependencyGraphError::FindBuildpackDirectories)
        .and_then(|buildpack_directories| {
            buildpack_directories
                .iter()
                .filter(|buildpack_directory| {
                    matches!(
                        determine_buildpack_kind(buildpack_directory),
                        Some(BuildpackKind::LibCnbRs | BuildpackKind::Composite)
                    )
                })
                .map(|buildpack_directory| {
                    build_libcnb_buildpack_dependency_graph_node(buildpack_directory)
                })
                .collect::<Result<Vec<_>, _>>()
        })
        .and_then(|nodes| {
            create_dependency_graph(nodes)
                .map_err(BuildBuildpackDependencyGraphError::CreateDependencyGraphError)
        })
}

fn build_libcnb_buildpack_dependency_graph_node(
    buildpack_directory: &Path,
) -> Result<BuildpackDependencyGraphNode, BuildBuildpackDependencyGraphError> {
    let buildpack_id =
        read_toml_file::<BuildpackDescriptor>(buildpack_directory.join("buildpack.toml"))
            .map_err(BuildBuildpackDependencyGraphError::ReadBuildpackDescriptorError)
            .map(|buildpack_descriptor| buildpack_descriptor.buildpack().id.clone())?;

    let dependencies = {
        let package_toml_path = buildpack_directory.join("package.toml");

        package_toml_path
            .is_file()
            .then(|| {
                read_toml_file::<PackageDescriptor>(package_toml_path)
                    .map_err(BuildBuildpackDependencyGraphError::ReadPackageDescriptorError)
                    .and_then(|package_descriptor| {
                        get_buildpack_dependencies(&package_descriptor).map_err(
                            BuildBuildpackDependencyGraphError::InvalidDependencyBuildpackId,
                        )
                    })
            })
            .unwrap_or(Ok(Vec::new()))
    }?;

    Ok(BuildpackDependencyGraphNode {
        buildpack_id,
        path: PathBuf::from(buildpack_directory),
        dependencies,
    })
}

#[derive(thiserror::Error, Debug)]
pub enum BuildBuildpackDependencyGraphError {
    #[error("Error while finding buildpack directories: {0}")]
    FindBuildpackDirectories(ignore::Error),
    #[error("Couldn't read buildpack.toml: {0}")]
    ReadBuildpackDescriptorError(TomlFileError),
    #[error("Couldn't read package.toml: {0}")]
    ReadPackageDescriptorError(TomlFileError),
    #[error("Dependency uses an invalid buildpack id: {0}")]
    InvalidDependencyBuildpackId(BuildpackIdError),
    #[error("Error while creating dependency graph: {0}")]
    CreateDependencyGraphError(CreateDependencyGraphError<BuildpackId, Infallible>),
}

#[derive(Debug)]
pub struct BuildpackDependencyGraphNode {
    pub buildpack_id: BuildpackId,
    pub path: PathBuf,
    pub dependencies: Vec<BuildpackId>,
}

impl DependencyNode<BuildpackId, Infallible> for BuildpackDependencyGraphNode {
    fn id(&self) -> BuildpackId {
        self.buildpack_id.clone()
    }

    fn dependencies(&self) -> Result<Vec<BuildpackId>, Infallible> {
        Ok(self.dependencies.clone())
    }
}

fn get_buildpack_dependencies(
    package_descriptor: &PackageDescriptor,
) -> Result<Vec<BuildpackId>, BuildpackIdError> {
    package_descriptor
        .dependencies
        .iter()
        .filter_map(|dependency| buildpack_id_from_libcnb_dependency(dependency).transpose())
        .collect()
}
