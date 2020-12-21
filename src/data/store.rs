use serde::{Deserialize, Serialize};
use toml::value::Table;

#[derive(Debug, Deserialize, Serialize)]
pub struct Store {
    pub metadata: Table,
}
