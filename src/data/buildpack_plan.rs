use serde::Deserialize;
use toml::value::Table;

#[derive(Debug, Deserialize)]
pub struct BuildpackPlan {
    pub entries: Vec<Entry>,
}

#[derive(Debug, Deserialize)]
pub struct Entry {
    pub name: String,
    pub metadata: Table,
}
