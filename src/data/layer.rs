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
    pub metadata: Table,
}
