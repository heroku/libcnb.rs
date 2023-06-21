use crate::buildpackage::PlatformOs::Linux;
use serde::{Deserialize, Deserializer, Serialize, Serializer};
use std::path::PathBuf;
use uriparse::{URIReference, URIReferenceError};

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
    /// The buildpack to package.
    pub buildpack: BuildpackageBuildpackReference,

    /// A set of dependent buildpack locations, for packaging a meta-buildpack. Each dependent buildpack location must correspond to an order group within the meta-buildpack being packaged.
    #[serde(default)]
    pub dependencies: Vec<BuildpackageDependency>,

    /// The expected runtime environment for the buildpackage.
    #[serde(default)]
    pub platform: Platform,
}

impl Default for Buildpackage {
    fn default() -> Self {
        Buildpackage {
            buildpack: BuildpackageBuildpackReference::try_from(".")
                .expect("a package.toml with buildpack.uri=\".\" should be valid"),
            dependencies: vec![],
            platform: Platform::default(),
        }
    }
}

/// The buildpack to package.
#[derive(Debug, Deserialize, Serialize, Eq, PartialEq, Clone)]
#[serde(deny_unknown_fields)]
pub struct BuildpackageBuildpackReference {
    /// A URL or path to an archive, or a path to a directory.
    /// If the `uri` field is a relative path it will be relative to the `package.toml` file.
    #[serde(deserialize_with = "deserialize_uri_reference")]
    #[serde(serialize_with = "serialize_uri_reference")]
    pub uri: URIReference<'static>,
}

#[derive(Debug)]
pub enum BuildpackageBuildpackError {
    InvalidUri(String),
}

impl TryFrom<&str> for BuildpackageBuildpackReference {
    type Error = BuildpackageBuildpackError;

    fn try_from(value: &str) -> Result<Self, Self::Error> {
        try_uri_from_str(value)
            .map(|uri| BuildpackageBuildpackReference { uri })
            .map_err(|_| BuildpackageBuildpackError::InvalidUri(value.to_string()))
    }
}

/// A dependent buildpack location for packaging a meta-buildpack.
#[derive(Debug, Deserialize, Serialize, Eq, PartialEq, Clone)]
#[serde(deny_unknown_fields)]
pub struct BuildpackageDependency {
    /// A URL or path to an archive, a packaged buildpack (saved as a .cnb file), or a directory.
    /// If the `uri` field is a relative path it will be relative to the `package.toml` file.
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
    /// The operating system type that the buildpackage will run on.
    /// Only linux or windows is supported. If omitted, linux will be the default.
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

// Even though `uriparse` has Serde support it only works if the value we are deserializing is an
// map that contains URI fields like 'path', 'host', 'scheme', etc. The value from package.toml is
// just a plain string so we need this custom deserializer that will parse the value into
// a `URIReference`.
fn deserialize_uri_reference<'de, D>(deserializer: D) -> Result<URIReference<'static>, D::Error>
where
    D: Deserializer<'de>,
{
    let value = String::deserialize(deserializer)?;
    let uri = URIReference::try_from(value.as_str()).map_err(serde::de::Error::custom)?;
    Ok(uri.into_owned())
}

// The Serde support in `uriparse` wants to serialize our `URIReference` into a map of URI fields
// like 'path', 'host', 'scheme', etc. This custom serializer is needed to ensure the value is
// converted into a plain string value which is what is required for the package.toml format.
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
            BuildpackageBuildpackReference::try_from(".").unwrap()
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
            BuildpackageBuildpackReference::try_from(".").unwrap()
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
            buildpack: BuildpackageBuildpackReference::try_from(".").unwrap(),
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
