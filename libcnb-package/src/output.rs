use crate::CargoProfile;
use libcnb_data::buildpack::BuildpackId;
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
            .join(default_buildpack_directory_name(&buildpack_id))
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
