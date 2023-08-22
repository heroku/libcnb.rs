use libcnb_data::buildpack::{BuildpackId, BuildpackIdError};
use libcnb_package::build::{BuildBinariesError, BuildError};
use libcnb_package::buildpack_dependency::{
    RewriteBuildpackageLocalDependenciesError,
    RewriteBuildpackageRelativePathDependenciesToAbsoluteError,
};
use libcnb_package::dependency_graph::{CreateDependencyGraphError, GetDependenciesError};
use libcnb_package::FindCargoWorkspaceError;
use std::path::PathBuf;
use std::process::ExitStatus;

#[derive(Debug, thiserror::Error)]
pub(crate) enum Error {
    #[error("Failed to get current dir\nError: {0}")]
    GetCurrentDir(std::io::Error),

    #[error("Could not locate a Cargo workspace within `{0}` or it's parent directories")]
    GetWorkspaceDirectory(PathBuf),

    #[error("Could not read Cargo.toml metadata in `{path}`\nError: {source}")]
    ReadCargoMetadata {
        path: PathBuf,
        source: cargo_metadata::Error,
    },

    #[error("Could not create package directory: {0}\nError: {1}")]
    CreatePackageDirectory(PathBuf, std::io::Error),

    #[error("Could not determine a target directory for buildpack with id `{buildpack_id}`")]
    TargetDirectoryLookup { buildpack_id: BuildpackId },

    #[error("{message}")]
    CrossCompilationHelp { message: String },

    #[error("No environment variable named `CARGO` is set")]
    GetCargoBin(#[from] std::env::VarError),

    #[error("Meta-buildpack is missing expected package.toml file")]
    MissingBuildpackageData,

    #[error("Failed to serialize package.toml\nError: {0}")]
    SerializeBuildpackage(toml::ser::Error),

    #[error("Error while finding buildpack directories\nLocation: {0}\nError: {1}")]
    FindBuildpackDirs(PathBuf, std::io::Error),

    #[error("There was a problem with the build configuration")]
    BinaryConfig,

    #[error("I/O error while executing Cargo for target {target}\nError: {source}")]
    BinaryBuildExecution {
        target: String,
        source: std::io::Error,
    },

    #[error("Unexpected Cargo exit status for target {0}\nExit Status: {}\nExamine Cargo output for details and potential compilation errors.", .exit_code_or_unknown(.1))]
    BinaryBuildExitStatus(String, ExitStatus),

    #[error("Configured buildpack target name {target} could not be found!")]
    BinaryBuildMissingTarget { target: String },

    #[error("Failed to read buildpack data\nLocation: {path}\nError: {source}")]
    ReadBuildpackData {
        path: PathBuf,
        source: std::io::Error,
    },

    #[error("Failed to parse buildpack data\nLocation: {path}\nError: {source}")]
    ParseBuildpackData {
        path: PathBuf,
        source: toml::de::Error,
    },

    #[error("Failed to read buildpackage data\nLocation: {path}\nError: {source}")]
    ReadBuildpackageData {
        path: PathBuf,
        source: std::io::Error,
    },

    #[error("Failed to parse buildpackage data\nLocation: {path}\nError: {source}")]
    ParseBuildpackageData {
        path: PathBuf,
        source: toml::de::Error,
    },

    #[error("Failed to lookup buildpack dependency with id `{0}`")]
    BuildpackDependencyLookup(BuildpackId),

    #[error("Buildpack has an invalid id\nError: `{0}`")]
    BuildpackPackagesId(BuildpackIdError),

    #[error("Failed to locate buildpack with id `{0}`")]
    BuildpackPackagesLookup(BuildpackId),

    #[error("Failed to lookup target directory for dependency with id `{0}`")]
    RewriteLocalDependencyTargetDirectoryLookup(BuildpackId),

    #[error("Could not convert path into buildpackage file uri: {0}")]
    InvalidPathDependency(PathBuf),

    #[error("Unexpected error while getting buildpack dependencies\nError: {0}")]
    GetBuildpackDependencies(BuildpackIdError),

    #[error("No buildpacks found!")]
    NoBuildpacksFound,

    #[error("Could not assemble buildpack directory\nPath: {0}\nError: {1}")]
    AssembleBuildpackDirectory(PathBuf, std::io::Error),

    #[error(
        "Failed to write package.toml to the target buildpack directory\nPath: {0}\nError: {1}"
    )]
    WriteBuildpackage(PathBuf, std::io::Error),

    #[error("I/O error while creating target buildpack directory\nPath: {0}\nError: {1}")]
    CreateBuildpackTargetDirectory(PathBuf, std::io::Error),

    #[error(
        "Failed to write buildpack.toml to the target buildpack directory\nPath: {0}\nError: {1}"
    )]
    WriteBuildpack(PathBuf, std::io::Error),

    #[error("Could not remove existing buildpack target directory\nPath: {0}\nError: {1}")]
    CleanBuildpackTargetDirectory(PathBuf, std::io::Error),

    #[error("I/O error while calculating directory size\nPath: {0}\nError: {1}")]
    CalculateDirectorySize(PathBuf, std::io::Error),

    #[error("Failed to spawn Cargo command\nError: {0}")]
    SpawnCargoCommand(std::io::Error),

    #[error("Unexpected Cargo exit status while attempting to read workspace root\nExit Status: {}\nExamine Cargo output for details and potential compilation errors.", exit_code_or_unknown(.0))]
    CargoCommandFailure(ExitStatus),

    #[error("Could not read Cargo.toml metadata from workspace\nPath: {0}\nError: {1}")]
    GetBuildpackOutputDir(PathBuf, cargo_metadata::Error),
}

