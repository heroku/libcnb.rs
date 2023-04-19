use crate::cli::PackageArgs;
use crate::logging::{log, warn};
use crate::package::locate_buildpacks::BuildpackDirectoryDependencyError::InvalidLocalDependencyUri;
use crate::package::locate_buildpacks::BuildpackDirectoryError::{
    FailedToReadBuildpack, FailedToReadBuildpackage, InvalidBuildpackDependencyUris,
};
use crate::package::PackageCommandError::{FailedToGlobWorkspaceDirectory, MissingParentDirectory};
use crate::package::PackageableBuildpackDependency::{External, Local};
use crate::package::{PackageCommandError, PackageableBuildpack, PackageableBuildpackDependency};
use cargo_metadata::MetadataCommand;
use glob::glob;
use indoc::formatdoc;
use libcnb_data::buildpack::BuildpackId;
use libcnb_data::buildpackage::BuildpackageDependency;
use libcnb_package::{
    default_buildpack_directory_name, read_buildpack_data, read_buildpackage_data,
    BuildpackDataError, BuildpackageData, BuildpackageDataError,
};
use std::env;
use std::path::{Path, PathBuf};
use std::process::Command;

pub(crate) fn locate_packageable_buildpacks(
    args: &PackageArgs,
) -> Result<Vec<PackageableBuildpack>, PackageCommandError> {
    log("🔍 Locating buildpacks...");
    let buildpack_workspace = get_buildpack_workspace()?;

    let (packageable_buildpacks, errors) = partition_result(
        find_buildpack_dirs(&buildpack_workspace.root)?
            .iter()
            .filter(|buildpack_dir| !buildpack_dir.starts_with(&buildpack_workspace.target_dir))
            .map(|buildpack_dir| {
                to_packageable_buildpack(buildpack_dir, &buildpack_workspace, args)
            }),
    );

    errors.iter().for_each(report_buildpack_project_warning);

    Ok(packageable_buildpacks)
}

fn find_buildpack_dirs(root: &Path) -> Result<Vec<PathBuf>, PackageCommandError> {
    glob(&root.join("**/buildpack.toml").to_string_lossy())
        .expect("Valid glob pattern")
        .map(|glob_result| {
            glob_result
                .map_err(FailedToGlobWorkspaceDirectory)
                .map(|path| {
                    path.parent()
                        .map(ToOwned::to_owned)
                        .ok_or(MissingParentDirectory)
                })?
        })
        .collect()
}

fn to_packageable_buildpack(
    buildpack_dir: &PathBuf,
    buildpack_workspace: &BuildpackWorkspace,
    args: &PackageArgs,
) -> Result<PackageableBuildpack, BuildpackDirectoryError> {
    let source_dir = buildpack_dir.clone();

    let buildpack_data = read_buildpack_data(buildpack_dir)
        .map_err(|error| FailedToReadBuildpack(buildpack_dir.clone(), error))?;

    let buildpack_id = &buildpack_data.buildpack_descriptor.buildpack().id;
    let target_dir = get_buildpack_target_dir(buildpack_id, buildpack_workspace, args);

    let mut buildpackage_data: Option<BuildpackageData> = None;
    if buildpack_dir.join("package.toml").exists() {
        buildpackage_data = read_buildpackage_data(buildpack_dir)
            .map_err(|error| FailedToReadBuildpackage(buildpack_dir.clone(), error))
            .map(Some)?;
    };

    let dependencies: Vec<_> = match &buildpackage_data {
        Some(buildpackage_data) => {
            get_buildpack_dependencies(buildpack_dir, buildpackage_data, buildpack_workspace, args)?
        }
        None => vec![],
    };

    Ok(PackageableBuildpack {
        source_dir,
        target_dir,
        buildpack_data,
        buildpackage_data,
        dependencies,
    })
}

fn get_buildpack_dependencies(
    buildpack_dir: &Path,
    buildpackage_dependencies: &BuildpackageData,
    buildpack_workspace: &BuildpackWorkspace,
    args: &PackageArgs,
) -> Result<Vec<PackageableBuildpackDependency>, BuildpackDirectoryError> {
    let (dependencies, errors): (Vec<_>, Vec<_>) = buildpackage_dependencies
        .buildpackage_descriptor
        .dependencies
        .iter()
        .map(|buildpackage_dependency| {
            to_buildpack_dependency(buildpackage_dependency, buildpack_workspace, args)
        })
        .partition(Result::is_ok);

    let errors: Vec<_> = errors.into_iter().filter_map(Result::err).collect();
    if errors.is_empty() {
        Ok(dependencies.into_iter().filter_map(Result::ok).collect())
    } else {
        Err(InvalidBuildpackDependencyUris(
            buildpack_dir.to_path_buf(),
            errors,
        ))
    }
}

