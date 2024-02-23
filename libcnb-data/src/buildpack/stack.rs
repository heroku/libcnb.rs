use serde::Deserialize;

// Stacks are deprecated in Buildpack API 0.10, and libcnb.rs effectively
// ignores them. However, they are still supported by the Buildpack API, so
// libcnb should continue to allow them to exist in buildpack.toml.
#[derive(Deserialize, Debug, Eq, PartialEq)]
#[serde(deny_unknown_fields)]
pub struct Stack {
    pub id: String,
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub mixins: Vec<String>,
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
            Ok(Stack {
                id: String::from("heroku-20"),
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
            Ok(Stack {
                id: String::from("io.buildpacks.stacks.focal"),
                mixins: vec![String::from("build:jq"), String::from("wget")]
            }),
        );
    }

    #[test]
    fn deserialize_any_stack() {
        let toml_str = r#"
id = "*"
"#;
        assert_eq!(
            toml::from_str::<Stack>(toml_str),
            Ok(Stack {
                id: String::from("*"),
                mixins: vec![],
            }),
        );
    }
}
