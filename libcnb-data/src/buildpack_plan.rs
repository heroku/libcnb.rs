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
    ///
    /// # Errors
    /// This will return an error if it's not possible to serialize from a TOML Table into a T
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
        #[derive(Deserialize, Eq, PartialEq, Debug)]
        struct Metadata {
            foo: String,
        }

        let mut metadata = Table::new();
        metadata.insert(
            String::from("foo"),
            toml::Value::String(String::from("bar")),
        );
        let entry = Entry {
            name: String::from("foo"),
            metadata,
        };

        assert_eq!(
            entry.metadata(),
            Ok(Metadata {
                foo: String::from("bar"),
            })
        );
    }
}
