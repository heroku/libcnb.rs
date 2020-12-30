use std::{
    collections::HashMap,
    env::VarError,
    ffi::{OsStr, OsString},
    fs, io,
    path::Path,
};

/// Represents a Cloud Native Buildpack platform.
///
/// Most buildpacks target a generic platform and this library provides a [`GenericPlatform`] for that
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
    ///use libcnb::platform::Platform;
    ///use libcnb::platform::GenericPlatform;
    ///let platform = GenericPlatform::from_path("/platform").unwrap();
    /// ```
    fn from_path(platform_dir: impl AsRef<Path>) -> io::Result<Self>;
}

/// A generic platform that only provides access to environment variables.
pub struct GenericPlatform {
    env: PlatformEnv,
}

impl Platform for GenericPlatform {
    fn env(&self) -> &PlatformEnv {
        &self.env
    }

    fn from_path(platform_dir: impl AsRef<Path>) -> io::Result<Self> {
        Ok(GenericPlatform {
            env: PlatformEnv::from_path(platform_dir)?,
        })
    }
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
    ///use libcnb::platform::PlatformEnv;
    ///let env = PlatformEnv::from_path("/platform").unwrap();
    ///let value = env.var("SOME_ENV_VAR");
    /// ```
    pub fn var<K: AsRef<OsStr>>(&self, key: K) -> Result<String, VarError> {
        self.vars
            .get(key.as_ref())
            .map(|s| s.to_owned())
            .ok_or(VarError::NotPresent)
    }

    /// Initializes a new PlatformEnv from the given platform directory.
    ///
    /// Buildpack authors usually do not need to create their own [`PlatformEnv`] and instead use the
    /// one passed via context structs ([`DetectContext`](crate::detect::DetectContext) and [`BuildContext`](crate::build::BuildContext)).
    ///
    /// # Examples
    /// ```no_run
    ///use libcnb::platform::PlatformEnv;
    ///let platform = PlatformEnv::from_path("/platform").unwrap();
    /// ```
    pub fn from_path(platform_dir: impl AsRef<Path>) -> Result<Self, io::Error> {
        let env_path = platform_dir.as_ref().join("env");
        let mut env_vars: HashMap<OsString, String> = HashMap::new();

        for entry in fs::read_dir(env_path)? {
            let entry = entry?;
            let path = entry.path();

            if let Some(file_name) = path.file_name() {
                let file_contents = fs::read_to_string(&path)?;
                env_vars.insert(file_name.to_owned(), file_contents);
            }
        }

        Ok(PlatformEnv { vars: env_vars })
    }
}
