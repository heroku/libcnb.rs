use crate::{
    read_buildpack_data, read_buildpackage_data, BuildpackData, BuildpackageData, GenericMetadata,
    ReadBuildpackDataError, ReadBuildpackageDataError,
};
use libcnb_data::buildpack::BuildpackId;
use std::path::PathBuf;

/// A folder that can be packaged into a [Cloud Native Buildpack](https://buildpacks.io/)
#[derive(Debug)]
pub struct BuildpackPackage<T = GenericMetadata> {
    pub path: PathBuf,
    pub buildpack_data: BuildpackData<T>,
    pub buildpackage_data: Option<BuildpackageData>,
}

impl BuildpackPackage {
    #[must_use]
    pub fn buildpack_id(&self) -> &BuildpackId {
        &self.buildpack_data.buildpack_descriptor.buildpack().id
    }
}

/// Reads both the buildpack and buildpackage data from a given project path.
///  
/// # Errors
///
/// Will return an `Err` if either the buildpack or buildpackage data could not be read.
pub fn read_buildpack_package<P: Into<PathBuf>>(
    project_path: P,
) -> Result<BuildpackPackage, ReadBuildpackPackageError> {
    let path = project_path.into();
    let buildpack_data =
        read_buildpack_data(&path).map_err(ReadBuildpackPackageError::ReadBuildpackDataError)?;
    let buildpackage_data = read_buildpackage_data(&path)
        .map_err(ReadBuildpackPackageError::ReadBuildpackageDataError)?;
    Ok(BuildpackPackage {
        path,
        buildpack_data,
        buildpackage_data,
    })
}

/// An error from [`read_buildpack_package`]
#[derive(Debug)]
pub enum ReadBuildpackPackageError {
    ReadBuildpackDataError(ReadBuildpackDataError),
    ReadBuildpackageDataError(ReadBuildpackageDataError),
}
