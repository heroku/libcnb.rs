use std::convert::TryFrom;
use std::fmt;
use std::fmt::{Display, Formatter};

use fancy_regex::Regex;
use serde::Deserialize;

/// The Buildpack version.
///
/// This MUST be in the form `<X>.<Y>.<Z>` where `X`, `Y`, and `Z` are non-negative integers
/// and must not contain leading zeros.
#[derive(Deserialize, Debug, Eq, PartialEq)]
#[serde(try_from = "&str")]
pub struct BuildpackVersion {
    pub major: u64,
    pub minor: u64,
    pub patch: u64,
}

impl BuildpackVersion {
    #[must_use]
    pub fn new(major: u64, minor: u64, patch: u64) -> Self {
        Self {
            major,
            minor,
            patch,
        }
    }
}

impl TryFrom<&str> for BuildpackVersion {
    type Error = BuildpackVersionError;

    fn try_from(value: &str) -> Result<Self, Self::Error> {
        // We're not using the `semver` crate, since semver versions also permit pre-release and
        // build metadata suffixes. We have to use regex (vs just `.split(".")`), since the spec
        // forbids redundant leading zeros, and `std::parse()` otherwise silently ignores them.
        let re = Regex::new(r"^(0|[1-9]\d*)\.(0|[1-9]\d*)\.(0|[1-9]\d*)$").unwrap();
        re.captures(value)
            .unwrap_or_default()
            .and_then(|captures| {
                Some(Self::new(
                    captures.get(1)?.as_str().parse().ok()?,
                    captures.get(2)?.as_str().parse().ok()?,
                    captures.get(3)?.as_str().parse().ok()?,
                ))
            })
            .ok_or_else(|| Self::Error::InvalidBuildpackVersion(String::from(value)))
    }
}

impl Display for BuildpackVersion {
    fn fmt(&self, formatter: &mut Formatter<'_>) -> fmt::Result {
        formatter.write_str(&format!("{}.{}.{}", self.major, self.minor, self.patch))
    }
}

#[derive(thiserror::Error, Debug)]
pub enum BuildpackVersionError {
    #[error("Invalid buildpack version: `{0}`")]
    InvalidBuildpackVersion(String),
}

#[cfg(test)]
mod tests {
    use serde_test::{assert_de_tokens, assert_de_tokens_error, Token};

    use super::*;

    #[test]
    fn deserialize_valid_versions() {
        assert_de_tokens(
            &BuildpackVersion {
                major: 1,
                minor: 3,
                patch: 4,
            },
            &[Token::BorrowedStr("1.3.4")],
        );
        assert_de_tokens(
            &BuildpackVersion {
                major: 0,
                minor: 0,
                patch: 0,
            },
            &[Token::BorrowedStr("0.0.0")],
        );
        assert_de_tokens(
            &BuildpackVersion {
                major: 1234,
                minor: 5678,
                patch: 9876,
            },
            &[Token::BorrowedStr("1234.5678.9876")],
        );
    }

    #[test]
    fn reject_wrong_number_of_version_parts() {
        assert_de_tokens_error::<BuildpackVersion>(
            &[Token::BorrowedStr("")],
            "Invalid buildpack version: ``",
        );
        assert_de_tokens_error::<BuildpackVersion>(
            &[Token::BorrowedStr("12345")],
            "Invalid buildpack version: `12345`",
        );
        assert_de_tokens_error::<BuildpackVersion>(
            &[Token::BorrowedStr("1.2")],
            "Invalid buildpack version: `1.2`",
        );
        assert_de_tokens_error::<BuildpackVersion>(
            &[Token::BorrowedStr("1.2.3.4")],
            "Invalid buildpack version: `1.2.3.4`",
        );
        assert_de_tokens_error::<BuildpackVersion>(
            &[Token::BorrowedStr(".2.3")],
            "Invalid buildpack version: `.2.3`",
        );
        assert_de_tokens_error::<BuildpackVersion>(
            &[Token::BorrowedStr("1.2.")],
            "Invalid buildpack version: `1.2.`",
        );
        assert_de_tokens_error::<BuildpackVersion>(
            &[Token::BorrowedStr("1..3")],
            "Invalid buildpack version: `1..3`",
        );
        assert_de_tokens_error::<BuildpackVersion>(
            &[Token::BorrowedStr("1..2.3")],
            "Invalid buildpack version: `1..2.3`",
        );
        assert_de_tokens_error::<BuildpackVersion>(
            &[Token::BorrowedStr("1.2_3")],
            "Invalid buildpack version: `1.2_3`",
        );
    }

    #[test]
    fn reject_version_suffixes() {
        // These are valid semver, but not a valid buildpack version.
        assert_de_tokens_error::<BuildpackVersion>(
            &[Token::BorrowedStr("1.2.3-dev")],
            "Invalid buildpack version: `1.2.3-dev`",
        );
        assert_de_tokens_error::<BuildpackVersion>(
            &[Token::BorrowedStr("1.2.3+abc")],
            "Invalid buildpack version: `1.2.3+abc`",
        );
    }

    #[test]
    fn reject_negative_versions() {
        assert_de_tokens_error::<BuildpackVersion>(
            &[Token::BorrowedStr("-1.2.3")],
            "Invalid buildpack version: `-1.2.3`",
        );
        assert_de_tokens_error::<BuildpackVersion>(
            &[Token::BorrowedStr("1.-2.3")],
            "Invalid buildpack version: `1.-2.3`",
        );
        assert_de_tokens_error::<BuildpackVersion>(
            &[Token::BorrowedStr("1.2.-3")],
            "Invalid buildpack version: `1.2.-3`",
        );
    }

    #[test]
    fn reject_versions_with_leading_zeros() {
        assert_de_tokens_error::<BuildpackVersion>(
            &[Token::BorrowedStr("01.2.3")],
            "Invalid buildpack version: `01.2.3`",
        );
        assert_de_tokens_error::<BuildpackVersion>(
            &[Token::BorrowedStr("1.00.3")],
            "Invalid buildpack version: `1.00.3`",
        );
        assert_de_tokens_error::<BuildpackVersion>(
            &[Token::BorrowedStr("1.2.030")],
            "Invalid buildpack version: `1.2.030`",
        );
    }

    #[test]
    fn buildpack_version_display() {
        assert_eq!(
            BuildpackVersion {
                major: 0,
                minor: 1,
                patch: 2,
            }
            .to_string(),
            "0.1.2"
        );
        assert_eq!(
            BuildpackVersion {
                major: 2000,
                minor: 10,
                patch: 20,
            }
            .to_string(),
            "2000.10.20"
        );
    }
}
