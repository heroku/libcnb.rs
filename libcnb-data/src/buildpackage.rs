use crate::buildpackage::PlatformOs::Linux;
use serde::{Deserialize, Deserializer, Serialize, Serializer};
use std::path::PathBuf;
use uriparse::{URIReference, URIReferenceError};

fn platform_default() -> Platform {
    Platform::default()
}

/// Data structure for the Buildpackage configuration schema (package.toml) for a buildpack.
///
/// Representation of [package.toml](https://buildpacks.io/docs/reference/config/package-config/).
///
/// # Example
/// ```
/// use libcnb_data::buildpackage::Buildpackage;
///
/// let toml_str = r#"
/// [buildpack]
/// uri = "."
///
/// [[dependencies]]
/// uri = "libcnb:buildpack_id"
///
/// [[dependencies]]
/// uri = "../relative/path"
///
/// [[dependencies]]
/// uri = "/absolute/path"
///
/// [[dependencies]]
/// uri = "docker://docker.io/heroku/procfile-cnb:2.0.0"
///
/// [platform]
/// os = "windows"
/// "#;
///
/// let buildpackage = toml::from_str::<Buildpackage>(toml_str).unwrap();
/// ```
#[derive(Debug, Deserialize, Serialize, Clone)]
#[serde(deny_unknown_fields)]
pub struct Buildpackage {
    pub buildpack: BuildpackageBuildpack,
    #[serde(default)]
    pub dependencies: Vec<BuildpackageDependency>,
    #[serde(default = "platform_default")]
    pub platform: Platform,
}

impl Default for Buildpackage {
    fn default() -> Self {
        Buildpackage {
            buildpack: BuildpackageBuildpack::try_from(".").expect("This must be a valid uri"),
            dependencies: vec![],
            platform: Platform::default(),
        }
    }
}

/// The buildpack to package. If the `uri` field is a relative path it will be relative to the `package.toml` file.
#[derive(Debug, Deserialize, Serialize, Eq, PartialEq, Clone)]
#[serde(deny_unknown_fields)]
pub struct BuildpackageBuildpack {
    #[serde(deserialize_with = "deserialize_uri_reference")]
    #[serde(serialize_with = "serialize_uri_reference")]
    pub uri: URIReference<'static>,
}

#[derive(Debug)]
pub enum BuildpackageBuildpackError {
    InvalidUri(String),
}

impl TryFrom<&str> for BuildpackageBuildpack {
    type Error = BuildpackageBuildpackError;

    fn try_from(value: &str) -> Result<Self, Self::Error> {
        try_uri_from_str(value)
            .map(|uri| BuildpackageBuildpack { uri })
            .map_err(|_| BuildpackageBuildpackError::InvalidUri(value.to_string()))
    }
}

/// A dependent buildpack location for packaging a meta-buildpack. If the `uri` field is a relative
/// path it will be relative to the `package.toml` file.
#[derive(Debug, Deserialize, Serialize, Eq, PartialEq, Clone)]
#[serde(deny_unknown_fields)]
pub struct BuildpackageDependency {
    #[serde(deserialize_with = "deserialize_uri_reference")]
    #[serde(serialize_with = "serialize_uri_reference")]
    pub uri: URIReference<'static>,
}

#[derive(Debug)]
pub enum BuildpackageDependencyError {
    InvalidUri(String),
}

impl TryFrom<PathBuf> for BuildpackageDependency {
    type Error = BuildpackageDependencyError;

    fn try_from(value: PathBuf) -> Result<Self, Self::Error> {
        Self::try_from(value.to_string_lossy().to_string().as_str())
    }
}

impl TryFrom<&str> for BuildpackageDependency {
    type Error = BuildpackageDependencyError;

    fn try_from(value: &str) -> Result<Self, Self::Error> {
        try_uri_from_str(value)
            .map(|uri| BuildpackageDependency { uri })
            .map_err(|_| BuildpackageDependencyError::InvalidUri(value.to_string()))
    }
}

