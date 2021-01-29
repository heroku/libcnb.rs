use serde::Serialize;
use toml::value::Table;

#[derive(Serialize, Debug)]
pub struct BuildPlan {
    #[serde(skip_serializing_if = "Vec::is_empty")]
    pub provides: Vec<Provide>,
    #[serde(skip_serializing_if = "Vec::is_empty")]
    pub requires: Vec<Require>,
    #[serde(skip_serializing_if = "Vec::is_empty")]
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
    #[serde(skip_serializing_if = "Vec::is_empty")]
    provides: Vec<Provide>,
    #[serde(skip_serializing_if = "Vec::is_empty")]
    requires: Vec<Require>,
}

#[derive(Serialize, Debug)]
pub struct Provide {
    name: String,
}

impl Provide {
    pub fn new(name: impl Into<String>) -> Self {
        Provide { name: name.into() }
    }
}

#[derive(Serialize, Debug)]
pub struct Require {
    name: String,
    metadata: Table,
}

impl Require {
    pub fn new(name: impl Into<String>) -> Self {
        Require {
            name: name.into(),
            metadata: Table::new(),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn it_writes_simple_build_plan() {
        let mut build_plan = BuildPlan::new();
        build_plan.provides.push(Provide::new("rust"));
        build_plan.requires.push(Require::new("rust"));

        assert!(toml::to_string(&build_plan).is_ok());
    }
}
