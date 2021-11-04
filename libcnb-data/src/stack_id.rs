use lazy_static::lazy_static;
use regex::Regex;
use serde::Deserialize;
use std::str::FromStr;

/// Stack Id. This is a newtype wrapper around a String.
/// It MUST only contain numbers, letters, and the characters ., /, and -.
/// or be `*`.
///
/// Use [`std::str::FromStr`] to create a new instance of this struct.
///
/// # Examples
/// ```
/// use std::str::FromStr;
/// use libcnb_data::stack_id::StackId;
///
/// let valid = StackId::from_str("io.buildpacks.bionic/Latest-2020");
/// assert_eq!(valid.unwrap().as_str(), "io.buildpacks.bionic/Latest-2020");
///
/// let invalid = StackId::from_str("!nvalid");
/// assert!(invalid.is_err());
///
/// let valid = StackId::from_str("*");
/// assert!(valid.is_ok());
/// ```
#[derive(Deserialize, Debug)]
pub struct StackId(String);

#[derive(thiserror::Error, Debug)]
pub enum StackIdError {
    #[error(
    "Found `{0}` but value MUST only contain numbers, letters, and the characters `.`, `/`, and `-`. or only `*`"
    )]
    InvalidStackId(String),
}

impl FromStr for StackId {
    type Err = StackIdError;

    fn from_str(value: &str) -> Result<Self, Self::Err> {
        lazy_static! {
            static ref RE: Regex = Regex::new(r"^([[:alnum:]./-]+|\*)$").unwrap();
        }

        let string = String::from(value);
        if RE.is_match(value) {
            Ok(Self(string))
        } else {
            Err(StackIdError::InvalidStackId(string))
        }
    }
}

impl From<StackId> for String {
    fn from(stack_id: StackId) -> Self {
        stack_id.0
    }
}

impl StackId {
    pub fn as_str(&self) -> &str {
        &self.0
    }
}
