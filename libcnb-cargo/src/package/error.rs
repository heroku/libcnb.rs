use libcnb_data::buildpack::{BuildpackId, BuildpackIdError};
use libcnb_package::build::{BuildBinariesError, BuildError};
use libcnb_package::buildpack_dependency::{
    RewriteBuildpackageLocalDependenciesError,
    RewriteBuildpackageRelativePathDependenciesToAbsoluteError,
};
use libcnb_package::dependency_graph::{CreateDependencyGraphError, GetDependenciesError};
use libcnb_package::output::AssembleBuildpackDirectoryError;
use std::path::PathBuf;

#[derive(Debug, thiserror::Error)]
pub(crate) enum Error {
    #[error("Failed to get current dir\nError: {0}")]
    GetCurrentDir(std::io::Error),

    #[error("Could not locate a Cargo workspace within `{path}` or it's parent directories")]
    GetWorkspaceDirectory { path: PathBuf },

    #[error("Could not execute `cargo locate-project --workspace --message-format plain in {path}\nError: {source}")]
    GetWorkspaceCommand {
        path: PathBuf,
        source: std::io::Error,
    },

    #[error("Could not read Cargo.toml metadata in `{0}`\nError: {1}")]
    ReadCargoMetadata(PathBuf, cargo_metadata::Error),

    #[error("Could not create package directory: {0}\nError: {1}")]
    CreatePackageDirectory(PathBuf, std::io::Error),

    #[error("{0}")]
    CrossCompilationHelp(String),

    #[error("No environment variable named `CARGO` is set")]
    GetCargoBin(#[from] std::env::VarError),

    #[error("Failed to serialize package.toml\nError: {0}")]
    SerializeBuildpackage(toml::ser::Error),

    #[error("Error while finding buildpack directories\nLocation: {path}\nError: {source}")]
    FindBuildpackDirs {
        path: PathBuf,
        source: std::io::Error,
    },

    #[error("There was a problem with the build configuration")]
    BinaryConfig,

    #[error("I/O error while executing Cargo for target {target}\nError: {source}")]
    BinaryBuildExecution {
        target: String,
        source: std::io::Error,
    },

    #[error("Unexpected Cargo exit status for target {target}\nExit Status: {code}\nExamine Cargo output for details and potential compilation errors.")]
    BinaryBuildExitStatus { target: String, code: String },

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

    #[error("Could not convert path into buildpackage file uri: {0}")]
    InvalidPathDependency(PathBuf),

    #[error("Unexpected error while getting buildpack dependencies\nError: {0}")]
    GetBuildpackDependencies(BuildpackIdError),

    #[error("No buildpacks found!")]
    NoBuildpacksFound,

    #[error(
        "Failed to write package.toml to the target buildpack directory\nPath: {0}\nError: {1}"
    )]
    WriteBuildpackage(PathBuf, std::io::Error),

    #[error(
        "Failed to write buildpack.toml to the target buildpack directory\nPath: {0}\nError: {1}"
    )]
    WriteBuildpack(PathBuf, std::io::Error),

    #[error("Could not remove existing buildpack target directory\nPath: {0}\nError: {1}")]
    CleanBuildpackTargetDirectory(PathBuf, std::io::Error),

    #[error("I/O error while calculating directory size\nPath: {0}\nError: {1}")]
    CalculateDirectorySize(PathBuf, std::io::Error),

    #[error("Could not create buildpack directory\nPath: {0}\nError: {1}")]
    CreateBuildpackDestinationDirectory(PathBuf, std::io::Error),

    #[error("Could not create buildpack bin directory\nPath: {0}\nError: {1}")]
    CreateBinDirectory(PathBuf, std::io::Error),

    #[error("Could not write `build` binary to destination\nPath: {0}\nError: {1}")]
    WriteBuildBinary(PathBuf, std::io::Error),

    #[error("Could not write `detect` binary to destination\nPath: {0}\nError: {1}")]
    WriteDetectBinary(PathBuf, std::io::Error),

    #[error("Could not create buildpack additional binary directory\nPath: {0}\nError: {1}")]
    CreateAdditionalBinariesDirectory(PathBuf, std::io::Error),

    #[error("Could not write additional binary to destination\nPath: {0}\nError: {1}")]
    WriteAdditionalBinary(PathBuf, std::io::Error),
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
            ) => Error::BinaryBuildExitStatus {
                target,
                code: exit_status
                    .code()
                    .map_or_else(|| String::from("<unknown>"), |code| code.to_string()),
            },

            BuildBinariesError::MissingBuildpackTarget(target) => {
                Error::BinaryBuildMissingTarget { target }
            }

            BuildBinariesError::ReadCargoMetadata(path, error) => {
                Error::ReadCargoMetadata(path, error)
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

impl From<AssembleBuildpackDirectoryError> for Error {
    fn from(value: AssembleBuildpackDirectoryError) -> Self {
        match value {
            AssembleBuildpackDirectoryError::CreateBuildpackDestinationDirectory(path, error) => {
                Error::CreateBuildpackDestinationDirectory(path, error)
            }
            AssembleBuildpackDirectoryError::WriteBuildpack(path, error) => {
                Error::WriteBuildpack(path, error)
            }
            AssembleBuildpackDirectoryError::SerializeBuildpackage(error) => {
                Error::SerializeBuildpackage(error)
            }
            AssembleBuildpackDirectoryError::WriteBuildpackage(path, error) => {
                Error::WriteBuildpackage(path, error)
            }
            AssembleBuildpackDirectoryError::CreateBinDirectory(path, error) => {
                Error::CreateBinDirectory(path, error)
            }
            AssembleBuildpackDirectoryError::WriteBuildBinary(path, error) => {
                Error::WriteBuildBinary(path, error)
            }
            AssembleBuildpackDirectoryError::WriteDetectBinary(path, error) => {
                Error::WriteDetectBinary(path, error)
            }
            AssembleBuildpackDirectoryError::CreateAdditionalBinariesDirectory(path, error) => {
                Error::CreateAdditionalBinariesDirectory(path, error)
            }
            AssembleBuildpackDirectoryError::WriteAdditionalBinary(path, error) => {
                Error::WriteAdditionalBinary(path, error)
            }
            AssembleBuildpackDirectoryError::RewriteLocalDependencies(error) => error.into(),
            AssembleBuildpackDirectoryError::RewriteRelativePathDependencies(error) => error.into(),
        }
    }
}
