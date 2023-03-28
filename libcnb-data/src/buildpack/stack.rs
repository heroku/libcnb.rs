use super::{StackId, StackIdError};
use serde::Deserialize;

// Used as a "shadow" struct to store
// potentially invalid `Stack` data when deserializing
// https://dev.to/equalma/validate-fields-and-types-in-serde-with-tryfrom-c2n
#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
struct StackUnchecked {
    pub id: String,
    #[serde(default)]
    pub mixins: Vec<String>,
}

#[derive(Deserialize, Debug, Eq, PartialEq, Clone)]
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
                Ok(Self::Any)
            } else {
                Err(Self::Error::InvalidAnyStack(mixins))
            }
        } else {
            Ok(Self::Specific {
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

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn deserialize_specific_stack_without_mixins() {
        let toml_str = r#"
id = "heroku-20"
"#;
        assert_eq!(
            toml::from_str::<Stack>(toml_str),
            Ok(Stack::Specific {
                // Cannot use the `stack_id!` macro due to: https://github.com/heroku/libcnb.rs/issues/179
                id: "heroku-20".parse().unwrap(),
                mixins: Vec::new()
            }),
        );
    }

    #[test]
    fn deserialize_specific_stack_with_mixins() {
        let toml_str = r#"
id = "io.buildpacks.stacks.focal"
mixins = ["build:jq", "wget"]
"#;
        assert_eq!(
            toml::from_str::<Stack>(toml_str),
            Ok(Stack::Specific {
                id: "io.buildpacks.stacks.focal".parse().unwrap(),
                mixins: vec![String::from("build:jq"), String::from("wget")]
            }),
        );
    }

    #[test]
    fn deserialize_any_stack() {
        let toml_str = r#"
id = "*"
"#;
        assert_eq!(toml::from_str::<Stack>(toml_str), Ok(Stack::Any));
    }

    #[test]
    fn reject_specific_stack_with_invalid_name() {
        let toml_str = r#"
id = "io.buildpacks.stacks.*"
"#;
        let err = toml::from_str::<Stack>(toml_str).unwrap_err();
        assert!(err
            .to_string()
            .contains("Invalid Stack ID: Invalid Value: io.buildpacks.stacks.*"));
    }

    #[test]
    fn reject_any_stack_with_mixins() {
        let toml_str = r#"
id = "*"
mixins = ["build:jq", "wget"]
"#;
        let err = toml::from_str::<Stack>(toml_str).unwrap_err();
        assert!(err
                .to_string()
                .contains("Stack with id `*` MUST NOT contain mixins, however the following mixins were specified: `build:jq`, `wget`"));
    }
}
