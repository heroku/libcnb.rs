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
/// let toml_str = r#"
/// api = "0.6"
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
/// let result = toml::from_str::<BuildpackToml<toml::value::Table>>(toml_str);
/// assert!(result.is_ok());
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
    pub name: Option<String>,
    // MUST be in the form <X>.<Y>.<Z> where X, Y, and Z are non-negative integers and must not contain leading zeroes
    pub version: Version,
    pub homepage: Option<String>,
    #[serde(default, rename = "clear-env")]
    pub clear_env: bool,
    pub description: Option<String>,
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub keywords: Vec<String>,
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub licenses: Vec<License>,
}

#[derive(Deserialize, Debug, Eq, PartialEq)]
pub struct License {
    pub r#type: Option<String>,
    pub uri: Option<String>,
}

#[derive(Deserialize, Debug, Eq, PartialEq)]
pub struct Order {
    pub group: Vec<Group>,
}

#[derive(Deserialize, Debug, Eq, PartialEq)]
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
    fn deserialize_buildpack() {
        let toml_str = r#"
api = "0.6"

[buildpack]
id = "foo/bar"
name = "Bar Buildpack"
version = "0.0.1"
homepage = "https://example.tld"
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

[[stacks]]
id = "heroku-20"

[[stacks]]
id = "io.buildpacks.stacks.bionic"
mixins = []

[[stacks]]
id = "io.buildpacks.stacks.focal"
mixins = ["build:jq", "wget"]

# As counter-intuitive as it may seem, the CNB spec permits specifying
# the "any" stack at the same time as stacks with specific IDs.
[[stacks]]
id = "*"

[metadata]
checksum = "abc123"
        "#;

        let buildpack_toml = toml::from_str::<GenericBuildpackToml>(toml_str).unwrap();

        assert_eq!(buildpack_toml.api, BuildpackApi { major: 0, minor: 6 });
        assert_eq!(buildpack_toml.buildpack.id, "foo/bar".parse().unwrap());
        assert_eq!(
            buildpack_toml.buildpack.name,
            Some(String::from("Bar Buildpack"))
        );
        assert_eq!(buildpack_toml.buildpack.version, Version::new(0, 0, 1));
        assert_eq!(
            buildpack_toml.buildpack.homepage,
            Some(String::from("https://example.tld"))
        );
        assert!(buildpack_toml.buildpack.clear_env);
        assert_eq!(
            buildpack_toml.buildpack.description,
            Some(String::from("A buildpack for Foo Bar"))
        );
        assert_eq!(
            buildpack_toml.buildpack.keywords,
            vec![String::from("foo"), String::from("bar")]
        );
        assert_eq!(
            buildpack_toml.buildpack.licenses,
            vec![
                License {
                    r#type: Some(String::from("BSD-3-Clause")),
                    uri: None
                },
                License {
                    r#type: Some(String::from("Custom license with type and URI")),
                    uri: Some(String::from("https://example.tld/my-license"))
                },
                License {
                    r#type: None,
                    uri: Some(String::from("https://example.tld/my-license"))
                }
            ]
        );
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
                    mixins: vec![String::from("build:jq"), String::from("wget")]
                },
                Stack::Any
            ]
        );
        assert_eq!(buildpack_toml.order, Vec::new());
        assert_eq!(
            buildpack_toml.metadata.unwrap().get("checksum"),
            Some(&toml::value::Value::try_from("abc123").unwrap())
        );
    }

    #[test]
    fn deserialize_metabuildpack() {
        let toml_str = r#"
api = "0.6"

[buildpack]
id = "foo/bar"
name = "Bar Buildpack"
version = "0.0.1"
homepage = "https://example.tld"
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

# This is invalid according to the spec, however libcnb currently requires it:
# https://github.com/Malax/libcnb.rs/issues/211
[[stacks]]
id = "*"

[[order]]

[[order.group]]
id = "foo/bar"
version = "0.0.1"

[[order.group]]
id = "foo/baz"
version = "0.1.0"
optional = true

[metadata]
checksum = "abc123"
        "#;

        let buildpack_toml = toml::from_str::<GenericBuildpackToml>(toml_str).unwrap();

        assert_eq!(buildpack_toml.api, BuildpackApi { major: 0, minor: 6 });
        assert_eq!(buildpack_toml.buildpack.id, "foo/bar".parse().unwrap());
        assert_eq!(
            buildpack_toml.buildpack.name,
            Some(String::from("Bar Buildpack"))
        );
        assert_eq!(buildpack_toml.buildpack.version, Version::new(0, 0, 1));
        assert_eq!(
            buildpack_toml.buildpack.homepage,
            Some(String::from("https://example.tld"))
        );
        assert!(buildpack_toml.buildpack.clear_env);
        assert_eq!(
            buildpack_toml.buildpack.description,
            Some(String::from("A buildpack for Foo Bar"))
        );
        assert_eq!(
            buildpack_toml.buildpack.keywords,
            vec![String::from("foo"), String::from("bar")]
        );
        assert_eq!(
            buildpack_toml.buildpack.licenses,
            vec![
                License {
                    r#type: Some(String::from("BSD-3-Clause")),
                    uri: None
                },
                License {
                    r#type: Some(String::from("Custom license with type and URI")),
                    uri: Some(String::from("https://example.tld/my-license"))
                },
                License {
                    r#type: None,
                    uri: Some(String::from("https://example.tld/my-license"))
                }
            ]
        );
        // This is invalid according to the spec, however libcnb currently requires it:
        // https://github.com/Malax/libcnb.rs/issues/211
        assert_eq!(buildpack_toml.stacks, vec![Stack::Any]);
        assert_eq!(
            buildpack_toml.order,
            vec![Order {
                group: vec![
                    Group {
                        id: "foo/bar".parse().unwrap(),
                        version: Version::new(0, 0, 1),
                        optional: false
                    },
                    Group {
                        id: "foo/baz".parse().unwrap(),
                        version: Version::new(0, 1, 0),
                        optional: true
                    }
                ]
            }]
        );
        assert_eq!(
            buildpack_toml.metadata.unwrap().get("checksum"),
            Some(&toml::value::Value::try_from("abc123").unwrap())
        );
    }

    #[test]
    fn deserialize_minimal_buildpack() {
        let toml_str = r#"
api = "0.6"

[buildpack]
id = "foo/bar"
name = "Bar Buildpack"
version = "0.0.1"

[[stacks]]
id = "*"
        "#;

        let buildpack_toml = toml::from_str::<GenericBuildpackToml>(toml_str).unwrap();

        assert_eq!(buildpack_toml.api, BuildpackApi { major: 0, minor: 6 });
        assert_eq!(buildpack_toml.buildpack.id, "foo/bar".parse().unwrap());
        assert_eq!(
            buildpack_toml.buildpack.name,
            Some(String::from("Bar Buildpack"))
        );
        assert_eq!(buildpack_toml.buildpack.version, Version::new(0, 0, 1));
        assert_eq!(buildpack_toml.buildpack.homepage, None);
        assert!(!buildpack_toml.buildpack.clear_env);
        assert_eq!(buildpack_toml.buildpack.description, None);
        assert_eq!(buildpack_toml.buildpack.keywords, Vec::<String>::new());
        assert_eq!(buildpack_toml.buildpack.licenses, Vec::new());
        assert_eq!(buildpack_toml.stacks, vec![Stack::Any]);
        assert_eq!(buildpack_toml.order, Vec::new());
        assert_eq!(buildpack_toml.metadata, None);
    }

    #[test]
    fn deserialize_minimal_metabuildpack() {
        let toml_str = r#"
api = "0.6"

[buildpack]
id = "foo/bar"
name = "Bar Buildpack"
version = "0.0.1"

# This is invalid according to the spec, however libcnb currently requires it:
# https://github.com/Malax/libcnb.rs/issues/211
[[stacks]]
id = "*"

[[order]]

[[order.group]]
id = "foo/bar"
version = "0.0.1"
"#;

        let buildpack_toml = toml::from_str::<GenericBuildpackToml>(toml_str).unwrap();

        assert_eq!(buildpack_toml.api, BuildpackApi { major: 0, minor: 6 });
        assert_eq!(buildpack_toml.buildpack.id, "foo/bar".parse().unwrap());
        assert_eq!(
            buildpack_toml.buildpack.name,
            Some(String::from("Bar Buildpack"))
        );
        assert_eq!(buildpack_toml.buildpack.version, Version::new(0, 0, 1));
        assert_eq!(buildpack_toml.buildpack.homepage, None);
        assert!(!buildpack_toml.buildpack.clear_env);
        assert_eq!(buildpack_toml.buildpack.description, None);
        assert_eq!(buildpack_toml.buildpack.keywords, Vec::<String>::new());
        assert_eq!(buildpack_toml.buildpack.licenses, Vec::new());
        // This is invalid according to the spec, however libcnb currently requires it:
        // https://github.com/Malax/libcnb.rs/issues/211
        assert_eq!(buildpack_toml.stacks, vec![Stack::Any]);
        assert_eq!(
            buildpack_toml.order,
            vec![Order {
                group: vec![Group {
                    id: "foo/bar".parse().unwrap(),
                    version: Version::new(0, 0, 1),
                    optional: false
                }]
            }]
        );
        assert_eq!(buildpack_toml.metadata, None);
    }

    #[test]
    fn reject_invalid_buildpack_version() {
        let toml_str = r#"
api = "0.6"

[buildpack]
id = "foo/bar"
name = "Bar Buildpack"
version = "1.0"

[[stacks]]
id = "*"
        "#;

        let err = toml::from_str::<GenericBuildpackToml>(toml_str).unwrap_err();
        assert!(err
            .to_string()
            .contains("unexpected end of input while parsing minor version number for key `buildpack.version`"));
    }
}
