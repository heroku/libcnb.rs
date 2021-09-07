use crate::data::defaults;
use lazy_static::lazy_static;
use regex::Regex;
use semver::Version;
use serde::{de, Deserialize};
use std::fmt::{Display, Formatter};
use std::{fmt, str::FromStr};
use thiserror;

/// Data structure for the Buildpack descriptor (buildpack.toml).
///
/// # Examples
/// ```
/// use libcnb::data::buildpack::BuildpackToml;
///
///         let raw = r#"
/// api = "0.4"
///
/// [buildpack]
/// id = "foo/bar"
/// name = "Bar Buildpack"
/// version = "0.0.1"
/// homepage = "https://www.foo.com/bar"
/// clear-env = false
///
/// [[stacks]]
/// id = "io.buildpacks.stacks.bionic"
/// mixins = ["yj", "yq"]
///
/// [metadata]
/// checksum = "awesome"
/// "#;
///
///         let result = toml::from_str::<BuildpackToml<toml::value::Table>>(raw);
///         assert!(result.is_ok());
/// ```
#[derive(Deserialize, Debug)]
pub struct BuildpackToml<BM> {
    // MUST be in form <major>.<minor> or <major>, where <major> is equivalent to <major>.0.
    pub api: BuildpackApi,
    pub buildpack: Buildpack,
    pub stacks: Vec<Stack>,
    #[serde(default)]
    pub order: Vec<Order>,
    pub metadata: BM,
}

#[derive(Deserialize, Debug)]
pub struct Buildpack {
    pub id: BuildpackId,
    pub name: String,
    // MUST be in the form <X>.<Y>.<Z> where X, Y, and Z are non-negative integers and must not contain leading zeroes
    pub version: Version,
    pub homepage: Option<String>,
    #[serde(rename = "clear-env")]
    #[serde(default = "defaults::r#false")]
    pub clear_env: bool,
}

#[derive(Deserialize, Debug)]
pub struct Stack {
    pub id: StackId,
    #[serde(default)]
    pub mixins: Vec<String>,
}

#[derive(Deserialize, Debug)]
pub struct Order {
    group: Vec<Group>,
}

#[derive(Deserialize, Debug)]
pub struct Group {
    pub id: BuildpackId,
    pub version: Version,
    #[serde(default = "defaults::r#false")]
    pub optional: bool,
}

#[derive(Debug, Eq, PartialEq)]
pub struct BuildpackApi {
    pub major: u32,
    pub minor: u32,
}

impl FromStr for BuildpackApi {
    type Err = BuildpackTomlError;

    fn from_str(value: &str) -> Result<Self, Self::Err> {
        lazy_static! {
            static ref RE: Regex = Regex::new(r"^(?P<major>\d+)(\.(?P<minor>\d+))?$").unwrap();
        }

        if let Some(captures) = RE.captures(value) {
            if let Some(major) = captures.name("major") {
                // these should never panic since we check with the regex unless it's greater than
                // `std::u32::MAX`
                let major = major
                    .as_str()
                    .parse::<u32>()
                    .map_err(|_| BuildpackTomlError::InvalidBuildpackApi(String::from(value)))?;

                // If no minor version is specified default to 0.
                let minor = captures
                    .name("minor")
                    .map(|s| s.as_str())
                    .unwrap_or("0")
                    .parse::<u32>()
                    .map_err(|_| BuildpackTomlError::InvalidBuildpackApi(String::from(value)))?;

                return Ok(BuildpackApi { major, minor });
            }
        }

        Err(BuildpackTomlError::InvalidBuildpackApi(String::from(value)))
    }
}

impl Display for BuildpackApi {
    fn fmt(&self, formatter: &mut Formatter<'_>) -> fmt::Result {
        formatter.write_str(&format!("{}.{}", self.major, self.minor))
    }
}

impl<'de> de::Deserialize<'de> for BuildpackApi {
    fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
    where
        D: de::Deserializer<'de>,
    {
        struct BuildpackApiVisitor;

        impl<'de> de::Visitor<'de> for BuildpackApiVisitor {
            type Value = BuildpackApi;

            fn expecting(&self, formatter: &mut fmt::Formatter) -> fmt::Result {
                formatter.write_str(
                    "a BuildpackApi as a string which MUST be in form <major>.<minor> or <major>",
                )
            }

            fn visit_str<E>(self, v: &str) -> Result<Self::Value, E>
            where
                E: de::Error,
            {
                BuildpackApi::from_str(v).map_err(de::Error::custom)
            }
        }

        deserializer.deserialize_str(BuildpackApiVisitor)
    }
}

/// buildpack.toml Buildpack Id. This is a newtype wrapper around a String. It MUST only contain numbers, letters, and the characters ., /, and -. It also cannot be `config` or `app`. Use [`std::str::FromStr`] to create a new instance of this struct.
///
/// # Examples
/// ```
/// use std::str::FromStr;
/// use libcnb::data::buildpack::BuildpackId;
///
/// let valid = BuildpackId::from_str("heroku/ruby-engine.MRI3");
/// assert_eq!(valid.unwrap().as_str(), "heroku/ruby-engine.MRI3");
///
/// let invalid = BuildpackId::from_str("!nvalid");
/// assert!(invalid.is_err());
/// ```
#[derive(Deserialize, Debug)]
pub struct BuildpackId(String);

impl FromStr for BuildpackId {
    type Err = BuildpackTomlError;

