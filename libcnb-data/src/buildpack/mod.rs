mod api;
mod id;
mod stack;
mod stack_id;
mod version;

use crate::generic::GenericMetadata;
use crate::sbom::SbomFormat;
pub use api::*;
pub use id::*;
use serde::Deserialize;
pub use stack::*;
pub use stack_id::*;
use std::collections::HashSet;
pub use version::*;

/// Data structures for the Buildpack descriptor (buildpack.toml).
///
/// For parsing of [buildpack.toml](https://github.com/buildpacks/spec/blob/main/buildpack.md#buildpacktoml-toml)
/// files when support for multiple types of buildpack is required.
///
/// When a specific buildpack type is expected, use [`ComponentBuildpackDescriptor`] or
/// [`CompositeBuildpackDescriptor`] directly instead, since they allow for more detailed
/// error messages if parsing fails.
///
/// # Example:
/// ```
/// use libcnb_data::buildpack::BuildpackDescriptor;
///
/// let toml_str = r#"
/// api = "0.9"
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
/// id = "*"
/// "#;
///
/// let buildpack_descriptor =
///     toml::from_str::<BuildpackDescriptor>(toml_str)
///         .expect("buildpack.toml did not match a known type!");
/// match buildpack_descriptor {
///     BuildpackDescriptor::Component(buildpack) => {
///         println!("Found component buildpack: {}", buildpack.buildpack.id);
///     }
///     BuildpackDescriptor::Composite(buildpack) => {
///         println!("Found composite buildpack: {}", buildpack.buildpack.id);
///     }
/// };
/// ```
#[derive(Deserialize, Debug)]
#[serde(untagged)]
pub enum BuildpackDescriptor<BM = GenericMetadata> {
    Component(ComponentBuildpackDescriptor<BM>),
    Composite(CompositeBuildpackDescriptor<BM>),
}

impl<BM> BuildpackDescriptor<BM> {
    pub fn buildpack(&self) -> &Buildpack {
        match self {
            BuildpackDescriptor::Component(descriptor) => &descriptor.buildpack,
            BuildpackDescriptor::Composite(descriptor) => &descriptor.buildpack,
        }
    }
}

/// Data structure for the Buildpack descriptor (buildpack.toml) of a component buildpack.
///
/// Representation of [buildpack.toml](https://github.com/buildpacks/spec/blob/main/buildpack.md#buildpacktoml-toml)
/// when the buildpack is a component buildpack - one that implements the Buildpack Interface
/// (ie: contains `/bin/detect` and `/bin/build` executables).
///
/// If support for multiple buildpack types is required, use [`BuildpackDescriptor`] instead.
///
/// # Example:
/// ```
/// use libcnb_data::buildpack::{ComponentBuildpackDescriptor, Stack};
/// use libcnb_data::buildpack_id;
///
/// let toml_str = r#"
/// api = "0.9"
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
/// id = "*"
/// "#;
///
/// let buildpack_descriptor =
///     toml::from_str::<ComponentBuildpackDescriptor>(toml_str).unwrap();
/// assert_eq!(buildpack_descriptor.buildpack.id, buildpack_id!("foo/bar"));
/// assert_eq!(buildpack_descriptor.stacks, [Stack::Any]);
/// ```
#[derive(Deserialize, Debug)]
#[serde(deny_unknown_fields)]
pub struct ComponentBuildpackDescriptor<BM = GenericMetadata> {
    pub api: BuildpackApi,
    pub buildpack: Buildpack,
    pub stacks: Vec<Stack>,
    pub metadata: BM,
}

/// Data structure for the Buildpack descriptor (buildpack.toml) of a composite buildpack.
///
/// Representation of [buildpack.toml](https://github.com/buildpacks/spec/blob/main/buildpack.md#buildpacktoml-toml)
/// when the buildpack is a composite buildpack - one that does not implement the Buildpack Interface
/// itself (ie: does not contain `/bin/detect` and `/bin/build` executables) but instead references
/// other buildpacks via an order definition.
///
/// If support for multiple buildpack types is required, use [`BuildpackDescriptor`] instead.
///
/// # Example:
/// ```
/// use libcnb_data::buildpack::CompositeBuildpackDescriptor;
/// use libcnb_data::buildpack_id;
///
/// let toml_str = r#"
/// api = "0.9"
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
/// [[order]]
///
/// [[order.group]]
/// id = "foo/baz"
/// version = "0.0.1"
/// "#;
///
/// let buildpack_descriptor =
///     toml::from_str::<CompositeBuildpackDescriptor>(toml_str).unwrap();
/// assert_eq!(buildpack_descriptor.buildpack.id, buildpack_id!("foo/bar"));
/// ```
#[derive(Deserialize, Debug)]
#[serde(deny_unknown_fields)]
pub struct CompositeBuildpackDescriptor<BM = GenericMetadata> {
    pub api: BuildpackApi,
    pub buildpack: Buildpack,
    pub order: Vec<Order>,
    pub metadata: BM,
}

