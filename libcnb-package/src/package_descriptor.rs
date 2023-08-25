use crate::util::absolutize_path;
use libcnb_data::buildpack::{BuildpackId, BuildpackIdError};
use libcnb_data::package_descriptor::{
    PackageDescriptor, PackageDescriptorDependency, PackageDescriptorDependencyError,
};
use std::collections::BTreeMap;

use std::path::{Path, PathBuf};

pub(crate) fn normalize_package_descriptor(
    descriptor: &PackageDescriptor,
    descriptor_path: &Path,
    buildpack_paths: &BTreeMap<BuildpackId, PathBuf>,
) -> Result<PackageDescriptor, NormalizePackageDescriptorError> {
    replace_libcnb_uris(descriptor, buildpack_paths)
        .map_err(NormalizePackageDescriptorError::ReplaceLibcnbUriError)
        .and_then(|package_descriptor| {
            absolutize_dependency_paths(&package_descriptor, descriptor_path)
                .map_err(NormalizePackageDescriptorError::PackageDescriptorDependencyError)
        })
}

#[derive(thiserror::Error, Debug)]
pub enum NormalizePackageDescriptorError {
    #[error("{0}")]
    ReplaceLibcnbUriError(#[source] ReplaceLibcnbUriError),
    #[error("{0}")]
    PackageDescriptorDependencyError(#[source] PackageDescriptorDependencyError),
}

fn replace_libcnb_uris(
    descriptor: &PackageDescriptor,
    buildpack_paths: &BTreeMap<BuildpackId, PathBuf>,
) -> Result<PackageDescriptor, ReplaceLibcnbUriError> {
    descriptor
        .dependencies
        .iter()
        .map(|dependency| replace_libcnb_uri(dependency, buildpack_paths))
        .collect::<Result<Vec<_>, _>>()
        .map(|dependencies| PackageDescriptor {
            dependencies,
            ..descriptor.clone()
        })
}

fn replace_libcnb_uri(
    dependency: &PackageDescriptorDependency,
    buildpack_paths: &BTreeMap<BuildpackId, PathBuf>,
) -> Result<PackageDescriptorDependency, ReplaceLibcnbUriError> {
    buildpack_id_from_libcnb_dependency(dependency)
        .map_err(ReplaceLibcnbUriError::BuildpackIdError)
        .and_then(|maybe_buildpack_id| {
            maybe_buildpack_id.map_or(Ok(dependency.clone()), |buildpack_id| {
                buildpack_paths
                    .get(&buildpack_id)
                    .ok_or(ReplaceLibcnbUriError::MissingBuildpackPath(buildpack_id))
                    .cloned()
                    .and_then(|buildpack_path| {
                        PackageDescriptorDependency::try_from(buildpack_path)
                            .map_err(ReplaceLibcnbUriError::PackageDescriptorDependencyError)
                    })
            })
        })
}

#[derive(thiserror::Error, Debug)]
pub enum ReplaceLibcnbUriError {
    #[error("Buildpack reference uses an invalid buildpack id: {0}")]
    BuildpackIdError(BuildpackIdError),
    #[error("Invalid package descriptor dependency: {0}")]
    PackageDescriptorDependencyError(PackageDescriptorDependencyError),
    #[error("Missing path for buildpack with id {0}")]
    MissingBuildpackPath(BuildpackId),
}

fn absolutize_dependency_paths(
    descriptor: &PackageDescriptor,
    descriptor_path: &Path,
) -> Result<PackageDescriptor, PackageDescriptorDependencyError> {
    let descriptor_parent_path = descriptor_path
        .parent()
        .map(PathBuf::from)
        .unwrap_or_default();

    descriptor
        .dependencies
        .iter()
        .map(|dependency| {
            let scheme = dependency
                .uri
                .scheme()
                .map(uriparse::scheme::Scheme::as_str);

            match scheme {
                None => PackageDescriptorDependency::try_from(absolutize_path(
                    &PathBuf::from(dependency.uri.path().to_string()),
                    &descriptor_parent_path,
                )),
                _ => Ok(dependency.clone()),
            }
        })
        .collect::<Result<Vec<_>, _>>()
        .map(|dependencies| PackageDescriptor {
            dependencies,
            ..descriptor.clone()
        })
}

pub(crate) fn buildpack_id_from_libcnb_dependency(
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
