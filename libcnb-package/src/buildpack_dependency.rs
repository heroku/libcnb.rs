use crate::output::BuildpackOutputDirectoryLocator;
use libcnb_data::buildpack::{BuildpackId, BuildpackIdError};
use libcnb_data::buildpackage::{Buildpackage, BuildpackageDependency};
use std::path::{Path, PathBuf};

/// Buildpack dependency type
#[derive(Debug, Clone, Eq, PartialEq)]
pub enum BuildpackDependency {
    External(BuildpackageDependency),
    Local(BuildpackId, BuildpackageDependency),
}

impl BuildpackDependency {
    #[must_use]
    pub fn get_local_buildpack_id(&self) -> Option<BuildpackId> {
        match self {
            BuildpackDependency::External(_) => None,
            BuildpackDependency::Local(id, _) => Some(id.clone()),
        }
    }
}

fn get_buildpack_dependencies(
    buildpackage: &Buildpackage,
) -> Result<Vec<BuildpackDependency>, BuildpackIdError> {
    buildpackage
        .dependencies
        .iter()
        .map(|dependency| {
            buildpack_id_from_libcnb_dependency(dependency).map(|buildpack_id| {
                buildpack_id.map_or_else(
                    || BuildpackDependency::External(dependency.clone()),
                    |value| BuildpackDependency::Local(value, dependency.clone()),
                )
            })
        })
        .collect::<Result<_, _>>()
}

fn buildpack_id_from_libcnb_dependency(
    dependency: &BuildpackageDependency,
) -> Result<Option<BuildpackId>, BuildpackIdError> {
    Some(&dependency.uri)
        .filter(|uri| {
            uri.scheme()
                .map_or(false, |scheme| scheme.as_str() == "libcnb")
        })
        .map(|uri| uri.path().to_string().parse())
        .transpose()
}

/// Reads the dependency URIs from the given `buildpackage` and returns any local libcnb project
/// references which should have the format `libcnb:{buildpack_id}`.
///
/// # Errors
///
/// Will return an `Err` if any of the local dependencies use an invalid [`BuildpackId`].
pub fn get_local_buildpackage_dependencies(
    buildpackage: &Buildpackage,
) -> Result<Vec<BuildpackId>, BuildpackIdError> {
    get_buildpack_dependencies(buildpackage).map(|dependencies| {
        dependencies
            .iter()
            .filter_map(BuildpackDependency::get_local_buildpack_id)
            .collect::<Vec<_>>()
    })
}

/// Creates a new [`Buildpackage`] value by replacing each local libcnb dependency with the
/// file path where the compiled dependency is located.
///
/// This assumes that each libcnb dependency has already been compiled and the given
/// `buildpack_ids_to_target_dir` contains the correct mappings of path locations for each
/// [`BuildpackId`].
///
/// # Errors
///
/// Will return an `Err` if:
/// * the given `buildpackage` contains a local dependency with an invalid [`BuildpackId`]
/// * there is no entry found in `buildpack_ids_to_target_dir` for a local dependency's [`BuildpackId`]
/// * the target path for a local dependency is an invalid URI
pub fn rewrite_buildpackage_local_dependencies(
    buildpackage: &Buildpackage,
    buildpack_output_directory_locator: &BuildpackOutputDirectoryLocator,
) -> Result<Buildpackage, RewriteBuildpackageLocalDependenciesError> {
    let local_dependency_to_target_dir = |target_dir: &PathBuf| {
        BuildpackageDependency::try_from(target_dir.clone()).map_err(|_| {
            RewriteBuildpackageLocalDependenciesError::InvalidDependency(target_dir.clone())
        })
    };

    get_buildpack_dependencies(buildpackage)
        .map_err(RewriteBuildpackageLocalDependenciesError::GetBuildpackDependenciesError)
        .and_then(|dependencies| {
            dependencies
                .into_iter()
                .map(|dependency| match dependency {
                    BuildpackDependency::External(buildpackage_dependency) => {
                        Ok(buildpackage_dependency)
                    }
                    BuildpackDependency::Local(buildpack_id, _) => {
                        let output_dir = buildpack_output_directory_locator.get(&buildpack_id);
                        local_dependency_to_target_dir(&output_dir)
                    }
                })
                .collect()
        })
        .map(|dependencies| Buildpackage {
            dependencies,
            buildpack: buildpackage.buildpack.clone(),
            platform: buildpackage.platform.clone(),
        })
}

/// An error for [`rewrite_buildpackage_local_dependencies`]
#[derive(Debug)]
pub enum RewriteBuildpackageLocalDependenciesError {
    TargetDirectoryLookup(BuildpackId),
    InvalidDependency(PathBuf),
    GetBuildpackDependenciesError(BuildpackIdError),
}

