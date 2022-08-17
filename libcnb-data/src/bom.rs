use serde::{Deserialize, Serialize};
use toml;

pub type Bom = Vec<Entry>;

#[derive(Deserialize, Serialize, Clone, Debug)]
#[serde(deny_unknown_fields)]
pub struct Entry {
    pub name: String,
    pub metadata: toml::value::Table,
}