    fn from_str(value: &str) -> Result<Self, Self::Err> {
        lazy_static! {
            static ref RE: Regex = Regex::new(r"^[[:alnum:]./-]+$").unwrap();
        }

        let string = String::from(value);
        if value != "app" && value != "config" && RE.is_match(value) {
            Ok(BuildpackId(string))
        } else {
            Err(BuildpackTomlError::InvalidBuildpackId(string))
        }
    }
}

impl BuildpackId {
    pub fn as_str(&self) -> &str {
        &self.0
    }
}

/// buildpack.toml Stack Id. This is a newtype wrapper around a String. It MUST only contain numbers, letters, and the characters ., /, and -. Use [`std::str::FromStr`] to create a new instance of this struct.
///
/// # Examples
/// ```
/// use std::str::FromStr;
/// use libcnb::data::buildpack::StackId;
///
/// let valid = StackId::from_str("io.buildpacks.bionic/Latest-2020");
/// assert_eq!(valid.unwrap().as_str(), "io.buildpacks.bionic/Latest-2020");
///
/// let invalid = StackId::from_str("!nvalid");
/// assert!(invalid.is_err());
/// ```

#[derive(Deserialize, Debug)]
pub struct StackId(String);

impl FromStr for StackId {
    type Err = BuildpackTomlError;

    fn from_str(value: &str) -> Result<Self, Self::Err> {
        lazy_static! {
            static ref RE: Regex = Regex::new(r"^[[:alnum:]./-]+$").unwrap();
        }

        let string = String::from(value);
        if RE.is_match(value) {
            Ok(StackId(string))
        } else {
            Err(BuildpackTomlError::InvalidStackId(string))
        }
    }
}

impl StackId {
    pub fn as_str(&self) -> &str {
        &self.0
    }
}

#[derive(thiserror::Error, Debug)]
pub enum BuildpackTomlError {
    #[error("Found `{0}` but value MUST only contain numbers, letters, and the characters ., /, and -. Value MUST NOT be 'config' or 'app'.")]
    InvalidBuildpackApi(String),

    #[error(
        "Found `{0}` but value MUST only contain numbers, letters, and the characters ., /, and -."
    )]
    InvalidStackId(String),

    #[error("Found `{0}` but value MUST only contain numbers, letters, and the characters ., /, and -. Value MUST NOT be 'config' or 'app'.")]
    InvalidBuildpackId(String),
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn buildpack_id_does_not_allow_app() {
        let result = BuildpackId::from_str("app");
        assert!(result.is_err());
    }

    #[test]
    fn buildpack_id_does_not_allow_config() {
        let result = BuildpackId::from_str("config");
        assert!(result.is_err());
    }

    #[test]
    fn buildpack_id_from_str_major_minor() {
        let result = BuildpackApi::from_str("0.4");
        assert!(result.is_ok());
        if let Ok(api) = result {
            assert_eq!(0, api.major);
            assert_eq!(4, api.minor);
        }
    }

    #[test]
    fn buildpack_id_from_str_major() {
        let result = BuildpackApi::from_str("1");
        assert!(result.is_ok());
        if let Ok(api) = result {
            assert_eq!(1, api.major);
            assert_eq!(0, api.minor);
        }
    }

    #[test]
    fn can_serialize_metabuildpack() {
        let raw = r#"
api = "0.4"

[buildpack]
id = "foo/bar"
name = "Bar Buildpack"
version = "0.0.1"
homepage = "https://www.foo.com/bar"
clear-env = false

[[order]]
[[order.group]]
id = "foo/baz"
version = "0.0.2"
optional = false

[[stacks]]
id = "io.buildpacks.stacks.bionic"
mixins = ["yj", "yq"]

[metadata]
checksum = "awesome"
"#;

        let result = toml::from_str::<BuildpackToml<toml::value::Table>>(raw);
        result.unwrap();
        //assert!(result.is_ok());
    }

    #[test]
    fn can_serialize_minimal_buildpack() {
        let raw = r#"
api = "0.4"

[buildpack]
id = "foo/bar"
name = "Bar Buildpack"
version = "0.0.1"

[[stacks]]
id = "io.buildpacks.stacks.bionic"

[metadata]
checksum = "awesome"
"#;

        let result = toml::from_str::<BuildpackToml<toml::value::Table>>(raw);
        assert!(result.is_ok());
        if let Ok(toml) = result {
            assert!(!toml.buildpack.clear_env);
        }
    }

    #[test]
    fn can_serialize_minimal_metabuildpack() {
        let raw = r#"
api = "0.4"

[buildpack]
id = "foo/bar"
name = "Bar Buildpack"
version = "0.0.1"

[[order]]
[[order.group]]
id = "foo/baz"
version = "0.0.2"

[[stacks]]
id = "io.buildpacks.stacks.bionic"
"#;

        let result = toml::from_str::<BuildpackToml<Option<toml::value::Table>>>(raw);
        assert!(result.is_ok());
        if let Ok(toml) = result {
            assert!(!toml.order.get(0).unwrap().group.get(0).unwrap().optional);
        }
    }

    #[test]
    fn buildpack_api_display() {
        assert_eq!(BuildpackApi { major: 1, minor: 0 }.to_string(), "1.0");
        assert_eq!(BuildpackApi { major: 1, minor: 2 }.to_string(), "1.2");
        assert_eq!(BuildpackApi { major: 0, minor: 5 }.to_string(), "0.5");
    }
}
