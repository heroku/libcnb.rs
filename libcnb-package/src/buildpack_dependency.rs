use libcnb_data::buildpack::{BuildpackId, BuildpackIdError};
use libcnb_data::buildpackage::{PackageDescriptor, PackageDescriptorDependency};
use std::path::{Path, PathBuf};

/// Buildpack dependency type
#[derive(Debug, Clone, Eq, PartialEq)]
pub enum BuildpackDependency {
    External(PackageDescriptorDependency),
    Local(BuildpackId, PackageDescriptorDependency),
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
    package_descriptor: &PackageDescriptor,
) -> Result<Vec<BuildpackDependency>, BuildpackIdError> {
    package_descriptor
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
    dependency: &PackageDescriptorDependency,
) -> Result<Option<BuildpackId>, BuildpackIdError> {
    Some(&dependency.uri)
        .filter(|uri| {
            uri.scheme()
                .map_or(false, |scheme| scheme.as_str() == "libcnb")
        })
        .map(|uri| uri.path().to_string().parse())
        .transpose()
}

/// Reads the dependency URIs from the given [`PackageDescriptor`] and returns any local libcnb project
/// references which should have the format `libcnb:{buildpack_id}`.
///
/// # Errors
///
/// Will return an `Err` if any of the local dependencies use an invalid [`BuildpackId`].
pub fn get_local_package_descriptor_dependencies(
    package_descriptor: &PackageDescriptor,
) -> Result<Vec<BuildpackId>, BuildpackIdError> {
    get_buildpack_dependencies(package_descriptor).map(|dependencies| {
        dependencies
            .iter()
            .filter_map(BuildpackDependency::get_local_buildpack_id)
            .collect::<Vec<_>>()
    })
}

/// Creates a new [`PackageDescriptor`] value by replacing each local libcnb dependency with the
/// file path where the compiled dependency is located.
///
/// This assumes that each libcnb dependency has already been compiled and the given
/// `buildpack_ids_to_target_dir` contains the correct mappings of path locations for each
/// [`BuildpackId`].
///
/// # Errors
///
/// Will return an `Err` if:
/// * the given [`PackageDescriptor`] contains a local dependency with an invalid [`BuildpackId`]
/// * there is no entry found in `buildpack_ids_to_target_dir` for a local dependency's [`BuildpackId`]
/// * the target path for a local dependency is an invalid URI
pub fn rewrite_package_descriptor_local_dependencies(
    package_descriptor: &PackageDescriptor,
    packaged_buildpack_dir_resolver: &impl Fn(&BuildpackId) -> PathBuf,
) -> Result<PackageDescriptor, RewritePackageDescriptorLocalDependenciesError> {
    let local_dependency_to_target_dir = |target_dir: &PathBuf| {
        PackageDescriptorDependency::try_from(target_dir.clone()).map_err(|_| {
            RewritePackageDescriptorLocalDependenciesError::InvalidDependency(target_dir.clone())
        })
    };

    get_buildpack_dependencies(package_descriptor)
        .map_err(RewritePackageDescriptorLocalDependenciesError::InvalidBuildpackIdReference)
        .and_then(|dependencies| {
            dependencies
                .into_iter()
                .map(|dependency| match dependency {
                    BuildpackDependency::External(package_descriptor_dependency) => {
                        Ok(package_descriptor_dependency)
                    }
                    BuildpackDependency::Local(buildpack_id, _) => {
                        let output_dir = packaged_buildpack_dir_resolver(&buildpack_id);
                        local_dependency_to_target_dir(&output_dir)
                    }
                })
                .collect()
        })
        .map(|dependencies| PackageDescriptor {
            dependencies,
            buildpack: package_descriptor.buildpack.clone(),
            platform: package_descriptor.platform.clone(),
        })
}

