use serde::{Deserialize, Serialize};
use toml;

pub type Bom = Vec<Entry>;

#[derive(Deserialize, Serialize, Debug)]
pub struct Entry {
    pub name: String,
    pub metadata: toml::value::Table,
}
