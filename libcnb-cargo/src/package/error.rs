use libcnb_data::buildpack::{BuildpackId, BuildpackIdError};
use libcnb_package::build::BuildBinariesError;
use std::path::PathBuf;

#[derive(thiserror::Error, Debug)]
pub(crate) enum Error {
    #[error("Failed to get current dir: {0}")]
    GetCurrentDir(#[source] std::io::Error),
    #[error("Failed to find Cargo workspace root: {0}")]
    FindCargoWorkspaceRoot(#[source] libcnb_package::FindCargoWorkspaceRootError),
    #[error("Failed to create package directory: {0}")]
    CreatePackageDirectory(PathBuf, #[source] std::io::Error),
    #[error("Failed to find buildpack directories in path {0}: {1}")]
    FindBuildpackDirs(PathBuf, #[source] std::io::Error),
    #[error("Failed to read buildpack package: {0}")]
    ReadBuildpackPackage(#[source] Box<libcnb_package::buildpack_package::ReadBuildpackPackageError>),
    #[error("Failed to create buildpack dependency graph: {0}")]
    CreateDependencyGraph(
        #[source] libcnb_package::dependency_graph::CreateDependencyGraphError<BuildpackId, BuildpackIdError>,
    ),
    #[error("No buildpacks found!")]
    NoBuildpacksFound,
    #[error("Failed to get dependencies: {0}")]
    GetDependencies(#[source] libcnb_package::dependency_graph::GetDependenciesError<BuildpackId>),
    #[error("Failed to read Cargo metadata: {1}")]
    ReadCargoMetadata(PathBuf, #[source] cargo_metadata::Error),
    #[error("Failed to assemble buildpack directory in {0}: {1}")]
    AssembleBuildpackDirectory(PathBuf, #[source] std::io::Error),
    #[error("Failed to build buildpack binaries: {0}")]
    BuildBinaries(#[source] BuildBinariesError),
    #[error("Failed to serialize package.toml: {0}")]
    SerializePackageDescriptor(#[source] toml::ser::Error),
    #[error("Failed to write package.toml to {0}: {1}")]
    WritePackageDescriptor(PathBuf, #[source] std::io::Error),
    #[error("Failed to create buildpack target directory {0}: {1}")]
    CreateBuildpackTargetDirectory(PathBuf, #[source] std::io::Error),
    #[error("Failed to copy buildpack.toml to {0}: {1}")]
    CopyBuildpackToml(PathBuf, #[source] std::io::Error),
    #[error("Buildpack does not contain a package.toml file")]
    MissingPackageDescriptorData,
    #[error("Failed to rewrite package.toml: {0}")]
    RewritePackageDescriptorLocalDependencies(#[source] libcnb_package::buildpack_dependency::RewritePackageDescriptorLocalDependenciesError),
    #[error("Failed to rewrite package.toml: {0}")]
    RewritePackageDescriptorRelativePathDependenciesToAbsolute(
        #[source] libcnb_package::buildpack_dependency::RewritePackageDescriptorRelativePathDependenciesToAbsoluteError,
    ),
    #[error("Failed to clean buildpack target directory {0}: {1}")]
    CleanBuildpackTargetDirectory(PathBuf, #[source] std::io::Error),
    #[error("Failed to calculate directory size of {0}: {1}")]
    CalculateDirectorySize(PathBuf, #[source] std::io::Error),
    #[error("{0}")]
    CrossCompilationHelp(String),
}
