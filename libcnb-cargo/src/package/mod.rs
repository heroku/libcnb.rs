use crate::cli::PackageArgs;
use crate::logging::fail_with_error;
use crate::package::locate_buildpacks::locate_packageable_buildpacks;
use crate::package::package_buildpacks::package_buildpacks;
use indoc::formatdoc;
use libcnb_data::buildpack::BuildpackId;
use libcnb_data::buildpackage::BuildpackageDependency;
use libcnb_package::build::{BuildBinariesError, BuildError};
use libcnb_package::{BuildpackData, BuildpackageData};
use std::path::PathBuf;
use toml::Table;

pub(crate) mod locate_buildpacks;
pub(crate) mod package_buildpacks;

pub(crate) fn run_package_command(args: &PackageArgs) {
    // first we locate all buildpacks and resolve their dependencies
    let buildpacks = match locate_packageable_buildpacks(args) {
        Ok(buildpacks) => buildpacks,
        Err(error) => on_package_command_error(error),
    };

    // then we compile the requested buildpacks
    // * either this is a single buildpack if you ran the command in a directory with a buildpack.toml
    // * or all the buildpacks if you ran the command from the root of a project
    let packaged_buildpacks = match package_buildpacks(buildpacks, args) {
        Ok(packaged_buildpacks) => packaged_buildpacks,
        Err(error) => on_package_command_error(error),
    };

    // print each packaged buildpack to stdout
    for packaged_buildpack in packaged_buildpacks {
        println!("{}", packaged_buildpack.to_string_lossy());
    }
}

fn on_package_command_error(error: PackageCommandError) -> ! {
    let message = match error {
        PackageCommandError::CouldNotObtainCargoMetadata(error) => {
            format!("Could not obtain metadata from Cargo: {error}")
        }

        PackageCommandError::CouldNotLocateCargoWorkspace(error) => {
            format!("Could not locate Cargo workspace: {error}")
        }

        PackageCommandError::CouldNotGetCargoWorkspaceDirectory => {
            "Could not get Cargo workspace directory".to_string()
        }

        PackageCommandError::CouldNotGetCurrentDir(error) => {
            format!("Could not get current directory: {error}")
        }

        PackageCommandError::UnresolvedLocalDependency(buildpack, id) => {
            formatdoc! { "
                Unresolved dependency encountered while trying to compile buildpack! 

                Check your package.toml file to make sure the reference to libcnb:{id} matches
                a buildpack in your workspace.
                
                Buildpack: {}

            ", buildpack.source_dir.to_string_lossy() }
        }

        PackageCommandError::CrossCompilationError(help_text) => {
            formatdoc! { "
                {help_text}
                
                To disable cross-compile assistance, pass --no-cross-compile-assistance.
            " }
        }

        PackageCommandError::FailedToCompileBuildpackError(error) => {
            let error_header = "Packaging buildpack failed due to a build related error!";

            match error {
                BuildBinariesError::ConfigError(_) => error_header.to_string(),

                BuildBinariesError::BuildError(target_name, BuildError::IoError(io_error)) => {
                    formatdoc! { "
                        {error_header}
                        
                        IO error while executing Cargo for target {target_name}: {io_error}
                    " }
                }

                BuildBinariesError::BuildError(
                    target_name,
                    BuildError::UnexpectedCargoExitStatus(exit_status),
                ) => {
                    formatdoc! { "
                            {error_header}
                            
                            Unexpected Cargo exit status for target {target_name}: {}
                            
                            Examine Cargo output for details and potential compilation errors.
                        ", 
                        exit_status
                            .code()
                            .map_or_else(|| String::from("<unknown>"), |code| code.to_string())
                    }
                }

                BuildBinariesError::MissingBuildpackTarget(target_name) => {
                    formatdoc! { "
                        {error_header}
                        
                        Configured buildpack target name {target_name} could not be found!
                    " }
                }
            }
        }

        PackageCommandError::FailedToCleanBuildpackTargetDirectory(error) => {
            format!("Failed to clean buildpack target directory: {error}")
        }

        PackageCommandError::CouldNotWriteToBuildpackTargetDirectory(error) => {
            format!("IO error while writing buildpack directory: {error}")
        }

        PackageCommandError::CouldNotCalculateCompiledBuildpackSize(error) => {
            format!("IO error while calculating buildpack directory size: {error}")
        }

        PackageCommandError::CouldNotCreateBuildpackTargetDir(error) => {
            format!("IO error while creating buildpack target directory: {error}")
        }

        PackageCommandError::CouldNotCopyBuildpackTomlToTargetDir(error) => {
            format!("IO error while copying buildpack.toml to target directory: {error}")
        }

        PackageCommandError::MetaBuildpackIsMissingBuildpackageData => {
            "Could not compile meta-buildpack - no package.toml data was present".to_string()
        }

        PackageCommandError::CouldNotSerializeBuildpackageData(error) => {
            format!("Could not serialize package.toml content: {error}")
        }
        PackageCommandError::CouldNotWriteBuildpackageData(error) => {
            format!("IO error while writing package.toml to target directory: {error}")
        }

        PackageCommandError::FailedToCreateLocalBuildPackageDependencies => {
            "Failed to update package.toml libcnb references to local paths".to_string()
        }

        PackageCommandError::NoBuildpacksToCompile => "No buildpacks to compile".to_string(),
    };
    fail_with_error(message);
}

#[derive(Debug, Clone)]
pub(crate) struct PackageableBuildpack {
    source_dir: PathBuf,
    target_dir: PathBuf,
    buildpack_data: BuildpackData<Option<Table>>,
    buildpackage_data: Option<BuildpackageData>,
    dependencies: Vec<PackageableBuildpackDependency>,
}

#[allow(clippy::large_enum_variant)]
#[derive(Debug, Clone)]
pub(crate) enum PackageableBuildpackDependency {
    Local {
        buildpack_id: BuildpackId,
        target_dir: PathBuf,
    },
    External(BuildpackageDependency),
}

pub(crate) enum PackageCommandError {
    CouldNotObtainCargoMetadata(cargo_metadata::Error),
    CouldNotLocateCargoWorkspace(std::io::Error),
    CouldNotGetCargoWorkspaceDirectory,
    CouldNotGetCurrentDir(std::io::Error),
    UnresolvedLocalDependency(PackageableBuildpack, BuildpackId),
    CrossCompilationError(String),
    FailedToCompileBuildpackError(BuildBinariesError),
    FailedToCleanBuildpackTargetDirectory(std::io::Error),
    CouldNotWriteToBuildpackTargetDirectory(std::io::Error),
    CouldNotCalculateCompiledBuildpackSize(std::io::Error),
    CouldNotCreateBuildpackTargetDir(std::io::Error),
    CouldNotCopyBuildpackTomlToTargetDir(std::io::Error),
    MetaBuildpackIsMissingBuildpackageData,
    CouldNotSerializeBuildpackageData(toml::ser::Error),
    CouldNotWriteBuildpackageData(std::io::Error),
    FailedToCreateLocalBuildPackageDependencies,
    NoBuildpacksToCompile,
}
