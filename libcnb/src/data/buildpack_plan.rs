use serde::Deserialize;
use toml::value::Table;

#[derive(Debug, Deserialize)]
pub struct BuildpackPlan {
    #[serde(default)]
    pub entries: Vec<Entry>,
}

#[derive(Debug, Deserialize)]
pub struct Entry {
    pub name: String,
    #[serde(default)]
    pub metadata: Table,
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn it_parses_empty() {
        let raw = "";

        let result = toml::from_str::<BuildpackPlan>(raw);
        assert!(result.is_ok());
    }

    #[test]
    fn it_parses_simple() {
        let toml = r#"
[[entries]]
name = "rust"
"#;
        let result = toml::from_str::<BuildpackPlan>(toml);
        assert!(result.is_ok());
    }

    #[test]
    fn it_parses_with_metadata() {
        let toml = r#"
[[entries]]
name = "rust"
    [entries.metadata]
    version = "1.39"
"#;

        let result = toml::from_str::<BuildpackPlan>(toml);
        assert!(result.is_ok());
    }
}
