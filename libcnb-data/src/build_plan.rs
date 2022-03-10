use serde::Serialize;
use std::collections::VecDeque;
use toml::value::Table;

#[derive(Serialize, Debug, Default)]
#[must_use]
pub struct BuildPlan {
    #[serde(skip_serializing_if = "Vec::is_empty")]
    pub provides: Vec<Provide>,
    #[serde(skip_serializing_if = "Vec::is_empty")]
    pub requires: Vec<Require>,
    #[serde(skip_serializing_if = "Vec::is_empty")]
    pub or: Vec<Or>,
}

impl BuildPlan {
    pub fn new() -> Self {
        Self::default()
    }
}

#[derive(Default)]
#[must_use]
pub struct BuildPlanBuilder {
    acc: VecDeque<(Vec<Provide>, Vec<Require>)>,
    current_provides: Vec<Provide>,
    current_requires: Vec<Require>,
}

impl BuildPlanBuilder {
    pub fn new() -> Self {
        Self::default()
    }

    pub fn provides(mut self, name: impl AsRef<str>) -> Self {
        self.current_provides.push(Provide::new(name.as_ref()));
        self
    }

    pub fn requires(mut self, name: impl AsRef<str>) -> Self {
        self.current_requires.push(Require::new(name.as_ref()));
        self
    }

    /// Add Require to the BuildPlan
    pub fn requires_with_struct(mut self, require: Require) -> Self {
        self.current_requires.push(require);
        self
    }

    pub fn or(mut self) -> Self {
        self.acc
            .push_back((self.current_provides, self.current_requires));
        self.current_provides = vec![];
        self.current_requires = vec![];

        self
    }

    pub fn build(self) -> BuildPlan {
        let mut xyz = self.or();

        if let Some(head) = xyz.acc.pop_front() {
            let mut build_plan = BuildPlan::new();
            build_plan.provides = head.0;
            build_plan.requires = head.1;

            for alternative in xyz.acc {
                build_plan.or.push(Or {
                    provides: alternative.0,
                    requires: alternative.1,
                });
            }

            build_plan
        } else {
            BuildPlan::new()
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
        Self { name: name.into() }
    }
}

#[derive(Serialize, Debug)]
pub struct Require {
    pub name: String,
    pub metadata: Table,
}

impl Require {
    pub fn new(name: impl Into<String>) -> Self {
        Self {
            name: name.into(),
            metadata: Table::new(),
        }
    }

    /// Convert a Serializable struct and store it as a toml Table for metadata
    pub fn metadata<T: Serialize>(&mut self, metadata: T) -> Result<(), toml::ser::Error> {
        if let toml::Value::Table(table) = toml::Value::try_from(metadata)? {
            self.metadata = table;

            Ok(())
        } else {
            Err(toml::ser::Error::Custom(
                "Could not be serialized as a map.".to_string(),
            ))
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

    #[test]
    fn it_serializes_metadata() {
        #[derive(Serialize)]
        struct Metadata {
            foo: String,
        }

        let mut require = Require::new("foo");
        let metadata = Metadata {
            foo: "bar".to_string(),
        };
        let result = require.metadata(metadata);
        assert!(result.is_ok());
        assert_eq!(
            require.metadata.get("foo"),
            Some(&toml::Value::String("bar".to_string()))
        )
    }
}