fn to_buildpack_dependency(
    buildpackage_dependency: &BuildpackageDependency,
    buildpack_workspace: &BuildpackWorkspace,
    args: &PackageArgs,
) -> Result<PackageableBuildpackDependency, BuildpackDirectoryDependencyError> {
    match &buildpackage_dependency.uri.scheme() {
        Some(scheme) => {
            if scheme.as_str() == "libcnb" {
                buildpackage_dependency
                    .uri
                    .path()
                    .to_string()
                    .parse::<BuildpackId>()
                    .map(|buildpack_id| Local {
                        buildpack_id: buildpack_id.clone(),
                        target_dir: get_buildpack_target_dir(
                            &buildpack_id,
                            buildpack_workspace,
                            args,
                        ),
                    })
                    .map_err(|_| InvalidLocalDependencyUri(buildpackage_dependency.uri.to_string()))
            } else {
                Ok(External(buildpackage_dependency.clone()))
            }
        }
        None => Ok(External(buildpackage_dependency.clone())),
    }
}

fn get_buildpack_workspace() -> Result<BuildpackWorkspace, PackageCommandError> {
    let cargo = env::var("CARGO")
        .map(PathBuf::from)
        .ok()
        .unwrap_or_else(|| PathBuf::from("cargo"));

    let mut locate_project = Command::new(cargo);
    locate_project.args(["locate-project", "--workspace", "--message-format", "plain"]);

    let workspace_cargo_path = locate_project
        .output()
        .map(|output| {
            let stdout = String::from_utf8_lossy(&output.stdout);
            PathBuf::from(stdout.trim())
        })
        .map_err(PackageCommandError::CouldNotLocateCargoWorkspace)?;

    let workspace_root = match workspace_cargo_path.parent() {
        Some(workspace_root) => PathBuf::from(workspace_root),
        None => return Err(PackageCommandError::CouldNotGetCargoWorkspaceDirectory),
    };

    let workspace_cargo_metadata = MetadataCommand::new()
        .manifest_path(&workspace_root.join("Cargo.toml"))
        .exec()
        .map_err(PackageCommandError::CouldNotObtainCargoMetadata)?;

    let workspace_target_directory = workspace_cargo_metadata
        .target_directory
        .into_std_path_buf();

    Ok(BuildpackWorkspace {
        root: workspace_root,
        target_dir: workspace_target_directory,
    })
}

fn get_buildpack_target_dir(
    buildpack_id: &BuildpackId,
    buildpack_workspace: &BuildpackWorkspace,
    args: &PackageArgs,
) -> PathBuf {
    buildpack_workspace
        .target_dir
        .join("buildpack")
        .join(if args.release { "release" } else { "debug" })
        .join(default_buildpack_directory_name(buildpack_id))
}

fn partition_result<T, E, I>(results: I) -> (Vec<T>, Vec<E>)
where
    I: IntoIterator<Item = Result<T, E>>,
{
    let partition: (Vec<_>, Vec<_>) = results.into_iter().partition(Result::is_ok);
    (
        partition.0.into_iter().filter_map(Result::ok).collect(),
        partition.1.into_iter().filter_map(Result::err).collect(),
    )
}

fn report_buildpack_project_warning(error: &BuildpackDirectoryError) {
    let warning = match error {
        FailedToReadBuildpack(buildpack_dir, error) => {
            formatdoc! { "
                Ignoring buildpack project from {} due to failure reading `buildpack.toml`
    
                To include this project, please verify that the `buildpack.toml` file:
                • is readable
                • contains valid buildpack metadata
    
                Error: {:#?}",
                &buildpack_dir.to_string_lossy(), error
            }
        }

        FailedToReadBuildpackage(buildpack_dir, error) => {
            formatdoc! { "
                Ignoring buildpack project from {} due to failure reading `package.toml`
    
                To include this project, please verify that the `package.toml` file:
                • is readable
                • contains valid buildpackagage metadata
    
                Error: {:#?}",
                &buildpack_dir.to_string_lossy(), error
            }
        }

        InvalidBuildpackDependencyUris(buildpack_dir, errors) => {
            formatdoc! { "
                Ignoring buildpack project from {} due to invalid local dependency declarations
    
                To include this project, please fix the following URIs with invalid Buildpack Ids in the `package.toml` file:
                {}",
                &buildpack_dir.to_string_lossy(),
                errors.iter().map(|error| {
                    match error {
                        InvalidLocalDependencyUri(uri) => format!("• {uri}")
                    }
                }).collect::<Vec<String>>().join("\n")
            }
        }
    };

    warn(warning);
}

#[derive(Debug, Clone)]
struct BuildpackWorkspace {
    root: PathBuf,
    target_dir: PathBuf,
}

#[derive(Debug)]
enum BuildpackDirectoryError {
    FailedToReadBuildpack(PathBuf, BuildpackDataError),
    FailedToReadBuildpackage(PathBuf, BuildpackageDataError),
    InvalidBuildpackDependencyUris(PathBuf, Vec<BuildpackDirectoryDependencyError>),
}

#[derive(Debug)]
enum BuildpackDirectoryDependencyError {
    InvalidLocalDependencyUri(String),
}
