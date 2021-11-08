use std::{
    collections::HashMap,
    env::VarError,
    ffi::{OsStr, OsString},
    fs, io,
    path::Path,
};

/// Represents a Cloud Native Buildpack platform.
///
/// Most buildpacks target a generic platform and this library provides a [`crate::generic::GenericPlatform`] for that
/// use-case. Buildpack authors usually do not need to implement this trait. See
/// [detection](https://github.com/buildpacks/spec/blob/main/buildpack.md#detection) and
/// [build](https://github.com/buildpacks/spec/blob/main/buildpack.md#build) in the buildpack
/// specification for details.
pub trait Platform
where
    Self: Sized,
{
    /// Retrieve a [`PlatformEnv`] reference for convenient access to environment variables which
    /// all platforms have to provide.
    fn env(&self) -> &PlatformEnv;

    /// Initializes the platform from the given platform directory.
    ///
    /// # Examples
    /// ```no_run
    ///use libcnb::Platform;
    ///use libcnb::GenericPlatform;
    ///let platform = GenericPlatform::from_path("/platform").unwrap();
    /// ```
    fn from_path(platform_dir: impl AsRef<Path>) -> io::Result<Self>;
}

/// Provides access to platform environment variables.
pub struct PlatformEnv {
    vars: HashMap<OsString, String>,
}

impl PlatformEnv {
    /// Fetches the environment variable `key` from the platform.
    ///
    /// # Examples
    /// ```no_run
    ///use libcnb::PlatformEnv;
    ///let env = PlatformEnv::from_path("/platform").unwrap();
    ///let value = env.var("SOME_ENV_VAR");
    /// ```
    pub fn var<K: AsRef<OsStr>>(&self, key: K) -> Result<String, VarError> {
        self.vars
            .get(key.as_ref())
            .map(|s| s.clone())
            .ok_or(VarError::NotPresent)
    }

    /// Initializes a new PlatformEnv from the given platform directory.
    ///
    /// Buildpack authors usually do not need to create their own [`PlatformEnv`] and instead use the
    /// one passed via context structs ([`DetectContext`](crate::detect::DetectContext) and [`BuildContext`](crate::build::BuildContext)).
    ///
    /// # Examples
    /// ```no_run
    ///use libcnb::PlatformEnv;
    ///let platform = PlatformEnv::from_path("/platform").unwrap();
    /// ```
    pub fn from_path(platform_dir: impl AsRef<Path>) -> Result<Self, io::Error> {
        let env_path = platform_dir.as_ref().join("env");
        let mut env_vars: HashMap<OsString, String> = HashMap::new();

        match fs::read_dir(env_path) {
            Ok(entries) => {
                for entry in entries {
                    let entry = entry?;
                    let path = entry.path();

                    if let Some(file_name) = path.file_name() {
                        // k8s volume mounts will mount a directory symlink in, so we need to check
                        // that it's actually a file
                        if path.is_file() {
                            let file_contents = fs::read_to_string(&path)?;
                            env_vars.insert(file_name.to_owned(), file_contents);
                        }
                    }
                }
            }
            Err(err) => {
                // don't fail if `<platform>/env` doesn't exist
                if err.kind() != std::io::ErrorKind::NotFound {
                    return Err(err);
                }
            }
        }

        Ok(Self { vars: env_vars })
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::fs;

    #[test]
    fn from_path_handles_directories_in_env_folder() {
        let tmpdir = tempfile::tempdir().unwrap();
        let env_dir = tmpdir.path().join("env");
        fs::create_dir(&env_dir).unwrap();
        let dummy_dir = env_dir.join("foobar");
        fs::create_dir(&dummy_dir).unwrap();
        fs::write(env_dir.join("FOO"), "BAR").unwrap();

        let result = PlatformEnv::from_path(tmpdir.path());
        assert!(result.is_ok());
    }

    // this symlink is only supported on unix
    #[cfg(target_family = "unix")]
    #[test]
    fn from_path_handles_directories_via_symlinks() {
        let tmpdir = tempfile::tempdir().unwrap();
        let env_dir = tmpdir.path().join("env");
        fs::create_dir(&env_dir).unwrap();
        let dummy_dir = env_dir.join("foobar");
        fs::create_dir(&dummy_dir).unwrap();
        let dst_symlink = env_dir.join("data");
        std::os::unix::fs::symlink(&dummy_dir, &dst_symlink).unwrap();
        fs::write(env_dir.join("FOO"), "BAR").unwrap();

        let result = PlatformEnv::from_path(tmpdir.path());
        assert!(result.is_ok());
    }

    #[test]
    fn from_path_doesnt_blow_up_if_platform_env_is_missing() {
        let tmpdir = tempfile::tempdir().unwrap();

        let result = PlatformEnv::from_path(tmpdir.path());
        assert!(result.is_ok());
    }
}
