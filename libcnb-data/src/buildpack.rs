use crate::newtypes::libcnb_newtype;
use fancy_regex::Regex;
use lazy_static::lazy_static;
use semver::Version;
use serde::Deserialize;
use std::convert::TryFrom;
use std::fmt::{Display, Formatter};
use std::{fmt, str::FromStr};
use thiserror;

/// Data structure for the Buildpack descriptor (buildpack.toml).
///
/// # Examples
/// ```
/// use libcnb_data::buildpack::BuildpackToml;
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
/// description = "A buildpack for Foo Bar"
/// keywords = ["foo"]
///
/// [[buildpack.licenses]]
/// type = "BSD-3-Clause"
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
    #[serde(default)]
    pub clear_env: bool,
    pub description: Option<String>,
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub keywords: Vec<String>,
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub licenses: Vec<License>,
}

#[derive(Deserialize, Debug)]
pub struct License {
    pub r#type: Option<String>,
    pub uri: Option<String>,
}

#[derive(Deserialize, Debug)]
#[serde(try_from = "StackUnchecked")]
pub struct Stack {
    pub id: StackId,
    pub mixins: Vec<String>,
}

// Used as a "shadow" struct to store
// potentially invalid `Stack` data when deserializing
// https://dev.to/equalma/validate-fields-and-types-in-serde-with-tryfrom-c2n
#[derive(Deserialize)]
struct StackUnchecked {
    pub id: StackId,

    #[serde(default)]
    pub mixins: Vec<String>,
}

impl TryFrom<StackUnchecked> for Stack {
    type Error = BuildpackTomlError;

    fn try_from(value: StackUnchecked) -> Result<Self, Self::Error> {
        let StackUnchecked { id, mixins } = value;

        if id.as_str() == "*" && !mixins.is_empty() {
            Err(BuildpackTomlError::InvalidStarStack(mixins.join(", ")))
        } else {
            Ok(Self { id, mixins })
        }
    }
}

#[derive(Deserialize, Debug)]
pub struct Order {
    group: Vec<Group>,
}

#[derive(Deserialize, Debug)]
pub struct Group {
    pub id: BuildpackId,
    pub version: Version,
    #[serde(default)]
    pub optional: bool,
}

#[derive(Deserialize, Debug, Eq, PartialEq)]
#[serde(try_from = "BuildpackApiUnchecked")]
pub struct BuildpackApi {
    pub major: u32,
    pub minor: u32,
}
// Used as a "shadow" struct to store
// potentially invalid `BuildpackApi` data when deserializing
// <https://dev.to/equalma/validate-fields-and-types-in-serde-with-tryfrom-c2n>
#[derive(Deserialize)]
struct BuildpackApiUnchecked(String);

impl TryFrom<BuildpackApiUnchecked> for BuildpackApi {
    type Error = BuildpackTomlError;

    fn try_from(value: BuildpackApiUnchecked) -> Result<Self, Self::Error> {
        Self::from_str(value.0.as_str())
    }
}

impl FromStr for BuildpackApi {
    type Err = BuildpackTomlError;

