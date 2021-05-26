use crate::data::defaults;
use serde::{Deserialize, Serialize};
use toml::value::Table;

#[derive(Debug, Deserialize, Serialize)]
pub struct Layer {
    #[serde(default = "defaults::r#false")]
    pub launch: bool,
    #[serde(default = "defaults::r#false")]
    pub build: bool,
    #[serde(default = "defaults::r#false")]
    pub cache: bool,
    #[serde(default)]
    pub metadata: Table,
}

impl Layer {
    pub fn new() -> Self {
        Layer {
            launch: false,
            build: false,
            cache: false,
            metadata: Table::new(),
        }
    }

    /// Reset flags to false and empty metadata table.
    pub fn clear(&mut self) {
        self.launch = false;
        self.build = false;
        self.cache = false;
        self.metadata.clear();
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn metadata_is_optional() {
        let layer: Result<Layer, toml::de::Error> = toml::from_str(
            r#"
            launch = true
            build = true
            cache = false
            "#,
        );

        assert!(!layer.is_err());
    }
}
