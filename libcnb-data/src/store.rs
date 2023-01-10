use serde::{Deserialize, Serialize};
use toml::value::Table;

#[derive(Clone, Default, Debug, Deserialize, Serialize)]
#[serde(deny_unknown_fields)]
pub struct Store {
    pub metadata: Table,
}
