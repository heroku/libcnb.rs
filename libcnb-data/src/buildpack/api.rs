use std::convert::TryFrom;
use std::fmt;
use std::fmt::{Display, Formatter};

use fancy_regex::Regex;
use lazy_static::lazy_static;
use serde::Deserialize;

/// The Buildpack API version.
///
/// This MUST be in form `<major>.<minor>` or `<major>`, where `<major>` is equivalent to `<major>.0`.
#[derive(Deserialize, Debug, Eq, PartialEq)]
#[serde(try_from = "&str")]
pub struct BuildpackApi {
    pub major: u32,
    pub minor: u32,
}

impl TryFrom<&str> for BuildpackApi {
    type Error = BuildpackApiError;

    fn try_from(value: &str) -> Result<Self, Self::Error> {
        // We're not using the `semver` crate, since it only supports non-range versions of form `X.Y.Z`.
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
                    .map_err(|_| Self::Error::InvalidBuildpackApi(String::from(value)))?;

                // If no minor version is specified default to 0.
                let minor = captures
                    .name("minor")
                    .map_or("0", |s| s.as_str())
                    .parse::<u32>()
                    .map_err(|_| Self::Error::InvalidBuildpackApi(String::from(value)))?;

                return Ok(Self { major, minor });
            }
        }

        Err(Self::Error::InvalidBuildpackApi(String::from(value)))
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
    use serde_test::{assert_de_tokens, assert_de_tokens_error, Token};

    use super::*;

    #[test]
    fn deserialize_valid_api_versions() {
        assert_de_tokens(
            &BuildpackApi { major: 1, minor: 3 },
            &[Token::BorrowedStr("1.3")],
        );
        assert_de_tokens(
            &BuildpackApi { major: 0, minor: 0 },
            &[Token::BorrowedStr("0.0")],
        );
        assert_de_tokens(
            &BuildpackApi {
                major: 2020,
                minor: 10,
            },
            &[Token::BorrowedStr("2020.10")],
        );
        assert_de_tokens(
            &BuildpackApi { major: 2, minor: 0 },
            &[Token::BorrowedStr("2")],
        );
    }

    #[test]
    fn reject_invalid_api_versions() {
        assert_de_tokens_error::<BuildpackApi>(
            &[Token::BorrowedStr("1.2.3")],
            "Found `1.2.3` but value MUST be in the form `<major>.<minor>` or `<major>` and only contain numbers.",
        );
        assert_de_tokens_error::<BuildpackApi>(
            &[Token::BorrowedStr("1.2-dev")],
             "Found `1.2-dev` but value MUST be in the form `<major>.<minor>` or `<major>` and only contain numbers.",
        );
        assert_de_tokens_error::<BuildpackApi>(
            &[Token::BorrowedStr("-1")],
             "Found `-1` but value MUST be in the form `<major>.<minor>` or `<major>` and only contain numbers.",
        );
        assert_de_tokens_error::<BuildpackApi>(
            &[Token::BorrowedStr(".1")],
             "Found `.1` but value MUST be in the form `<major>.<minor>` or `<major>` and only contain numbers.",
        );
        assert_de_tokens_error::<BuildpackApi>(
            &[Token::BorrowedStr("1.")],
             "Found `1.` but value MUST be in the form `<major>.<minor>` or `<major>` and only contain numbers.",
        );
        assert_de_tokens_error::<BuildpackApi>(
            &[Token::BorrowedStr("1..2")],
             "Found `1..2` but value MUST be in the form `<major>.<minor>` or `<major>` and only contain numbers.",
        );
        assert_de_tokens_error::<BuildpackApi>(
            &[Token::BorrowedStr("")],
             "Found `` but value MUST be in the form `<major>.<minor>` or `<major>` and only contain numbers.",
        );
    }

    #[test]
    fn buildpack_api_display() {
        assert_eq!(BuildpackApi { major: 1, minor: 0 }.to_string(), "1.0");
        assert_eq!(BuildpackApi { major: 1, minor: 2 }.to_string(), "1.2");
        assert_eq!(
            BuildpackApi {
                major: 0,
                minor: 10
            }
            .to_string(),
            "0.10"
        );
    }
}