/// An error for [`rewrite_package_descriptor_local_dependencies`]
#[derive(thiserror::Error, Debug)]
pub enum RewritePackageDescriptorLocalDependenciesError {
    #[error("Path {0} cannot be treated as a buildpack dependency")]
    InvalidDependency(PathBuf),
    #[error("Package descriptor references another buildpack with an invalid id: {0}")]
    InvalidBuildpackIdReference(#[source] BuildpackIdError),
}

/// Creates a new [`PackageDescriptor`] value by replacing each relative URI with it's absolute path using
/// the given `source_path`.
///
/// # Errors
///
/// Will return an `Err` if:
/// * the given [`PackageDescriptor`] contains a local dependency with an invalid [`BuildpackId`]
/// * the constructed absolute path is an invalid URI
pub fn rewrite_package_descriptor_relative_path_dependencies_to_absolute(
    package_descriptor: &PackageDescriptor,
    source_dir: &Path,
) -> Result<PackageDescriptor, RewritePackageDescriptorRelativePathDependenciesToAbsoluteError> {
    let relative_dependency_to_absolute =
        |source_dir: &Path, package_descriptor_dependency: PackageDescriptorDependency| {
            let absolute_path =
                source_dir.join(package_descriptor_dependency.uri.path().to_string());
            PackageDescriptorDependency::try_from(absolute_path.clone()).map_err(|_| {
                RewritePackageDescriptorRelativePathDependenciesToAbsoluteError::InvalidDependency(
                    absolute_path,
                )
            })
        };

    get_buildpack_dependencies(package_descriptor)
        .map_err(
            RewritePackageDescriptorRelativePathDependenciesToAbsoluteError::InvalidBuildpackIdReference,
        )
        .and_then(|dependencies| {
            dependencies
                .into_iter()
                .map(|dependency| match dependency {
                    BuildpackDependency::External(package_descriptor_dependency) => {
                        if package_descriptor_dependency.uri.is_relative_path_reference() {
                            relative_dependency_to_absolute(source_dir, package_descriptor_dependency)
                        } else {
                            Ok(package_descriptor_dependency)
                        }
                    }
                    BuildpackDependency::Local(_, package_descriptor_dependency) => {
                        Ok(package_descriptor_dependency)
                    }
                })
                .collect()
        })
        .map(|dependencies| PackageDescriptor {
            dependencies,
            buildpack: package_descriptor.buildpack.clone(),
            platform: package_descriptor.platform.clone(),
        })
}

/// An error for [`rewrite_package_descriptor_relative_path_dependencies_to_absolute`]
#[derive(thiserror::Error, Debug)]
pub enum RewritePackageDescriptorRelativePathDependenciesToAbsoluteError {
    #[error("Path {0} cannot be treated as a buildpack dependency")]
    InvalidDependency(PathBuf),
    #[error("Package descriptor references another buildpack with an invalid id: {0}")]
    InvalidBuildpackIdReference(#[source] BuildpackIdError),
}

#[cfg(test)]
mod tests {
    use crate::buildpack_dependency::{
        get_local_package_descriptor_dependencies, rewrite_package_descriptor_local_dependencies,
        rewrite_package_descriptor_relative_path_dependencies_to_absolute,
    };
    use crate::output::create_packaged_buildpack_dir_resolver;
    use crate::CargoProfile;
    use libcnb_data::buildpack_id;
    use libcnb_data::buildpackage::{
        PackageDescriptor, PackageDescriptorBuildpackReference, PackageDescriptorDependency,
        Platform,
    };
    use std::path::PathBuf;

    #[test]
    fn test_rewrite_package_descriptor_relative_path_dependencies() {
        let package_descriptor = create_package_descriptor();
        let source_dir = PathBuf::from("/test/source/path");
        let new_package_descriptor =
            rewrite_package_descriptor_relative_path_dependencies_to_absolute(
                &package_descriptor,
                &source_dir,
            )
            .unwrap();
        assert_eq!(
            new_package_descriptor.dependencies[1].uri.to_string(),
            "/test/source/path/../relative/path"
        );
    }

    #[test]
    fn test_rewrite_package_descriptor_local_dependencies() {
        let package_descriptor = create_package_descriptor();
        let packaged_buildpack_dir_resolver = create_packaged_buildpack_dir_resolver(
            &PathBuf::from("/path/to/target"),
            CargoProfile::Dev,
            "arch",
        );
        let new_package_descriptor = rewrite_package_descriptor_local_dependencies(
            &package_descriptor,
            &packaged_buildpack_dir_resolver,
        )
        .unwrap();
        assert_eq!(
            new_package_descriptor.dependencies[0].uri.to_string(),
            "/path/to/target/arch/debug/buildpack-id"
        );
    }

    #[test]
    fn test_get_local_package_descriptor_dependencies() {
        let package_descriptor = create_package_descriptor();
        assert_eq!(
            get_local_package_descriptor_dependencies(&package_descriptor).unwrap(),
            vec![buildpack_id!("buildpack-id")]
        );
    }

    fn create_package_descriptor() -> PackageDescriptor {
        create_package_descriptor_with_dependencies(vec![
            "libcnb:buildpack-id",
            "../relative/path",
            "/absolute/path",
            "docker://docker.io/heroku/example:1.2.3",
        ])
    }

    fn create_package_descriptor_with_dependencies<S>(dependencies: Vec<S>) -> PackageDescriptor
    where
        S: Into<String>,
    {
        PackageDescriptor {
            buildpack: PackageDescriptorBuildpackReference::try_from(".").unwrap(),
            dependencies: dependencies
                .into_iter()
                .map(|v| PackageDescriptorDependency::try_from(v.into().as_ref()).unwrap())
                .collect(),
            platform: Platform::default(),
        }
    }
}