#[derive(Deserialize, Debug)]
#[serde(deny_unknown_fields)]
pub struct Buildpack {
    pub id: BuildpackId,
    pub name: Option<String>,
    pub version: BuildpackVersion,
    pub homepage: Option<String>,
    #[serde(default, rename = "clear-env")]
    pub clear_env: bool,
    pub description: Option<String>,
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub keywords: Vec<String>,
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub licenses: Vec<License>,
    #[serde(
        default,
        rename = "sbom-formats",
        skip_serializing_if = "HashSet::is_empty"
    )]
    pub sbom_formats: HashSet<SbomFormat>,
}

#[derive(Deserialize, Debug, Eq, PartialEq)]
#[serde(deny_unknown_fields)]
pub struct License {
    pub r#type: Option<String>,
    pub uri: Option<String>,
}

#[derive(Deserialize, Debug, Eq, PartialEq)]
#[serde(deny_unknown_fields)]
pub struct Order {
    pub group: Vec<Group>,
}

#[derive(Deserialize, Debug, Eq, PartialEq)]
#[serde(deny_unknown_fields)]
pub struct Group {
    pub id: BuildpackId,
    pub version: BuildpackVersion,
    #[serde(default)]
    pub optional: bool,
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::sbom::SbomFormat;

    #[test]
    #[allow(clippy::too_many_lines)]
    fn deserialize_component_buildpack() {
        let toml_str = r#"
api = "0.9"

[buildpack]
id = "foo/bar"
name = "Bar Buildpack"
version = "0.0.1"
homepage = "https://example.tld"
clear-env = true
description = "A buildpack for Foo Bar"
keywords = ["foo", "bar"]
# Duplication of the Syft entry is intentional!
sbom-formats = ["application/vnd.cyclonedx+json", "application/spdx+json", "application/vnd.syft+json", "application/vnd.syft+json"]

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

        let buildpack_descriptor =
            toml::from_str::<ComponentBuildpackDescriptor>(toml_str).unwrap();

        assert_eq!(
            buildpack_descriptor.api,
            BuildpackApi { major: 0, minor: 9 }
        );
        assert_eq!(
            buildpack_descriptor.buildpack.id,
            "foo/bar".parse().unwrap()
        );
        assert_eq!(
            buildpack_descriptor.buildpack.name,
            Some(String::from("Bar Buildpack"))
        );
        assert_eq!(
            buildpack_descriptor.buildpack.version,
            BuildpackVersion::new(0, 0, 1)
        );
        assert_eq!(
            buildpack_descriptor.buildpack.homepage,
            Some(String::from("https://example.tld"))
        );
        assert!(buildpack_descriptor.buildpack.clear_env);
        assert_eq!(
            buildpack_descriptor.buildpack.description,
            Some(String::from("A buildpack for Foo Bar"))
        );
        assert_eq!(
            buildpack_descriptor.buildpack.keywords,
            [String::from("foo"), String::from("bar")]
        );
        assert_eq!(
            buildpack_descriptor.buildpack.licenses,
            [
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
            buildpack_descriptor.buildpack.sbom_formats,
            HashSet::from([
                SbomFormat::SyftJson,
                SbomFormat::CycloneDxJson,
                SbomFormat::SpdxJson
            ])
        );
        assert_eq!(
            buildpack_descriptor.stacks,
            [
                Stack::Specific {
                    // Cannot use the `stack_id!` macro due to: https://github.com/heroku/libcnb.rs/issues/179
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
        assert_eq!(
            buildpack_descriptor.metadata.unwrap().get("checksum"),
            Some(&toml::value::Value::try_from("abc123").unwrap())
        );
    }

    #[test]
    fn deserialize_composite_buildpack() {
        let toml_str = r#"
api = "0.9"

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

        let buildpack_descriptor =
            toml::from_str::<CompositeBuildpackDescriptor>(toml_str).unwrap();

        assert_eq!(
            buildpack_descriptor.api,
            BuildpackApi { major: 0, minor: 9 }
        );
        assert_eq!(
            buildpack_descriptor.buildpack.id,
            "foo/bar".parse().unwrap()
        );
        assert_eq!(
            buildpack_descriptor.buildpack.name,
            Some(String::from("Bar Buildpack"))
        );
        assert_eq!(
            buildpack_descriptor.buildpack.version,
            BuildpackVersion::new(0, 0, 1)
        );
        assert_eq!(
            buildpack_descriptor.buildpack.homepage,
            Some(String::from("https://example.tld"))
        );
        assert!(buildpack_descriptor.buildpack.clear_env);
        assert_eq!(
            buildpack_descriptor.buildpack.description,
            Some(String::from("A buildpack for Foo Bar"))
        );
        assert_eq!(
            buildpack_descriptor.buildpack.keywords,
            [String::from("foo"), String::from("bar")]
        );
        assert_eq!(
            buildpack_descriptor.buildpack.licenses,
            [
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
            buildpack_descriptor.order,
            [Order {
                group: vec![
                    Group {
                        id: "foo/bar".parse().unwrap(),
                        version: BuildpackVersion::new(0, 0, 1),
                        optional: false
                    },
                    Group {
                        id: "foo/baz".parse().unwrap(),
                        version: BuildpackVersion::new(0, 1, 0),
                        optional: true
                    }
                ]
            }]
        );
        assert_eq!(
            buildpack_descriptor.metadata.unwrap().get("checksum"),
            Some(&toml::value::Value::try_from("abc123").unwrap())
        );
    }

    #[test]
    fn deserialize_minimal_component_buildpack() {
        let toml_str = r#"
api = "0.9"

[buildpack]
id = "foo/bar"
version = "0.0.1"

[[stacks]]
id = "*"
        "#;

        let buildpack_descriptor =
            toml::from_str::<ComponentBuildpackDescriptor>(toml_str).unwrap();

        assert_eq!(
            buildpack_descriptor.api,
            BuildpackApi { major: 0, minor: 9 }
        );
        assert_eq!(
            buildpack_descriptor.buildpack.id,
            "foo/bar".parse().unwrap()
        );
        assert_eq!(buildpack_descriptor.buildpack.name, None);
        assert_eq!(
            buildpack_descriptor.buildpack.version,
            BuildpackVersion::new(0, 0, 1)
        );
        assert_eq!(buildpack_descriptor.buildpack.homepage, None);
        assert!(!buildpack_descriptor.buildpack.clear_env);
        assert_eq!(buildpack_descriptor.buildpack.description, None);
        assert_eq!(
            buildpack_descriptor.buildpack.keywords,
            Vec::<String>::new()
        );
        assert_eq!(buildpack_descriptor.buildpack.licenses, Vec::new());
        assert_eq!(buildpack_descriptor.buildpack.sbom_formats, HashSet::new());
        assert_eq!(buildpack_descriptor.stacks, [Stack::Any]);
        assert_eq!(buildpack_descriptor.metadata, None);
    }

    #[test]
    fn deserialize_minimal_composite_buildpack() {
        let toml_str = r#"
api = "0.9"

[buildpack]
id = "foo/bar"
version = "0.0.1"

[[order]]

[[order.group]]
id = "foo/bar"
version = "0.0.1"
"#;

        let buildpack_descriptor =
            toml::from_str::<CompositeBuildpackDescriptor>(toml_str).unwrap();

        assert_eq!(
            buildpack_descriptor.api,
            BuildpackApi { major: 0, minor: 9 }
        );
        assert_eq!(
            buildpack_descriptor.buildpack.id,
            "foo/bar".parse().unwrap()
        );
        assert_eq!(buildpack_descriptor.buildpack.name, None);
        assert_eq!(
            buildpack_descriptor.buildpack.version,
            BuildpackVersion::new(0, 0, 1)
        );
        assert_eq!(buildpack_descriptor.buildpack.homepage, None);
        assert!(!buildpack_descriptor.buildpack.clear_env);
        assert_eq!(buildpack_descriptor.buildpack.description, None);
        assert_eq!(
            buildpack_descriptor.buildpack.keywords,
            Vec::<String>::new()
        );
        assert_eq!(buildpack_descriptor.buildpack.licenses, Vec::new());
        assert_eq!(
            buildpack_descriptor.order,
            [Order {
                group: vec![Group {
                    id: "foo/bar".parse().unwrap(),
                    version: BuildpackVersion::new(0, 0, 1),
                    optional: false
                }]
            }]
        );
        assert_eq!(buildpack_descriptor.metadata, None);
    }

    #[test]
    fn deserialize_buildpackdescriptor_component() {
        let toml_str = r#"
api = "0.9"

[buildpack]
id = "foo/bar"
version = "0.0.1"

[[stacks]]
id = "*"
        "#;

        let buildpack_descriptor = toml::from_str::<BuildpackDescriptor>(toml_str).unwrap();
        assert!(matches!(
            buildpack_descriptor,
            BuildpackDescriptor::Component(_)
        ));
    }

    #[test]
    fn deserialize_buildpackdescriptor_composite() {
        let toml_str = r#"
api = "0.9"

[buildpack]
id = "foo/bar"
version = "0.0.1"

[[order]]

[[order.group]]
id = "foo/baz"
version = "0.0.1"
        "#;

        let buildpack_descriptor = toml::from_str::<BuildpackDescriptor>(toml_str).unwrap();
        assert!(matches!(
            buildpack_descriptor,
            BuildpackDescriptor::Composite(_)
        ));
    }

    #[test]
    fn reject_buildpack_with_both_stacks_and_order() {
        let toml_str = r#"
api = "0.9"

[buildpack]
id = "foo/bar"
version = "0.0.1"

[[stacks]]
id = "*"

[[order]]

[[order.group]]
id = "foo/baz"
version = "0.0.1"
"#;

        let err = toml::from_str::<BuildpackDescriptor>(toml_str).unwrap_err();
        assert_eq!(
            err.to_string(),
            "data did not match any variant of untagged enum BuildpackDescriptor\n"
        );

        let err = toml::from_str::<ComponentBuildpackDescriptor>(toml_str).unwrap_err();
        assert!(err.to_string().contains("unknown field `order`"));

        let err = toml::from_str::<CompositeBuildpackDescriptor>(toml_str).unwrap_err();
        assert!(err.to_string().contains("unknown field `stacks`"));
    }
}
