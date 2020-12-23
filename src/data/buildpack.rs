use crate::Error;
use lazy_static::lazy_static;
use regex::Regex;
use semver::Version;
use serde::{de, Deserialize};
use std::{fmt, str::FromStr};

#[derive(Deserialize, Debug)]
pub struct BuildpackToml {
    // MUST be in form <major>.<minor> or <major>, where <major> is equivalent to <major>.0.
    pub api: BuildpackApi,
    pub buildpack: Buildpack,
    pub stacks: Vec<Stack>,
    pub metadata: toml::value::Table,
}

#[derive(Deserialize, Debug)]
pub struct Buildpack {
    pub id: BuildpackId,
    pub name: String,
    // MUST be in the form <X>.<Y>.<Z> where X, Y, and Z are non-negative integers and must not contain leading zeroes
    pub version: Version,
    pub homepage: Option<String>,
    pub clear_env: bool,
}

#[derive(Deserialize, Debug)]
pub struct Stack {
    pub id: StackId,
    pub mixins: Vec<String>,
}

#[derive(Debug)]
pub struct BuildpackApi {
    pub major: u32,
    pub minor: u32,
}

impl FromStr for BuildpackApi {
    type Err = Error;

    fn from_str(value: &str) -> Result<Self, Self::Err> {
        lazy_static! {
            static ref RE: Regex = Regex::new(r"^(?P<major>\d+)\.(?P<minor>\d*)$").unwrap();
        }

        let string = String::from(value);
        if let Some(captures) = RE.captures(value) {
            if let Some(major) = captures.name("major") {
                // these should never panic since we check with the regex unless it's greater than
                // `std::u32::MAX`
                let major = match major.as_str().parse::<u32>() {
                    Ok(parsed) => parsed,
                    Err(_) => return Err(Error::InvalidBuildpackApi(string)),
                };
                // If no minor version is specified default to 0.
                let minor = match captures
                    .name("minor")
                    .map(|s| s.as_str())
                    .unwrap_or("0")
                    .parse::<u32>()
                {
                    Ok(parsed) => parsed,
                    Err(_) => return Err(Error::InvalidBuildpackApi(string)),
                };

                return Ok(BuildpackApi { major, minor });
            }
        }

        Err(Error::InvalidBuildpackApi(string))
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
    type Err = Error;

    fn from_str(value: &str) -> Result<Self, Self::Err> {
        lazy_static! {
            static ref RE: Regex = Regex::new(r"^[[:alnum:]./-]+$").unwrap();
        }

        let string = String::from(value);
        if value != "app" && value != "config" && RE.is_match(value) {
            Ok(BuildpackId(string))
        } else {
            Err(Error::InvalidBuildpackId(string))
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
    type Err = Error;

    fn from_str(value: &str) -> Result<Self, Self::Err> {
        lazy_static! {
            static ref RE: Regex = Regex::new(r"^[[:alnum:]./-]+$").unwrap();
        }

        let string = String::from(value);
        if RE.is_match(value) {
            Ok(StackId(string))
        } else {
            Err(Error::InvalidStackId(string))
        }
    }
}

impl StackId {
    pub fn as_str(&self) -> &str {
        &self.0
    }
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
}
