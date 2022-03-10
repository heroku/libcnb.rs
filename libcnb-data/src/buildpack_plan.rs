use serde::Deserialize;
use toml::value::Table;

#[derive(Debug, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct BuildpackPlan {
    #[serde(default)]
    pub entries: Vec<Entry>,
}

#[derive(Debug, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Entry {
    pub name: String,
    #[serde(default)]
    pub metadata: Table,
}

impl Entry {
    /// Deserializes Metadata to a type T that implements Deserialize
    pub fn metadata<'de, T>(&self) -> Result<T, toml::de::Error>
    where
        T: Deserialize<'de>,
    {
        // serde::de::Deserializer is not implemented for toml::map::Map, so need to clone() here
        toml::Value::Table(self.metadata.clone()).try_into()
    }
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

    #[test]
    fn it_deserializes_metadata() {
        #[derive(Deserialize)]
        struct Metadata {
            foo: String,
        }

        let mut metadata = Table::new();
        metadata.insert("foo".to_string(), toml::Value::String("bar".to_string()));
        let entry = Entry {
            name: "foo".to_string(),
            metadata,
        };

        let result = entry.metadata::<Metadata>();
        assert!(result.is_ok());
        let metadata = result.unwrap();
        assert_eq!(metadata.foo, "bar".to_string());
    }
}
