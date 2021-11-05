use lazy_static::lazy_static;
use regex::Regex;
use serde::Deserialize;
use std::borrow::Borrow;
use std::ops::Deref;
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
/// // StackId implements various traits to improve interoperability. We can, for example, call
/// // methods of String on StackId or pass it to functions that expect a String/str:
/// let stack_id = "heroku-20".parse::<StackId>().unwrap();
///
/// assert_eq!("HEROKU-20", stack_id.to_uppercase());
///
/// fn length(s: &str) -> usize {
///     s.len()
/// }
///
/// assert_eq!(9, length(&stack_id));
/// ```
#[derive(Deserialize, Debug)]
pub struct StackId(String);

impl StackId {
    pub fn as_str(&self) -> &str {
        &self.0
    }
}

impl Borrow<String> for StackId {
    fn borrow(&self) -> &String {
        &self.0
    }
}

impl Deref for StackId {
    type Target = String;

    fn deref(&self) -> &Self::Target {
        &self.0
    }
}

impl AsRef<String> for StackId {
    fn as_ref(&self) -> &String {
        &self.0
    }
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

#[derive(thiserror::Error, Debug)]
pub enum StackIdError {
    #[error(
    "Found `{0}` but value MUST only contain numbers, letters, and the characters `.`, `/`, and `-`. or only `*`"
    )]
    InvalidStackId(String),
}
