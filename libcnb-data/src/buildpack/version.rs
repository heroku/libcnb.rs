use serde::Deserialize;
use std::fmt;
use std::fmt::{Display, Formatter};

/// The Buildpack version.
///
/// This MUST be in the form `<X>.<Y>.<Z>` where `X`, `Y`, and `Z` are non-negative integers
/// and must not contain leading zeros.
#[derive(Deserialize, Debug, Eq, PartialEq)]
#[serde(try_from = "String")]
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

impl TryFrom<String> for BuildpackVersion {
    type Error = BuildpackVersionError;

    fn try_from(value: String) -> Result<Self, Self::Error> {
        // We're not using the `semver` crate, since semver versions also permit pre-release and
        // build metadata suffixes, which are not valid in buildpack versions.
        match value
            .split('.')
            .map(|s| {
                // The spec forbids redundant leading zeros.
                if s.starts_with('0') && s != "0" {
                    None
                } else {
                    s.parse().ok()
                }
            })
            .collect::<Option<Vec<_>>>()
            .unwrap_or_default()
            .as_slice()
        {
            &[major, minor, patch] => Ok(Self::new(major, minor, patch)),
            _ => Err(Self::Error::InvalidBuildpackVersion(value)),
        }
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
