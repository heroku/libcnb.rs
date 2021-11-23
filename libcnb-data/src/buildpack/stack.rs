use serde::Deserialize;

use super::{StackId, StackIdError};

// Used as a "shadow" struct to store
// potentially invalid `Stack` data when deserializing
// https://dev.to/equalma/validate-fields-and-types-in-serde-with-tryfrom-c2n
#[derive(Deserialize)]
struct StackUnchecked {
    pub id: String,
    #[serde(default)]
    pub mixins: Vec<String>,
}

#[derive(Deserialize, Debug, Eq, PartialEq)]
#[serde(try_from = "StackUnchecked")]
pub enum Stack {
    Any,
    Specific { id: StackId, mixins: Vec<String> },
}

impl TryFrom<StackUnchecked> for Stack {
    type Error = StackError;

    fn try_from(value: StackUnchecked) -> Result<Self, Self::Error> {
        let StackUnchecked { id, mixins } = value;

        if id.as_str() == "*" {
            if mixins.is_empty() {
                Ok(Stack::Any)
            } else {
                Err(Self::Error::InvalidAnyStack(mixins))
            }
        } else {
            Ok(Stack::Specific {
                id: id.parse()?,
                mixins,
            })
        }
    }
}

#[derive(thiserror::Error, Debug)]
pub enum StackError {
    #[error("Stack with id `*` MUST NOT contain mixins, however the following mixins were specified: `{}`", .0.join("`, `"))]
    InvalidAnyStack(Vec<String>),

    #[error("Invalid Stack ID: {0}")]
    InvalidStackId(#[from] StackIdError),
}
