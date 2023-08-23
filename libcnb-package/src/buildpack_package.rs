use crate::buildpack_dependency::get_local_package_descriptor_dependencies;
use crate::dependency_graph::DependencyNode;
use crate::{
    read_buildpack_descriptor, read_package_descriptor, GenericMetadata,
    ReadBuildpackDescriptorError, ReadPackageDescriptorError,
};
use libcnb_data::buildpack::{BuildpackDescriptor, BuildpackId, BuildpackIdError};
use libcnb_data::package_descriptor::PackageDescriptor;
use std::path::PathBuf;

/// A folder that can be packaged into a [Cloud Native Buildpack](https://buildpacks.io/)
#[derive(Debug)]
pub struct BuildpackPackage<T = GenericMetadata> {
    pub path: PathBuf,
    pub buildpack_descriptor: BuildpackDescriptor<T>,
    pub package_descriptor: Option<PackageDescriptor>,
}

impl BuildpackPackage {
    #[must_use]
    pub fn buildpack_id(&self) -> &BuildpackId {
        &self.buildpack_descriptor.buildpack().id
    }
}

impl DependencyNode<BuildpackId, BuildpackIdError> for BuildpackPackage {
    fn id(&self) -> BuildpackId {
        self.buildpack_descriptor.buildpack().id.clone()
    }

    fn dependencies(&self) -> Result<Vec<BuildpackId>, BuildpackIdError> {
        self.package_descriptor
            .as_ref()
            .map_or(Ok(vec![]), get_local_package_descriptor_dependencies)
    }
}

/// Reads both the buildpack and package descriptor data from a given project path.
///  
/// # Errors
///
/// Will return an `Err` if either the buildpack or package descriptor data could not be read.
pub fn read_buildpack_package<P: Into<PathBuf>>(
    project_path: P,
) -> Result<BuildpackPackage, ReadBuildpackPackageError> {
    let path = project_path.into();
    let buildpack_descriptor = read_buildpack_descriptor(&path)
        .map_err(ReadBuildpackPackageError::ReadBuildpackDescriptorError)?;

    let package_toml_path = path.join("package.toml");
    let package_descriptor = package_toml_path
        .is_file()
        .then(|| {
            read_package_descriptor(&package_toml_path)
                .map_err(ReadBuildpackPackageError::ReadPackageDescriptorError)
        })
        .transpose()?;

    Ok(BuildpackPackage {
        path,
        buildpack_descriptor,
        package_descriptor,
    })
}

/// An error from [`read_buildpack_package`]
#[derive(thiserror::Error, Debug)]
pub enum ReadBuildpackPackageError {
    #[error("Failed to read package descriptor data: {0}")]
    ReadBuildpackDescriptorError(#[source] ReadBuildpackDescriptorError),
    #[error("Failed to read package descriptor data: {0}")]
    ReadPackageDescriptorError(#[source] ReadPackageDescriptorError),
}
