use crate::build::BuildpackBinaries;
use crate::buildpack_dependency::{
    rewrite_buildpackage_local_dependencies,
    rewrite_buildpackage_relative_path_dependencies_to_absolute,
    RewriteBuildpackageLocalDependenciesError,
    RewriteBuildpackageRelativePathDependenciesToAbsoluteError,
};
use crate::buildpack_package::BuildpackPackage;
use crate::CargoProfile;
use libcnb_data::buildpack::BuildpackId;
use libcnb_data::buildpackage::Buildpackage;
use std::fs;
use std::path::{Path, PathBuf};

/// Create a function that can construct the output location for a buildpack.
pub fn create_packaged_buildpack_dir_resolver(
    package_dir: &Path,
    cargo_profile: CargoProfile,
    target_triple: &str,
) -> impl Fn(&BuildpackId) -> PathBuf {
    let package_dir = PathBuf::from(package_dir);
    let target_triple = target_triple.to_string();

    move |buildpack_id| {
        package_dir
            .join(&target_triple)
            .join(match cargo_profile {
                CargoProfile::Dev => "debug",
                CargoProfile::Release => "release",
            })
            .join(default_buildpack_directory_name(buildpack_id))
    }
}

/// Construct a good default filename for a buildpack directory.
///
/// This function ensures the resulting name is valid and does not contain problematic characters
/// such as `/`.
#[must_use]
pub fn default_buildpack_directory_name(buildpack_id: &BuildpackId) -> String {
    buildpack_id.replace('/', "_")
}

#[derive(Debug)]
pub enum AssembleBuildpackDirectoryError {
    CreateBuildpackDestinationDirectory(PathBuf, std::io::Error),
    WriteBuildpack(PathBuf, std::io::Error),
    SerializeBuildpackage(toml::ser::Error),
    WriteBuildpackage(PathBuf, std::io::Error),
    CreateBinDirectory(PathBuf, std::io::Error),
    WriteBuildBinary(PathBuf, std::io::Error),
    WriteDetectBinary(PathBuf, std::io::Error),
    CreateAdditionalBinariesDirectory(PathBuf, std::io::Error),
    WriteAdditionalBinary(PathBuf, std::io::Error),
    RewriteLocalDependencies(RewriteBuildpackageLocalDependenciesError),
    RewriteRelativePathDependencies(RewriteBuildpackageRelativePathDependenciesToAbsoluteError),
}

/// Creates a buildpack directory and copies all buildpack assets to it.
///
/// Assembly of the directory follows the constraints set by the libcnb framework. For example,
/// the buildpack binary is only copied once and symlinks are used to refer to it when the CNB
/// spec requires different file(name)s.
///
/// This function will not validate if the buildpack descriptor at the given path is valid and will
/// use it as-is.
///
/// # Errors
///
/// Will return `Err` if the buildpack directory could not be assembled.
pub fn assemble_single_buildpack_directory(
    destination_path: impl AsRef<Path>,
    buildpack_package: &BuildpackPackage,
    buildpack_binaries: &BuildpackBinaries,
) -> Result<(), AssembleBuildpackDirectoryError> {
    fs::create_dir_all(destination_path.as_ref()).map_err(|e| {
        AssembleBuildpackDirectoryError::CreateBuildpackDestinationDirectory(
            destination_path.as_ref().to_path_buf(),
            e,
        )
    })?;

    let source_buildpack_path = &buildpack_package.buildpack_data.buildpack_descriptor_path;
    let destination_buildpack_path = destination_path.as_ref().join("buildpack.toml");
    fs::copy(source_buildpack_path, &destination_buildpack_path).map_err(|e| {
        AssembleBuildpackDirectoryError::WriteBuildpack(destination_buildpack_path, e)
    })?;

    let default_buildpackage = Buildpackage::default();
    let buildpackage = buildpack_package
        .buildpackage_data
        .as_ref()
        .map_or(&default_buildpackage, |data| &data.buildpackage_descriptor);
    let buildpackage_content = toml::to_string(buildpackage)
        .map_err(AssembleBuildpackDirectoryError::SerializeBuildpackage)?;
    let destination_buildpackage_path = destination_path.as_ref().join("package.toml");
    fs::write(&destination_buildpackage_path, buildpackage_content).map_err(|e| {
        AssembleBuildpackDirectoryError::WriteBuildpackage(destination_buildpackage_path, e)
    })?;

    let bin_path = destination_path.as_ref().join("bin");
    fs::create_dir_all(&bin_path)
        .map_err(|e| AssembleBuildpackDirectoryError::CreateBinDirectory(bin_path.clone(), e))?;

    fs::copy(
        &buildpack_binaries.buildpack_target_binary_path,
        bin_path.join("build"),
    )
    .map_err(|e| AssembleBuildpackDirectoryError::WriteBuildBinary(bin_path.join("build"), e))?;

    create_file_symlink("build", bin_path.join("detect")).map_err(|e| {
        AssembleBuildpackDirectoryError::WriteDetectBinary(bin_path.join("detect"), e)
    })?;

    if !buildpack_binaries.additional_target_binary_paths.is_empty() {
        let additional_binaries_dir = destination_path
            .as_ref()
            .join(".libcnb-cargo")
            .join("additional-bin");

        fs::create_dir_all(&additional_binaries_dir).map_err(|e| {
            AssembleBuildpackDirectoryError::CreateAdditionalBinariesDirectory(
                additional_binaries_dir.clone(),
                e,
            )
        })?;

        for (binary_target_name, binary_path) in &buildpack_binaries.additional_target_binary_paths
        {
            fs::copy(
                binary_path,
                additional_binaries_dir.join(binary_target_name),
            )
            .map_err(|e| {
                AssembleBuildpackDirectoryError::WriteAdditionalBinary(
                    additional_binaries_dir.join(binary_target_name),
                    e,
                )
            })?;
        }
    }

    Ok(())
}

