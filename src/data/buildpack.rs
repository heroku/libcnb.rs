use crate::Error;
use lazy_static::lazy_static;
use regex::Regex;
use semver::Version;
use serde::Deserialize;
use std::str::FromStr;

#[derive(Deserialize, Debug)]
pub struct BuildpackToml {
    // MUST be in form <major>.<minor> or <major>, where <major> is equivalent to <major>.0.
    pub api: String,
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
    pub id: String,
    pub mixins: Vec<String>,
}

/// buildpack.toml Buildpack Id. This is a newtype wrapper around a String. It MUST only contain numbers, letters, and the characters ., /, and -. It also cannot be `config` or `app`. Use [`std::str::FromStr`] to create a new instance of this struct.
///
/// # Examples
/// ```
/// use std::str::FromStr;
/// use libcnb::data::buildpack::BuildpackId;
///
/// let valid = BuildpackId::from_str("heroku/ruby-engine.mri3");
/// assert_eq!(valid.unwrap().as_str(), "heroku/ruby-engine.mri3");
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
            static ref RE: Regex = Regex::new("^[[:alnum:]./-]+$").unwrap();
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
