use std::fmt::{Debug, Display};
use std::path::Path;
use std::path::PathBuf;

use crate::{
    data::build_plan::BuildPlan, data::buildpack::BuildpackToml, error::Error, platform::Platform,
};

use serde::de::DeserializeOwned;

/// Context for a buildpack's detect phase execution.
pub struct DetectContext<P: Platform, BM> {
    pub app_dir: PathBuf,
    pub buildpack_dir: PathBuf,
    pub stack_id: String,
    pub platform: P,
    pub buildpack_descriptor: BuildpackToml<BM>,
}

/// Describes the outcome of the buildpack's detect phase.
#[derive(Debug)]
pub enum DetectOutcome {
    Pass(BuildPlan),
    Fail,
}

pub struct DetectContextBuilder<P: Platform, BM: DeserializeOwned> {
    app_dir: Option<PathBuf>,
    buildpack_dir: Option<PathBuf>,
    stack_id: String,
    platform: Option<P>,
    platform_dir: Option<PathBuf>,
    buildpack_toml: Option<BuildpackToml<BM>>,
    buildpack_toml_path: Option<PathBuf>,
    buildpack_toml_string: Option<String>,
}
impl<P: Platform, BM: DeserializeOwned> DetectContextBuilder<P, BM> {
    pub fn new(stack_id: impl AsRef<str>) -> Self {
        DetectContextBuilder {
            app_dir: None,
            buildpack_dir: None,
            buildpack_toml_path: None,
            buildpack_toml_string: None,
            platform_dir: None,
            stack_id: String::from(stack_id.as_ref()),
            platform: None,
            buildpack_toml: None,
        }
    }

    /// Used for testing purposes to set a minimal viable context
    ///
    ///```
    /// use tempfile;
    /// use libcnb::DetectContextBuilder;
    ///
    /// pub fn detect(context: libcnb::GenericDetectContext) {}
    ///
    /// let temp_dir = tempfile::tempdir().unwrap();
    /// let context = DetectContextBuilder::new("heroku-20")
    ///     .buildpack_toml_from_string(include_str!("../examples/example-02-ruby-sample/buildpack.toml"))
    ///     .from_temp_dir(&temp_dir)
    ///     .build::<std::io::Error>()
    ///     .unwrap();
    ///
    /// detect(context);
    ///```
    pub fn from_temp_dir(mut self, temp_dir: &tempfile::TempDir) -> DetectContextBuilder<P, BM> {
        let app_dir = temp_dir.path().join("app");
        let platform_dir = temp_dir.path().join("platform");
        let buildpack_dir = temp_dir.path().join("buildpack");

        for dir in [&app_dir, &platform_dir, &buildpack_dir] {
            std::fs::create_dir_all(dir).unwrap();
        }

        if self.app_dir.is_none() {
            self.app_dir = Some(app_dir);
        }

        if self.buildpack_dir.is_none() {
            self.buildpack_dir = Some(buildpack_dir);
        }

        if self.platform.is_none() {
            self.platform_dir = Some(platform_dir);
        }
        self
    }

    pub fn buildpack_toml_from_string(
        mut self,
        body: impl AsRef<str>,
    ) -> DetectContextBuilder<P, BM> {
        self.buildpack_toml_string = Some(String::from(body.as_ref()));
        self
    }

    pub fn buildpack_toml_from_file(
        mut self,
        path: impl AsRef<Path>,
    ) -> DetectContextBuilder<P, BM> {
        self.buildpack_toml_path = Some(path.as_ref().to_path_buf());
        self
    }

    pub fn buildpack_toml(
        mut self,
        buildpack_toml: BuildpackToml<BM>,
    ) -> DetectContextBuilder<P, BM> {
        self.buildpack_toml = Some(buildpack_toml);
        self
    }

    pub fn platform(mut self, platform: P) -> DetectContextBuilder<P, BM> {
        self.platform = Some(platform);
        self
    }

    pub fn app_dir(mut self, app_dir: impl AsRef<Path>) -> DetectContextBuilder<P, BM> {
        self.app_dir = Some(app_dir.as_ref().to_path_buf());
        self
    }

