use serde::Serialize;
use toml::value::Table;

#[derive(Serialize, Debug)]
pub struct BuildPlan {
    provides: Vec<Provide>,
    requires: Vec<Require>,
    or: Vec<Or>,
}

impl BuildPlan {
    pub fn new() -> BuildPlan {
        BuildPlan {
            provides: vec![],
            requires: vec![],
            or: vec![],
        }
    }
}

#[derive(Serialize, Debug)]
pub struct Or {
    provides: Vec<Provide>,
    requires: Vec<Require>,
}

#[derive(Serialize, Debug)]
pub struct Provide {
    name: String,
}

#[derive(Serialize, Debug)]
pub struct Require {
    name: String,
    metadata: Table,
}
