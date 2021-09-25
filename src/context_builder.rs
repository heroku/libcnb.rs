use serde::de::DeserializeOwned;
use std::fmt::{Debug, Display};
use std::path::{Path, PathBuf};
use tempfile::TempDir;

use crate::{
    data::{buildpack::BuildpackToml, buildpack_plan::BuildpackPlan, buildpack_plan::Entry},
    error::Error,
    platform::Platform,
    toml_file::TomlFileError,
    BuildContext, DetectContext,
};

/// Generates Detect and Build contexts
///
/// Set values on the builder. Generate a BuildContext<P, BM> with the `build()` function.
/// Generate a DetectContext<P, BM> with the `detect()` function.
///
/// The builder can be used as a oneliner to generate a single context:
///
///```
/// use tempfile::TempDir;
/// use libcnb::ContextBuilder;
/// use libcnb::{GenericPlatform, Platform};
///
/// pub fn detect(context: libcnb::GenericDetectContext) {}
///
/// let temp_dir = TempDir::new().unwrap();
/// let dir = temp_dir.path();
/// let context = ContextBuilder::new("heroku-20")
///     .buildpack_descriptor(include_str!("../examples/example-02-ruby-sample/buildpack.toml"))
///     .platform(GenericPlatform::from_path(dir.join("platform")).unwrap())
///     .buildpack_dir(dir.join("buildpack").to_path_buf())
///     .app_dir(dir.join("app").to_path_buf())
///     .detect::<std::io::Error>()
///     .unwrap();
///
/// detect(context);
///```
///
/// The builder can be reused to generate multiple contexts:
///
///```
/// use tempfile::TempDir;
/// use libcnb::ContextBuilder;
/// use libcnb::{GenericPlatform, Platform, data::buildpack_plan::BuildpackPlan, data::buildpack_plan::Entry};
///
/// pub fn detect(context: libcnb::GenericDetectContext) {};
/// pub fn build(context: libcnb::GenericBuildContext) {};

/// let temp_dir = TempDir::new().unwrap();
/// let dir = temp_dir.path();
/// let mut builder = ContextBuilder::new("heroku-20")
///     .buildpack_descriptor(include_str!("../examples/example-02-ruby-sample/buildpack.toml"))
///     .buildpack_plan(BuildpackPlan {entries: Vec::<Entry>::new()})
///     .buildpack_dir(dir.join("buildpack").to_path_buf())
///     .layers_dir(dir.join("layers").to_path_buf())
///     .platform(GenericPlatform::from_path(dir.join("platform")).unwrap())
///     .app_dir(dir.join("app").to_path_buf());
///
///  let detect_context = (&mut builder).detect::<std::io::Error>().unwrap();
///  let build_context = (&mut builder).build::<std::io::Error>().unwrap();
///
/// detect(detect_context);
/// build(build_context);
///```
///
/// In a test environment, default temporary directory values can be set
/// quickly using `with_tempdir()`:
///
///```
/// use libcnb::ContextBuilder;
///
/// pub fn detect(context: libcnb::GenericDetectContext) {}
///
/// let context = ContextBuilder::new("heroku-20")
///     .buildpack_descriptor(include_str!("../examples/example-02-ruby-sample/buildpack.toml"))
///     .with_tempdir()
///     .detect::<std::io::Error>()
///     .unwrap();
///
/// detect(context);
///```
///
/// WRONG INFO, pending wording updates.
///
/// I know this isn't exactly right and I need some help explaining the relationship
/// between the specific error we give to `build` and `libcnb::Error<E>`.
///
/// The generic error type returned by `detect()` and `build()` is constrained by `E: Clone + Copy`
/// where E is expected to `impl Into<libcnb::Error<E>>`.
///
/// Because E is unknown at build time, it must be resolved before it can
/// compile, that's why in these examples `build::<std::io::Error>()` is needed
/// instead of simply `build()`.
///
/// This is resolved by https://github.com/Malax/libcnb.rs/pull/97, but even so...I
/// want to understand the relationship above to explain it in a way that makes sense.
pub struct ContextBuilder<P: Platform + Clone, BM: DeserializeOwned + Clone> {
    app_dir: PathArg,
    layers_dir: PathArg,
    buildpack_dir: PathArg,
    stack_id: String,
    platform: PlatformArg<P>,
    buildpack_plan: BuildpackPlanArg,
    buildpack_descriptor: BuildpackDescriptorArg<BM>,
    _temp_dirs: Vec<TempDir>,
}
impl<P: Platform + Clone, BM: DeserializeOwned + Clone> ContextBuilder<P, BM> {
    pub fn new(stack_id: impl AsRef<str>) -> Self {
        ContextBuilder {
            app_dir: PathArg::None,
            layers_dir: PathArg::None,
            buildpack_dir: PathArg::None,
            stack_id: String::from(stack_id.as_ref()),
            platform: PlatformArg::None,
            buildpack_plan: BuildpackPlanArg::None,
            buildpack_descriptor: BuildpackDescriptorArg::None,
            _temp_dirs: vec![],
        }
    }