fn try_uri_from_str(value: &str) -> Result<URIReference<'static>, URIReferenceError> {
    URIReference::try_from(value).map(URIReference::into_owned)
}

/// The expected runtime environment for the buildpackage.
#[derive(Debug, Deserialize, Serialize, Clone)]
#[serde(deny_unknown_fields)]
pub struct Platform {
    pub os: PlatformOs,
}

impl Default for Platform {
    fn default() -> Self {
        Self { os: Linux }
    }
}

#[derive(Debug, Deserialize, Serialize, Eq, PartialEq, Clone)]
#[serde(rename_all = "lowercase")]
pub enum PlatformOs {
    Linux,
    Windows,
}

fn deserialize_uri_reference<'de, D>(deserializer: D) -> Result<URIReference<'static>, D::Error>
where
    D: Deserializer<'de>,
{
    let value = String::deserialize(deserializer)?;
    let uri = URIReference::try_from(value.as_str()).map_err(serde::de::Error::custom)?;
    Ok(uri.into_owned())
}

fn serialize_uri_reference<S>(uri: &URIReference, serializer: S) -> Result<S::Ok, S::Error>
where
    S: Serializer,
{
    let value = uri.to_string();
    serializer.serialize_str(value.as_str())
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::buildpackage::PlatformOs::Windows;

    #[test]
    fn it_parses_minimal() {
        let toml_str = r#"
[buildpack]
uri = "."
"#;

        let buildpackage = toml::from_str::<Buildpackage>(toml_str).unwrap();
        assert_eq!(
            buildpackage.buildpack,
            BuildpackageBuildpack::try_from(".").unwrap()
        );
        assert_eq!(buildpackage.platform.os, Linux);
    }

    #[test]
    fn it_parses_with_dependencies_and_platform() {
        let toml_str = r#"
[buildpack]
uri = "."

[[dependencies]]
uri = "libcnb:buildpack-id"

[[dependencies]]
uri = "../relative/path"

[[dependencies]]
uri = "/absolute/path"

[[dependencies]]
uri = "docker://docker.io/heroku/procfile-cnb:2.0.0"

[platform]
os = "windows"
"#;

        let buildpackage = toml::from_str::<Buildpackage>(toml_str).unwrap();
        assert_eq!(
            buildpackage.buildpack,
            BuildpackageBuildpack::try_from(".").unwrap()
        );
        assert_eq!(buildpackage.platform.os, Windows);
        assert_eq!(
            buildpackage.dependencies,
            vec![
                BuildpackageDependency::try_from("libcnb:buildpack-id").unwrap(),
                BuildpackageDependency::try_from("../relative/path").unwrap(),
                BuildpackageDependency::try_from("/absolute/path").unwrap(),
                BuildpackageDependency::try_from("docker://docker.io/heroku/procfile-cnb:2.0.0")
                    .unwrap()
            ]
        );
    }

    #[test]
    fn it_serializes() {
        let buildpackage = Buildpackage {
            buildpack: BuildpackageBuildpack::try_from(".").unwrap(),
            dependencies: vec![
                BuildpackageDependency::try_from("libcnb:buildpack-id").unwrap(),
                BuildpackageDependency::try_from("../relative/path").unwrap(),
                BuildpackageDependency::try_from("/absolute/path").unwrap(),
                BuildpackageDependency::try_from("docker://docker.io/heroku/procfile-cnb:2.0.0")
                    .unwrap(),
            ],
            platform: Platform::default(),
        };

        let buildpackage_contents = toml::to_string(&buildpackage).unwrap();
        assert_eq!(
            buildpackage_contents,
            r#"
[buildpack]
uri = "."

[[dependencies]]
uri = "libcnb:buildpack-id"

[[dependencies]]
uri = "../relative/path"

[[dependencies]]
uri = "/absolute/path"

[[dependencies]]
uri = "docker://docker.io/heroku/procfile-cnb:2.0.0"

[platform]
os = "linux"
"#
            .trim_start()
        );
    }
}
