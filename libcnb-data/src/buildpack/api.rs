use std::convert::TryFrom;
use std::fmt::{Display, Formatter};
use std::{fmt, str::FromStr};

use fancy_regex::Regex;
use lazy_static::lazy_static;
use serde::Deserialize;

// Used as a "shadow" struct to store
// potentially invalid `BuildpackApi` data when deserializing
// <https://dev.to/equalma/validate-fields-and-types-in-serde-with-tryfrom-c2n>
#[derive(Deserialize)]
struct BuildpackApiUnchecked(String);

impl TryFrom<BuildpackApiUnchecked> for BuildpackApi {
    type Error = BuildpackApiError;

    fn try_from(value: BuildpackApiUnchecked) -> Result<Self, Self::Error> {
        Self::from_str(value.0.as_str())
    }
}

#[derive(Deserialize, Debug, Eq, PartialEq)]
#[serde(try_from = "BuildpackApiUnchecked")]
pub struct BuildpackApi {
    pub major: u32,
    pub minor: u32,
}

impl FromStr for BuildpackApi {
    type Err = BuildpackApiError;

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
                    .map_err(|_| Self::Err::InvalidBuildpackApi(String::from(value)))?;

                // If no minor version is specified default to 0.
                let minor = captures
                    .name("minor")
                    .map_or("0", |s| s.as_str())
                    .parse::<u32>()
                    .map_err(|_| Self::Err::InvalidBuildpackApi(String::from(value)))?;

                return Ok(Self { major, minor });
            }
        }

        Err(Self::Err::InvalidBuildpackApi(String::from(value)))
    }
}

impl Display for BuildpackApi {
    fn fmt(&self, formatter: &mut Formatter<'_>) -> fmt::Result {
        formatter.write_str(&format!("{}.{}", self.major, self.minor))
    }
}

#[derive(thiserror::Error, Debug)]
pub enum BuildpackApiError {
    #[error("Found `{0}` but value MUST be in the form `<major>.<minor>` or `<major>` and only contain numbers.")]
    InvalidBuildpackApi(String),
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn buildpack_api_from_str_major_minor() {
        let result = BuildpackApi::from_str("0.4");
        assert!(result.is_ok());
        if let Ok(api) = result {
            assert_eq!(0, api.major);
            assert_eq!(4, api.minor);
        }
    }

    #[test]
    fn buildpack_api_from_str_major() {
        let result = BuildpackApi::from_str("1");
        assert!(result.is_ok());
        if let Ok(api) = result {
            assert_eq!(1, api.major);
            assert_eq!(0, api.minor);
        }
    }

    #[test]
    fn buildpack_api_display() {
        assert_eq!(BuildpackApi { major: 1, minor: 0 }.to_string(), "1.0");
        assert_eq!(BuildpackApi { major: 1, minor: 2 }.to_string(), "1.2");
        assert_eq!(BuildpackApi { major: 0, minor: 5 }.to_string(), "0.5");
    }
}