/// Creates a new [`Buildpackage`] value by replacing each relative URI with it's absolute path using
/// the given `source_path`.
///
/// # Errors
///
/// Will return an `Err` if:
/// * the given `buildpackage` contains a local dependency with an invalid [`BuildpackId`]
/// * the constructed absolute path is an invalid URI
pub fn rewrite_buildpackage_relative_path_dependencies_to_absolute(
    buildpackage: &Buildpackage,
    source_dir: &Path,
) -> Result<Buildpackage, RewriteBuildpackageRelativePathDependenciesToAbsoluteError> {
    let relative_dependency_to_absolute =
        |source_dir: &Path, buildpackage_dependency: BuildpackageDependency| {
            let absolute_path = source_dir.join(buildpackage_dependency.uri.path().to_string());
            BuildpackageDependency::try_from(absolute_path.clone()).map_err(|_| {
                RewriteBuildpackageRelativePathDependenciesToAbsoluteError::InvalidDependency(
                    absolute_path,
                )
            })
        };

    get_buildpack_dependencies(buildpackage)
        .map_err(RewriteBuildpackageRelativePathDependenciesToAbsoluteError::GetBuildpackDependenciesError)
        .and_then(|dependencies| {
            dependencies
                .into_iter()
                .map(|dependency| match dependency {
                    BuildpackDependency::External(buildpackage_dependency) => {
                        if buildpackage_dependency.uri.is_relative_path_reference() {
                            relative_dependency_to_absolute(source_dir, buildpackage_dependency)
                        } else {
                            Ok(buildpackage_dependency)
                        }
                    }
                    BuildpackDependency::Local(_, buildpackage_dependency) => {
                        Ok(buildpackage_dependency)
                    }
                })
                .collect()
        })
        .map(|dependencies| Buildpackage {
            dependencies,
            buildpack: buildpackage.buildpack.clone(),
            platform: buildpackage.platform.clone(),
        })
}

/// An error for [`rewrite_buildpackage_relative_path_dependencies_to_absolute`]
#[derive(Debug)]
pub enum RewriteBuildpackageRelativePathDependenciesToAbsoluteError {
    InvalidDependency(PathBuf),
    GetBuildpackDependenciesError(BuildpackIdError),
}

#[cfg(test)]
mod tests {
    use crate::buildpack_dependency::{
        get_local_buildpackage_dependencies, rewrite_buildpackage_local_dependencies,
        rewrite_buildpackage_relative_path_dependencies_to_absolute,
    };
    use crate::output::BuildpackOutputDirectoryLocator;
    use crate::CargoProfile;
    use libcnb_data::buildpack_id;
    use libcnb_data::buildpackage::{
        Buildpackage, BuildpackageBuildpackReference, BuildpackageDependency, Platform,
    };
    use std::path::PathBuf;

    #[test]
    fn test_rewrite_buildpackage_relative_path_dependencies() {
        let buildpackage = create_buildpackage();
        let source_dir = PathBuf::from("/test/source/path");
        let new_buildpackage =
            rewrite_buildpackage_relative_path_dependencies_to_absolute(&buildpackage, &source_dir)
                .unwrap();
        assert_eq!(
            new_buildpackage.dependencies[1].uri.to_string(),
            "/test/source/path/../relative/path"
        );
    }

    #[test]
    fn test_rewrite_buildpackage_local_dependencies() {
        let buildpackage = create_buildpackage();
        let buildpack_output_directory_locator = BuildpackOutputDirectoryLocator::new(
            PathBuf::from("/path/to/target"),
            CargoProfile::Dev,
            "arch".to_string(),
        );
        let new_buildpackage = rewrite_buildpackage_local_dependencies(
            &buildpackage,
            &buildpack_output_directory_locator,
        )
        .unwrap();
        assert_eq!(
            new_buildpackage.dependencies[0].uri.to_string(),
            "/path/to/target/buildpack/arch/debug/buildpack-id"
        );
    }

    #[test]
    fn test_get_local_buildpackage_dependencies() {
        let buildpackage = create_buildpackage();
        assert_eq!(
            get_local_buildpackage_dependencies(&buildpackage).unwrap(),
            vec![buildpack_id!("buildpack-id")]
        );
    }

    fn create_buildpackage() -> Buildpackage {
        create_buildpackage_with_dependencies(vec![
            "libcnb:buildpack-id",
            "../relative/path",
            "/absolute/path",
            "docker://docker.io/heroku/procfile-cnb:2.0.0",
        ])
    }

    fn create_buildpackage_with_dependencies<S>(dependencies: Vec<S>) -> Buildpackage
    where
        S: Into<String>,
    {
        Buildpackage {
            buildpack: BuildpackageBuildpackReference::try_from(".").unwrap(),
            dependencies: dependencies
                .into_iter()
                .map(|v| BuildpackageDependency::try_from(v.into().as_ref()).unwrap())
                .collect(),
            platform: Platform::default(),
        }
    }
}