    pub fn platform(mut self, arg: impl Into<PlatformArg<P>>) -> ContextBuilder<P, BM> {
        self.platform = arg.into();
        self
    }

    pub fn buildpack_descriptor(
        mut self,
        arg: impl Into<BuildpackDescriptorArg<BM>>,
    ) -> ContextBuilder<P, BM> {
        self.buildpack_descriptor = arg.into();
        self
    }

    /// Used for testing purposes to set a minimal viable context
    ///
    /// When generating multiple contexts from the same builder directories
    /// are not shared.
    ///
    ///```
    /// use tempfile;
    /// use libcnb::ContextBuilder;
    ///
    /// pub fn detect(context: libcnb::GenericDetectContext) {};
    /// pub fn build(context: libcnb::GenericBuildContext) {};
    ///
    /// let mut builder = ContextBuilder::new("heroku-20")
    ///     .buildpack_descriptor(include_str!("../examples/example-02-ruby-sample/buildpack.toml"))
    ///     .with_tempdir();
    ///
    ///  let detect_context = (&mut builder).detect::<std::io::Error>().unwrap();
    ///  let build_context = (&mut builder).build::<std::io::Error>().unwrap();
    ///
    /// assert!(&build_context.app_dir != &detect_context.app_dir);
    ///
    /// detect(detect_context);
    /// build(build_context);
    ///```
    pub fn with_tempdir(mut self) -> ContextBuilder<P, BM> {
        if let PathArg::None = self.app_dir {
            self.app_dir = PathArg::Temporary;
        }
        if let PathArg::None = self.buildpack_dir {
            self.buildpack_dir = PathArg::Temporary;
        }
        if let PathArg::None = self.layers_dir {
            self.layers_dir = PathArg::Temporary
        }
        if let BuildpackPlanArg::None = self.buildpack_plan {
            self.buildpack_plan = BuildpackPlanArg::Temporary;
        }
        if let PlatformArg::None = self.platform {
            self.platform = PlatformArg::TemporaryDirectory;
        }
        self
    }

    pub fn layers_dir(mut self, dir: impl AsRef<Path>) -> ContextBuilder<P, BM> {
        self.layers_dir = PathArg::Static(dir.as_ref().to_path_buf());
        self
    }

    pub fn app_dir(mut self, dir: impl AsRef<Path>) -> ContextBuilder<P, BM> {
        self.app_dir = PathArg::Static(dir.as_ref().to_path_buf());
        self
    }

    pub fn buildpack_dir(mut self, dir: impl AsRef<Path>) -> ContextBuilder<P, BM> {
        self.buildpack_dir = PathArg::Static(dir.as_ref().to_path_buf());
        self
    }

    // [Internal]
    // Generates a temporary directory and retains it for the lifetime
    // of the builder.
    fn tempdir(&mut self) -> PathBuf {
        let temp_dir = TempDir::new().unwrap(); // Cheating with unwrap here, okay since it's only ever expected in test?
        let path = temp_dir.path().to_path_buf();
        self._temp_dirs.push(temp_dir);
        path
    }

    fn get_buildpack_descriptor<E: Display + Debug>(
        buildpack_descriptor: &BuildpackDescriptorArg<BM>,
    ) -> crate::Result<BuildpackToml<BM>, E> {
        match buildpack_descriptor.clone() {
            BuildpackDescriptorArg::String(contents) => toml::from_str(&contents)
                .map_err(TomlFileError::TomlDeserializationError)
                .map_err(Error::CannotReadBuildpackDescriptor),
            BuildpackDescriptorArg::File(path) => {
                crate::toml_file::read_toml_file(path).map_err(Error::CannotReadBuildpackDescriptor)
            }
            BuildpackDescriptorArg::BuildpackToml(a) => Ok(a),
            BuildpackDescriptorArg::None => Err(crate::error::Error::Other(
                "buildpack_descriptor is missing".to_string(),
            )),
        }
    }

    pub fn buildpack_plan(mut self, plan: BuildpackPlan) -> Self {
        self.buildpack_plan = BuildpackPlanArg::Plan(plan);
        self
    }