/// Creates a meta-buildpack directory and copies all required meta-buildpack assets to it.
///
/// This function will not validate if the buildpack descriptor at the given path is valid and will
/// use it as-is.
///
/// It will also rewrite all package.toml references that use the `libcnb:{buildpack_id}` format as
/// well as relative file references to use absolute paths.
///
/// # Errors
///
/// Will return `Err` if the meta-buildpack directory could not be assembled.
pub fn assemble_meta_buildpack_directory(
    destination_path: impl AsRef<Path>,
    buildpack_package: &BuildpackPackage,
    packaged_buildpack_dir_resolver: &impl Fn(&BuildpackId) -> PathBuf,
) -> Result<(), AssembleBuildpackDirectoryError> {
    fs::create_dir_all(destination_path.as_ref()).map_err(|e| {
        AssembleBuildpackDirectoryError::CreateBuildpackDestinationDirectory(
            destination_path.as_ref().to_path_buf(),
            e,
        )
    })?;

    let source_buildpack_path = &buildpack_package.buildpack_data.buildpack_descriptor_path;
    let destination_buildpack_path = destination_path.as_ref().join("buildpack.toml");
    fs::copy(source_buildpack_path, &destination_buildpack_path).map_err(|e| {
        AssembleBuildpackDirectoryError::WriteBuildpack(destination_buildpack_path, e)
    })?;

    let buildpack_source_dir = &buildpack_package.path;
    let default_buildpackage = Buildpackage::default();

    let buildpackage = rewrite_buildpackage_local_dependencies(
        buildpack_package
            .buildpackage_data
            .as_ref()
            .map_or(&default_buildpackage, |data| &data.buildpackage_descriptor),
        packaged_buildpack_dir_resolver,
    )
    .map_err(AssembleBuildpackDirectoryError::RewriteLocalDependencies)
    .and_then(|buildpackage| {
        rewrite_buildpackage_relative_path_dependencies_to_absolute(
            &buildpackage,
            buildpack_source_dir,
        )
        .map_err(AssembleBuildpackDirectoryError::RewriteRelativePathDependencies)
    })?;
    let buildpackage_content = toml::to_string(&buildpackage)
        .map_err(AssembleBuildpackDirectoryError::SerializeBuildpackage)?;
    let destination_buildpackage_path = destination_path.as_ref().join("package.toml");
    fs::write(&destination_buildpackage_path, buildpackage_content).map_err(|e| {
        AssembleBuildpackDirectoryError::WriteBuildpackage(destination_buildpackage_path, e)
    })?;

    Ok(())
}

#[cfg(target_family = "unix")]
fn create_file_symlink<P: AsRef<Path>, Q: AsRef<Path>>(
    original: P,
    link: Q,
) -> std::io::Result<()> {
    std::os::unix::fs::symlink(original.as_ref(), link.as_ref())
}

#[cfg(target_family = "windows")]
fn create_file_symlink<P: AsRef<Path>, Q: AsRef<Path>>(
    original: P,
    link: Q,
) -> std::io::Result<()> {
    std::os::windows::fs::symlink_file(original.as_ref(), link.as_ref())
}

#[cfg(test)]
mod tests {
    use crate::output::create_packaged_buildpack_dir_resolver;
    use crate::CargoProfile;
    use libcnb_data::buildpack_id;
    use std::path::PathBuf;

    #[test]
    fn test_get_buildpack_target_dir() {
        let buildpack_id = buildpack_id!("some-org/with-buildpack");
        let package_dir = PathBuf::from("/package");
        let target_triple = "x86_64-unknown-linux-musl";

        let dev_packaged_buildpack_dir_resolver =
            create_packaged_buildpack_dir_resolver(&package_dir, CargoProfile::Dev, target_triple);

        let release_packaged_buildpack_dir_resolver = create_packaged_buildpack_dir_resolver(
            &package_dir,
            CargoProfile::Release,
            target_triple,
        );

        assert_eq!(
            dev_packaged_buildpack_dir_resolver(&buildpack_id),
            PathBuf::from("/package/x86_64-unknown-linux-musl/debug/some-org_with-buildpack")
        );
        assert_eq!(
            release_packaged_buildpack_dir_resolver(&buildpack_id),
            PathBuf::from("/package/x86_64-unknown-linux-musl/release/some-org_with-buildpack")
        );
    }
}
