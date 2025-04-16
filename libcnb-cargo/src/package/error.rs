use libcnb_data::buildpack::BuildpackId;
use libcnb_package::buildpack_dependency_graph::BuildBuildpackDependencyGraphError;
use libcnb_package::dependency_graph::GetDependenciesError;
use libcnb_package::package::PackageBuildpackError;
use std::path::PathBuf;

#[derive(thiserror::Error, Debug)]
pub(crate) enum Error {
    #[error("Failed to get current dir: {0}")]
    CannotGetCurrentDir(#[source] std::io::Error),
    #[error("Failed to find Cargo workspace root: {0}")]
    CannotFindCargoWorkspaceRoot(#[source] libcnb_package::FindCargoWorkspaceRootError),
    #[error("Failed to create package directory {0}: {1}")]
    CannotCreatePackageDirectory(PathBuf, #[source] std::io::Error),
    #[error("Failed to create buildpack dependency graph: {0}")]
    CannotBuildBuildpackDependencyGraph(#[source] BuildBuildpackDependencyGraphError),
    #[error("Failed to get dependencies: {0}")]
    CannotGetDependencies(#[source] GetDependenciesError<BuildpackId>),
    #[error("Failed to create buildpack package directory {0}: {1}")]
    CannotCreateBuildpackDestinationDir(PathBuf, #[source] std::io::Error),
    #[error("Failed to package buildpack: {0}")]
    CannotPackageBuildpack(#[source] PackageBuildpackError),
    #[error("Failed to configure Cargo for cross-compilation")]
    CannotConfigureCrossCompilation,
    #[error("No buildpacks found!")]
    NoBuildpacksFound,
    #[error(
        "Could not determine a default target triple from the current architecture ({0}), you must explicitly provide the --target argument"
    )]
    CouldNotDetermineDefaultTargetForArch(String),
}
