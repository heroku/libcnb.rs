use serde::Deserialize;
use toml::value::Table;

#[derive(Deserialize, Debug)]
pub struct BuildPlan {
    provides: Vec<Provide>,
    requires: Vec<Require>,
    or: Vec<Or>,
}

#[derive(Deserialize, Debug)]
pub struct Or {
    provides: Vec<Provide>,
    requires: Vec<Require>,
}

#[derive(Deserialize, Debug)]
pub struct Provide {
    name: String,
}

#[derive(Deserialize, Debug)]
pub struct Require {
    name: String,
    metadata: Table,
}