    fn from_str(value: &str) -> Result<Self, Self::Err> {
        lazy_static! {
            static ref RE: Regex = Regex::new(r"^(?P<major>\d+)(\.(?P<minor>\d+))?$").unwrap();
        }

        if let Some(captures) = RE.captures(value).unwrap_or_default() {
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
                    .map_or("0", |s| s.as_str())
                    .parse::<u32>()
                    .map_err(|_| BuildpackTomlError::InvalidBuildpackApi(String::from(value)))?;

                return Ok(Self { major, minor });
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

libcnb_newtype!(
    buildpack,
    /// Construct a [`BuildpackId`] value at compile time.
    ///
    /// Passing a string that is not a valid `BuildpackId` value will yield a compilation error.
    ///
    /// # Examples:
    /// ```
    /// use libcnb_data::buildpack_id;
    /// use libcnb_data::buildpack::BuildpackId;
    ///
    /// let buildpack_id: BuildpackId = buildpack_id!("heroku/java");
    /// ```
    buildpack_id,
    /// The ID of a buildpack.
    ///
    /// It MUST only contain numbers, letters, and the characters `.`, `/`, and `-`.
    /// It also MUST NOT be `config` or `app`.
    ///
    /// Use the [`buildpack_id`](crate::buildpack_id) macro to construct a `BuildpackId` from a
    /// literal string. To parse a dynamic string into a `BuildpackId`, use
    /// [`str::parse`](str::parse).
    ///
    /// # Examples
    /// ```
    /// use libcnb_data::buildpack::BuildpackId;
    /// use libcnb_data::buildpack_id;
    ///
    /// let from_literal = buildpack_id!("heroku/jvm");
    ///
    /// let input = "heroku/jvm";
    /// let from_dynamic: BuildpackId = input.parse().unwrap();
    /// assert_eq!(from_dynamic, from_literal);
    ///
    /// let input = "app";
    /// let invalid: Result<BuildpackId, _> = input.parse();
    /// assert!(invalid.is_err());
    /// ```
    BuildpackId,
    BuildpackIdError,
    r"^(?!app$|config$)[[:alnum:]./-]+$"
);

libcnb_newtype!(
    buildpack,
    /// Construct a [`StackId`] value at compile time.
    ///
    /// Passing a string that is not a valid `StackId` value will yield a compilation error.
    ///
    /// # Examples:
    /// ```
    /// use libcnb_data::stack_id;
    /// use libcnb_data::buildpack::StackId;
    ///
    /// let stack_id: StackId = stack_id!("heroku-20");
    /// ```
    stack_id,
    /// The ID of a stack.
    ///
    /// It MUST only contain numbers, letters, and the characters `.`, `/`, and `-`.
    ///
    /// Use the [`stack_id`](crate::buildpack_id) macro to construct a `StackId` from a
    /// literal string. To parse a dynamic string into a `StackId`, use
    /// [`str::parse`](str::parse).
    ///
    /// # Examples
    /// ```
    /// use libcnb_data::buildpack::BuildpackId;
    /// use libcnb_data::buildpack_id;
    ///
    /// let from_literal = buildpack_id!("heroku/jvm");
    ///
    /// let input = "heroku/jvm";
    /// let from_dynamic: BuildpackId = input.parse().unwrap();
    /// assert_eq!(from_dynamic, from_literal);
    ///
    /// let input = "app";
    /// let invalid: Result<BuildpackId, _> = input.parse();
    /// assert!(invalid.is_err());
    /// ```
    StackId,
    StackIdError,
    r"^([[:alnum:]./-]+|\*)$"
);

#[derive(thiserror::Error, Debug)]
pub enum BuildpackTomlError {
    #[error("Found `{0}` but value MUST be in the form `<major>.<minor>` or `<major>` and only contain numbers.")]
    InvalidBuildpackApi(String),

    #[error("Stack with id `*` MUST not contain mixins. mixins: [{0}]")]
    InvalidStarStack(String),

    #[error("Invalid Stack ID: {0}")]
    InvalidStackId(#[from] StackIdError),

    #[error("Invalid Buildpack ID: {0}")]
    InvalidBuildpackId(#[from] BuildpackIdError),
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
    fn can_deserialize_metabuildpack() {
        let raw = r#"
api = "0.4"

[buildpack]
id = "foo/bar"
name = "Bar Buildpack"
version = "0.0.1"
homepage = "https://www.foo.com/bar"
clear-env = true
description = "A buildpack for Foo Bar"
keywords = ["foo", "bar"]

[[buildpack.licenses]]
type = "BSD-3-Clause"

[[buildpack.licenses]]
type = "Custom license with type and URI"
uri = "https://example.tld/my-license"

[[buildpack.licenses]]
uri = "https://example.tld/my-license"

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
        assert!(result.is_ok());
        if let Ok(toml) = result {
            assert_eq!(
                toml.buildpack.description.unwrap(),
                "A buildpack for Foo Bar"
            );
            assert_eq!(toml.buildpack.keywords.len(), 2);
            assert_eq!(toml.buildpack.licenses.len(), 3);
        }
    }

    #[test]
    fn can_deserialize_minimal_buildpack() {
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
    fn can_deserialize_minimal_metabuildpack() {
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
    fn cannot_use_star_stack_id_with_mixins() {
        let raw = r#"
api = "0.4"

[buildpack]
id = "foo/bar"
name = "Bar Buildpack"
version = "0.0.1"

[[stacks]]
id = "*"
mixins = ["yolo"]

[metadata]
checksum = "awesome"
"#;

        let result = toml::from_str::<BuildpackToml<toml::value::Table>>(raw);
        assert!(&result.is_err());
    }

    #[test]
    fn buildpack_api_display() {
        assert_eq!(BuildpackApi { major: 1, minor: 0 }.to_string(), "1.0");
        assert_eq!(BuildpackApi { major: 1, minor: 2 }.to_string(), "1.2");
        assert_eq!(BuildpackApi { major: 0, minor: 5 }.to_string(), "0.5");
    }
}