    // Generate a BuildContext
    pub fn build<E: Display + Debug>(&mut self) -> crate::Result<BuildContext<P, BM>, E> {
        // Re-use logic from building the DetectContext
        //
        // DetectContext and BuildContext share most of the same fields. Instead of extracting
        // the logic to build the fields into shared functions it was easier to build a
        // DetectContext and destructure its fields.
        let DetectContext {
            app_dir,
            buildpack_dir,
            platform,
            buildpack_descriptor,
            ..
        } = self.detect()?;

        let layers_dir = match self.layers_dir.clone() {
            PathArg::Static(p) => p,
            PathArg::Temporary => self.tempdir(),
            PathArg::None => Err(crate::error::Error::Other("missing layers_dir".to_string()))?,
        };

        let buildpack_plan = match self.buildpack_plan.clone() {
            BuildpackPlanArg::Plan(p) => p,
            BuildpackPlanArg::Temporary => BuildpackPlan {
                entries: Vec::<Entry>::new(),
            },
            BuildpackPlanArg::None => Err(crate::error::Error::Other(
                "buildpack_plan is missing".to_string(),
            ))?,
        };
        Ok(BuildContext {
            layers_dir,
            app_dir,
            buildpack_dir,
            stack_id: self.stack_id.clone(),
            platform,
            buildpack_plan,
            buildpack_descriptor,
        })
    }

    // Generate a DetectContext
    pub fn detect<E: Display + Debug>(&mut self) -> crate::Result<DetectContext<P, BM>, E> {
        let buildpack_descriptor = Self::get_buildpack_descriptor(&self.buildpack_descriptor)?;
        let platform = match (&self.platform).clone() {
            PlatformArg::Platform(p) => Ok(p),
            PlatformArg::Directory(dir) => {
                Platform::from_path(dir).map_err(Error::CannotCreatePlatformFromPath)
            }
            PlatformArg::TemporaryDirectory => {
                Platform::from_path(self.tempdir()).map_err(Error::CannotCreatePlatformFromPath)
            }
            PlatformArg::None => Err(crate::error::Error::Other(
                "Platform is missing".to_string(),
            )),
        }?;

        let app_dir = match self.app_dir.clone() {
            PathArg::Static(p) => p,
            PathArg::Temporary => self.tempdir(),
            PathArg::None => Err(crate::error::Error::Other("missing app_dir".to_string()))?,
        };

        let buildpack_dir = match self.buildpack_dir.clone() {
            PathArg::Static(p) => p,
            PathArg::Temporary => self.tempdir(),
            PathArg::None => Err(crate::error::Error::Other(
                "missing buildpack_dir".to_string(),
            ))?,
        };

        Ok(DetectContext {
            app_dir,
            buildpack_dir,
            stack_id: self.stack_id.clone(),
            platform,
            buildpack_descriptor,
        })
    }
}

#[derive(Clone)]
pub enum BuildpackDescriptorArg<BM: Clone> {
    File(PathBuf),
    String(String),
    BuildpackToml(BuildpackToml<BM>),
    None,
}

#[derive(Clone)]
pub enum PlatformArg<P: Platform> {
    Platform(P),
    Directory(PathBuf),
    TemporaryDirectory,
    None,
}

#[derive(Clone)]
enum PathArg {
    Static(PathBuf),
    Temporary,
    None,
}

#[derive(Clone)]
enum BuildpackPlanArg {
    Plan(BuildpackPlan),
    Temporary,
    None,
}

impl<BM: Clone> From<PathBuf> for BuildpackDescriptorArg<BM> {
    fn from(value: PathBuf) -> Self {
        BuildpackDescriptorArg::File(value)
    }
}

impl<BM: Clone> From<String> for BuildpackDescriptorArg<BM> {
    fn from(value: String) -> Self {
        BuildpackDescriptorArg::String(value)
    }
}

impl<BM: Clone> From<&str> for BuildpackDescriptorArg<BM> {
    fn from(value: &str) -> Self {
        BuildpackDescriptorArg::String(value.to_string())
    }
}

impl<BM: Clone> From<BuildpackToml<BM>> for BuildpackDescriptorArg<BM> {
    fn from(value: BuildpackToml<BM>) -> Self {
        BuildpackDescriptorArg::BuildpackToml(value)
    }
}

impl<P: Platform> From<PathBuf> for PlatformArg<P> {
    fn from(value: PathBuf) -> Self {
        PlatformArg::Directory(value)
    }
}

impl<P: Platform> From<P> for PlatformArg<P> {
    fn from(value: P) -> Self {
        PlatformArg::Platform(value)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::{GenericBuildContext, GenericDetectContext, GenericPlatform, Platform};

    pub fn fake_detect(_context: GenericDetectContext) {}
    pub fn fake_build(_context: GenericBuildContext) {}

    #[test]
    fn test_build() {
        let context = ContextBuilder::new("heroku-20")
            .with_tempdir()
            .buildpack_descriptor(include_str!(
                "../examples/example-02-ruby-sample/buildpack.toml"
            ))
            .build::<std::io::Error>()
            .unwrap();

        fake_build(context);
    }

    #[test]
    fn test_detect() {
        let context = ContextBuilder::new("heroku-20")
            .buildpack_descriptor(include_str!(
                "../examples/example-02-ruby-sample/buildpack.toml"
            ))
            .platform(GenericPlatform::from_path(".").unwrap())
            .app_dir(".")
            .buildpack_dir(".")
            .detect::<std::io::Error>()
            .unwrap();

        fake_detect(context);
    }
}
