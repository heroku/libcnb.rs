mod api;
mod id;
mod stack;
mod stack_id;

pub use api::*;
pub use id::*;
pub use stack::*;
pub use stack_id::*;

use semver::Version;
use serde::Deserialize;

/// Data structure for the Buildpack descriptor (buildpack.toml).
///
/// # Examples
/// ```
/// use libcnb_data::buildpack::BuildpackToml;
///
///         let raw = r#"
/// api = "0.4"
///
/// [buildpack]
/// id = "foo/bar"
/// name = "Bar Buildpack"
/// version = "0.0.1"
/// homepage = "https://www.foo.com/bar"
/// clear-env = false
/// description = "A buildpack for Foo Bar"
/// keywords = ["foo"]
///
/// [[buildpack.licenses]]
/// type = "BSD-3-Clause"
///
/// [[stacks]]
/// id = "io.buildpacks.stacks.bionic"
/// mixins = ["yj", "yq"]
///
/// [metadata]
/// checksum = "awesome"
/// "#;
///
///         let result = toml::from_str::<BuildpackToml<toml::value::Table>>(raw);
///         assert!(result.is_ok());
/// ```
#[derive(Deserialize, Debug)]
pub struct BuildpackToml<BM> {
    pub api: BuildpackApi,
    pub buildpack: Buildpack,
    pub stacks: Vec<Stack>,
    #[serde(default)]
    pub order: Vec<Order>,
    pub metadata: BM,
}

#[derive(Deserialize, Debug)]
pub struct Buildpack {
    pub id: BuildpackId,
    pub name: String,
    // MUST be in the form <X>.<Y>.<Z> where X, Y, and Z are non-negative integers and must not contain leading zeroes
    pub version: Version,
    pub homepage: Option<String>,
    #[serde(rename = "clear-env")]
    #[serde(default)]
    pub clear_env: bool,
    pub description: Option<String>,
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub keywords: Vec<String>,
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub licenses: Vec<License>,
}

#[derive(Deserialize, Debug)]
pub struct License {
    pub r#type: Option<String>,
    pub uri: Option<String>,
}

#[derive(Deserialize, Debug)]
pub struct Order {
    group: Vec<Group>,
}

#[derive(Deserialize, Debug)]
pub struct Group {
    pub id: BuildpackId,
    pub version: Version,
    #[serde(default)]
    pub optional: bool,
}

#[cfg(test)]
mod tests {
    use super::*;

    type GenericBuildpackToml = BuildpackToml<Option<toml::value::Table>>;

    #[test]
    fn can_deserialize_metabuildpack() {
        let raw = r#"
api = "0.4"

[buildpack]
id = "foo/bar"
name = "Bar Buildpack"
version = "0.0.1"
homepage = "https://www.foo.com/bar"
clear-env = true
description = "A buildpack for Foo Bar"
keywords = ["foo", "bar"]

[[buildpack.licenses]]
type = "BSD-3-Clause"

[[buildpack.licenses]]
type = "Custom license with type and URI"
uri = "https://example.tld/my-license"

[[buildpack.licenses]]
uri = "https://example.tld/my-license"

[[order]]
[[order.group]]
id = "foo/baz"
version = "0.0.2"
optional = false

[[stacks]]
id = "io.buildpacks.stacks.bionic"
mixins = ["yj", "yq"]

[metadata]
checksum = "awesome"
"#;

        let result = toml::from_str::<BuildpackToml<toml::value::Table>>(raw);
        assert!(result.is_ok());
        if let Ok(toml) = result {
            assert_eq!(
                toml.buildpack.description.unwrap(),
                "A buildpack for Foo Bar"
            );
            assert_eq!(toml.buildpack.keywords.len(), 2);
            assert_eq!(toml.buildpack.licenses.len(), 3);
        }
    }

    #[test]
    fn can_deserialize_minimal_buildpack() {
        let raw = r#"
api = "0.4"

[buildpack]
id = "foo/bar"
name = "Bar Buildpack"
version = "0.0.1"

[[stacks]]
id = "io.buildpacks.stacks.bionic"

[metadata]
checksum = "awesome"
"#;

        let result = toml::from_str::<BuildpackToml<toml::value::Table>>(raw);
        assert!(result.is_ok());
        if let Ok(toml) = result {
            assert!(!toml.buildpack.clear_env);
        }
    }

    #[test]
    fn can_deserialize_minimal_metabuildpack() {
        let raw = r#"
api = "0.4"

[buildpack]
id = "foo/bar"
name = "Bar Buildpack"
version = "0.0.1"

[[order]]
[[order.group]]
id = "foo/baz"
version = "0.0.2"

[[stacks]]
id = "io.buildpacks.stacks.bionic"
"#;

        let result = toml::from_str::<BuildpackToml<Option<toml::value::Table>>>(raw);
        assert!(result.is_ok());
        if let Ok(toml) = result {
            assert!(!toml.order.get(0).unwrap().group.get(0).unwrap().optional);
        }
    }

    #[test]
    fn stacks_valid() {
        let raw = r#"
api = "0.6"

[buildpack]
id = "foo/bar"
name = "Bar Buildpack"
version = "0.0.1"

[[stacks]]
id = "heroku-20"

[[stacks]]
id = "io.buildpacks.stacks.bionic"
mixins = []

[[stacks]]
id = "io.buildpacks.stacks.focal"
mixins = ["yj", "yq"]

# As counter-intuitive as it may seem, the CNB spec permits specifying
# the "any" stack at the same time as stacks with specific IDs.
[[stacks]]
id = "*"
"#;

        let buildpack_toml = toml::from_str::<GenericBuildpackToml>(raw).unwrap();
        assert_eq!(
            buildpack_toml.stacks,
            vec![
                Stack::Specific {
                    // Cannot use the `stack_id!` macro due to: https://github.com/Malax/libcnb.rs/issues/179
                    id: "heroku-20".parse().unwrap(),
                    mixins: Vec::new()
                },
                Stack::Specific {
                    id: "io.buildpacks.stacks.bionic".parse().unwrap(),
                    mixins: Vec::new()
                },
                Stack::Specific {
                    id: "io.buildpacks.stacks.focal".parse().unwrap(),
                    mixins: vec![String::from("yj"), String::from("yq")]
                },
                Stack::Any
            ]
        );
    }
}
