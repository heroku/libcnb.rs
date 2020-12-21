use serde::Serialize;
use toml;

pub type Bom = Vec<Entry>;

#[derive(Serialize, Debug)]
pub struct Entry {
    pub name: String,
    pub metadata: toml::value::Table,
}