    pub fn buildpack_dir(mut self, buildpack_dir: impl AsRef<Path>) -> DetectContextBuilder<P, BM> {
        self.buildpack_dir = Some(buildpack_dir.as_ref().to_path_buf());
        self
    }

    pub fn platform_from_path(
        mut self,
        platform_dir: impl AsRef<Path>,
    ) -> DetectContextBuilder<P, BM> {
        self.platform_dir = Some(platform_dir.as_ref().to_path_buf());
        self
    }

    pub fn build<E: Display + Debug>(self) -> crate::Result<DetectContext<P, BM>, E> {
        match vec![self.platform.is_some(), self.platform_dir.is_some()]
            .iter()
            .filter(|&&x| x)
            .count()
        {
            0 => {
                return Err(crate::error::Error::Other(
                    "Platform is missing, call platform() or platform_from_path()".to_string(),
                ))
            }
            1 => {}
            _ => {
                return Err(crate::error::Error::Other(
                    "Cannot determine platform, call ONLY one: platform() or platform_from_path()"
                        .to_string(),
                ))
            }
        }
        let platform = match self.platform {
            Some(p) => p,
            _ => match self.platform_dir {
                Some(dir) => {
                    Platform::from_path(dir).map_err(Error::CannotCreatePlatformFromPath)?
                }
                None => {
                    return Err(crate::error::Error::Other(
                        "Platform is missing, call platform() or platform_from_path()".to_string(),
                    ))
                }
            },
        };

        match vec![self.buildpack_toml.is_some(), self.buildpack_toml_string.is_some(), self.buildpack_toml_path.is_some()].iter().filter(|&&x| x).count() {
            0 => return Err(crate::error::Error::Other("buildpack_toml is missing, call buildpack_toml(), buildpack_toml_from_path(), or buildpack_toml_from_string()".to_string())),
            1 => {},
            _ => return Err(crate::error::Error::Other("Cannot determine buildpack_toml, call ONLY one: buildpack_toml(), buildpack_toml_from_path(), or buildpack_toml_from_string()".to_string())),
        }

        let buildpack_toml = match self.buildpack_toml {
            Some(buildpack_toml) => buildpack_toml,
            None => match self.buildpack_toml_path {
                Some(path) => crate::toml_file::read_toml_file(path)
                    .map_err(Error::CannotReadBuildpackDescriptor)?,
                None => match self.buildpack_toml_string {
                    Some(contents) => crate::toml_file::from_string(contents)
                        .map_err(Error::CannotReadBuildpackDescriptor)?,
                    None => return Err(crate::error::Error::Other("buildpack_toml is missing, call buildpack_toml(), buildpack_toml_from_path, or buildpack_toml_from_string".to_string()))
                }
            }
        };

        let app_dir = self
            .app_dir
            .ok_or_else(|| crate::error::Error::Other("missing app_dir".to_string()))?;
        let buildpack_dir = self
            .buildpack_dir
            .ok_or_else(|| crate::error::Error::Other("missing buildpack_dir".to_string()))?;

        Ok(DetectContext {
            app_dir,
            buildpack_dir,
            stack_id: self.stack_id,
            platform,
            buildpack_descriptor: buildpack_toml,
        })
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::{GenericDetectContext, GenericPlatform, Platform};

    pub fn fake_detect(_context: GenericDetectContext) {}

    #[test]
    fn test_from_temp_dir() {
        let temp_dir = tempfile::tempdir().unwrap();
        let context = DetectContextBuilder::new("heroku-20")
            .from_temp_dir(&temp_dir)
            .buildpack_toml_from_string(include_str!(
                "../examples/example-02-ruby-sample/buildpack.toml"
            ))
            .build::<std::io::Error>()
            .unwrap();

        std::fs::write(context.app_dir.join("Procfile"), "web: bundle exec rails s").unwrap();
        fake_detect(context);
    }

    #[test]
    fn test_buildpack_toml_from_string() {
        let context = DetectContextBuilder::new("heroku-20")
            .buildpack_toml_from_string(include_str!(
                "../examples/example-02-ruby-sample/buildpack.toml"
            ))
            .platform(GenericPlatform::from_path(".").unwrap())
            .app_dir(".")
            .buildpack_dir(".")
            .build::<std::io::Error>()
            .unwrap();

        fake_detect(context);
    }
}