impl From<BuildBinariesError> for Error {
    fn from(value: BuildBinariesError) -> Self {
        match value {
            BuildBinariesError::ConfigError(_) => Error::BinaryConfig,

            BuildBinariesError::BuildError(target, BuildError::IoError(source)) => {
                Error::BinaryBuildExecution { target, source }
            }

            BuildBinariesError::BuildError(
                target,
                BuildError::UnexpectedCargoExitStatus(exit_status),
            ) => Error::BinaryBuildExitStatus(target, exit_status),

            BuildBinariesError::MissingBuildpackTarget(target) => {
                Error::BinaryBuildMissingTarget { target }
            }
        }
    }
}

impl From<libcnb_package::buildpack_package::ReadBuildpackPackageError> for Error {
    fn from(value: libcnb_package::buildpack_package::ReadBuildpackPackageError) -> Self {
        match value {
            libcnb_package::buildpack_package::ReadBuildpackPackageError::ReadBuildpackDataError(error) => match error
            {
                libcnb_package::ReadBuildpackDataError::ReadingBuildpack { path, source } => {
                    Error::ReadBuildpackData { path, source }
                }
                libcnb_package::ReadBuildpackDataError::ParsingBuildpack { path, source } => {
                    Error::ParseBuildpackData { path, source }
                }
            },
            libcnb_package::buildpack_package::ReadBuildpackPackageError::ReadBuildpackageDataError(error) => {
                match error {
                    libcnb_package::ReadBuildpackageDataError::ReadingBuildpackage {
                        path,
                        source,
                    } => Error::ReadBuildpackageData { path, source },
                    libcnb_package::ReadBuildpackageDataError::ParsingBuildpackage {
                        path,
                        source,
                    } => Error::ParseBuildpackageData { path, source },
                }
            }
        }
    }
}

impl From<GetDependenciesError<BuildpackId>> for Error {
    fn from(value: GetDependenciesError<BuildpackId>) -> Self {
        match value {
            GetDependenciesError::MissingDependency(buildpack_id) => {
                Error::BuildpackDependencyLookup(buildpack_id)
            }
        }
    }
}

impl From<CreateDependencyGraphError<BuildpackId, BuildpackIdError>> for Error {
    fn from(value: CreateDependencyGraphError<BuildpackId, BuildpackIdError>) -> Self {
        match value {
            CreateDependencyGraphError::Dependencies(error) => Error::BuildpackPackagesId(error),
            CreateDependencyGraphError::MissingDependency(id) => Error::BuildpackPackagesLookup(id),
        }
    }
}

impl From<RewriteBuildpackageLocalDependenciesError> for Error {
    fn from(value: RewriteBuildpackageLocalDependenciesError) -> Self {
        match value {
            RewriteBuildpackageLocalDependenciesError::TargetDirectoryLookup(id) => {
                Error::RewriteLocalDependencyTargetDirectoryLookup(id)
            }
            RewriteBuildpackageLocalDependenciesError::InvalidDependency(path) => {
                Error::InvalidPathDependency(path)
            }
            RewriteBuildpackageLocalDependenciesError::GetBuildpackDependenciesError(error) => {
                Error::GetBuildpackDependencies(error)
            }
        }
    }
}

impl From<RewriteBuildpackageRelativePathDependenciesToAbsoluteError> for Error {
    fn from(value: RewriteBuildpackageRelativePathDependenciesToAbsoluteError) -> Self {
        match value {
            RewriteBuildpackageRelativePathDependenciesToAbsoluteError::InvalidDependency(path) => Error::InvalidPathDependency(path),
            RewriteBuildpackageRelativePathDependenciesToAbsoluteError::GetBuildpackDependenciesError(error) => Error::GetBuildpackDependencies(error)
        }
    }
}

impl From<FindCargoWorkspaceError> for Error {
    fn from(value: FindCargoWorkspaceError) -> Self {
        match value {
            FindCargoWorkspaceError::GetCargoEnv(error) => Error::GetCargoBin(error),
            FindCargoWorkspaceError::SpawnCommand(error) => Error::SpawnCargoCommand(error),
            FindCargoWorkspaceError::CommandFailure(exit_status) => {
                Error::CargoCommandFailure(exit_status)
            }
            FindCargoWorkspaceError::GetParentDirectory(path) => Error::GetWorkspaceDirectory(path),
        }
    }
}

// This function is used with the thiserror crate, where getting a value from an (error) enum
// variant yields a reference. Since this is the only use-case for this function, we accept a
// reference, even though ExitStatus is Copy.
#[allow(clippy::trivially_copy_pass_by_ref)]
fn exit_code_or_unknown(exit_status: &ExitStatus) -> String {
    exit_status
        .code()
        .map_or_else(|| String::from("<unknown>"), |code| code.to_string())
}
