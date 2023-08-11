use crate::CargoProfile;
use libcnb_data::buildpack::BuildpackId;
use std::path::PathBuf;

pub struct BuildpackOutputDirectoryLocator {
    root_dir: PathBuf,
    cargo_profile: CargoProfile,
    target_triple: String,
}

impl BuildpackOutputDirectoryLocator {
    #[must_use]
    pub fn new(root_dir: PathBuf, cargo_profile: CargoProfile, target_triple: String) -> Self {
        Self {
            root_dir,
            cargo_profile,
            target_triple,
        }
    }

    #[must_use]
    pub fn get(&self, buildpack_id: &BuildpackId) -> PathBuf {
        self.root_dir
            .join("buildpack")
            .join(&self.target_triple)
            .join(match self.cargo_profile {
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

#[cfg(test)]
mod tests {
    use crate::output::BuildpackOutputDirectoryLocator;
    use crate::CargoProfile;
    use libcnb_data::buildpack_id;
    use std::path::PathBuf;

    #[test]
    fn test_get_buildpack_output_directory_locator() {
        let buildpack_id = buildpack_id!("some-org/with-buildpack");

        assert_eq!(
            BuildpackOutputDirectoryLocator {
                cargo_profile: CargoProfile::Dev,
                target_triple: "x86_64-unknown-linux-musl".to_string(),
                root_dir: PathBuf::from("/target")
            }
            .get(&buildpack_id),
            PathBuf::from(
                "/target/buildpack/x86_64-unknown-linux-musl/debug/some-org_with-buildpack"
            )
        );
        assert_eq!(
            BuildpackOutputDirectoryLocator {
                cargo_profile: CargoProfile::Release,
                target_triple: "x86_64-unknown-linux-musl".to_string(),
                root_dir: PathBuf::from("/target")
            }
            .get(&buildpack_id),
            PathBuf::from(
                "/target/buildpack/x86_64-unknown-linux-musl/release/some-org_with-buildpack"
            )
        );
    }
}
