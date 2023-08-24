use crate::package_descriptor::PlatformOs::Linux;
use serde::{Deserialize, Deserializer, Serialize, Serializer};
use std::path::PathBuf;
use uriparse::{URIReference, URIReferenceError};

/// Representation of [package.toml](https://buildpacks.io/docs/reference/config/package-config/).
///
/// # Example
/// ```
/// use libcnb_data::package_descriptor::PackageDescriptor;
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
/// uri = "docker://docker.io/heroku/example:1.2.3"
///
/// [platform]
/// os = "windows"
/// "#;
///
/// toml::from_str::<PackageDescriptor>(toml_str).unwrap();
/// ```
#[derive(Debug, Deserialize, Serialize, Clone)]
#[serde(deny_unknown_fields)]
pub struct PackageDescriptor {
    /// The buildpack to package.
    pub buildpack: PackageDescriptorBuildpackReference,

    /// A set of dependent buildpack locations, for packaging a meta-buildpack. Each dependent buildpack location must correspond to an order group within the meta-buildpack being packaged.
    #[serde(default)]
    pub dependencies: Vec<PackageDescriptorDependency>,

    /// The expected runtime environment for the packaged buildpack.
    #[serde(default)]
    pub platform: Platform,
}

impl Default for PackageDescriptor {
    fn default() -> Self {
        PackageDescriptor {
            buildpack: PackageDescriptorBuildpackReference::try_from(".")
                .expect("a package.toml with buildpack.uri=\".\" should be valid"),
            dependencies: vec![],
            platform: Platform::default(),
        }
    }
}

/// The buildpack to package.
#[derive(Debug, Deserialize, Serialize, Eq, PartialEq, Clone)]
#[serde(deny_unknown_fields)]
pub struct PackageDescriptorBuildpackReference {
    /// A URL or path to an archive, or a path to a directory.
    /// If the `uri` field is a relative path it will be relative to the `package.toml` file.
    #[serde(deserialize_with = "deserialize_uri_reference")]
    #[serde(serialize_with = "serialize_uri_reference")]
    pub uri: URIReference<'static>,
}

#[derive(Debug)]
pub enum PackageDescriptorBuildpackError {
    InvalidUri(String),
}

impl TryFrom<&str> for PackageDescriptorBuildpackReference {
    type Error = PackageDescriptorBuildpackError;

    fn try_from(value: &str) -> Result<Self, Self::Error> {
        try_uri_from_str(value)
            .map(|uri| PackageDescriptorBuildpackReference { uri })
            .map_err(|_| PackageDescriptorBuildpackError::InvalidUri(value.to_string()))
    }
}

/// A dependent buildpack location for packaging a meta-buildpack.
#[derive(Debug, Deserialize, Serialize, Eq, PartialEq, Clone)]
#[serde(deny_unknown_fields)]
pub struct PackageDescriptorDependency {
    /// A URL or path to an archive, a packaged buildpack (saved as a .cnb file), or a directory.
    /// If the `uri` field is a relative path it will be relative to the `package.toml` file.
    #[serde(deserialize_with = "deserialize_uri_reference")]
    #[serde(serialize_with = "serialize_uri_reference")]
    pub uri: URIReference<'static>,
}

#[derive(Debug)]
pub enum PackageDescriptorDependencyError {
    InvalidUri(String),
}

impl TryFrom<PathBuf> for PackageDescriptorDependency {
    type Error = PackageDescriptorDependencyError;

    fn try_from(value: PathBuf) -> Result<Self, Self::Error> {
        Self::try_from(value.to_string_lossy().to_string().as_str())
    }
}

impl TryFrom<&str> for PackageDescriptorDependency {
    type Error = PackageDescriptorDependencyError;

    fn try_from(value: &str) -> Result<Self, Self::Error> {
        try_uri_from_str(value)
            .map(|uri| PackageDescriptorDependency { uri })
            .map_err(|_| PackageDescriptorDependencyError::InvalidUri(value.to_string()))
    }
}

fn try_uri_from_str(value: &str) -> Result<URIReference<'static>, URIReferenceError> {
    URIReference::try_from(value).map(URIReference::into_owned)
}

/// The expected runtime environment for the packaged buildpack.
#[derive(Debug, Deserialize, Serialize, Clone)]
#[serde(deny_unknown_fields)]
pub struct Platform {
    /// The operating system type that the packaged buildpack will run on.
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
    use crate::package_descriptor::PlatformOs::Windows;

    #[test]
    fn it_parses_minimal() {
        let toml_str = r#"
[buildpack]
uri = "."
"#;

        let package_descriptor = toml::from_str::<PackageDescriptor>(toml_str).unwrap();
        assert_eq!(
            package_descriptor.buildpack,
            PackageDescriptorBuildpackReference::try_from(".").unwrap()
        );
        assert_eq!(package_descriptor.platform.os, Linux);
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
uri = "docker://docker.io/heroku/example:1.2.3"

[platform]
os = "windows"
"#;

        let package_descriptor = toml::from_str::<PackageDescriptor>(toml_str).unwrap();
        assert_eq!(
            package_descriptor.buildpack,
            PackageDescriptorBuildpackReference::try_from(".").unwrap()
        );
        assert_eq!(package_descriptor.platform.os, Windows);
        assert_eq!(
            package_descriptor.dependencies,
            vec![
                PackageDescriptorDependency::try_from("libcnb:buildpack-id").unwrap(),
                PackageDescriptorDependency::try_from("../relative/path").unwrap(),
                PackageDescriptorDependency::try_from("/absolute/path").unwrap(),
                PackageDescriptorDependency::try_from("docker://docker.io/heroku/example:1.2.3")
                    .unwrap()
            ]
        );
    }

    #[test]
    fn it_serializes() {
        let package_descriptor = PackageDescriptor {
            buildpack: PackageDescriptorBuildpackReference::try_from(".").unwrap(),
            dependencies: vec![
                PackageDescriptorDependency::try_from("libcnb:buildpack-id").unwrap(),
                PackageDescriptorDependency::try_from("../relative/path").unwrap(),
                PackageDescriptorDependency::try_from("/absolute/path").unwrap(),
                PackageDescriptorDependency::try_from("docker://docker.io/heroku/example:1.2.3")
                    .unwrap(),
            ],
            platform: Platform::default(),
        };

        let package_descriptor_contents = toml::to_string(&package_descriptor).unwrap();
        assert_eq!(
            package_descriptor_contents,
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
uri = "docker://docker.io/heroku/example:1.2.3"

[platform]
os = "linux"
"#
            .trim_start()
        );
    }
}
