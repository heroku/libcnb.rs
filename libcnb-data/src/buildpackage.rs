use crate::buildpackage::PlatformOs::Linux;
use serde::{Deserialize, Deserializer, Serialize, Serializer};
use std::path::PathBuf;
use uriparse::URIReference;

fn platform_default() -> Platform {
    Platform::default()
}

#[derive(Debug, Deserialize, Serialize, Clone)]
#[serde(deny_unknown_fields)]
pub struct Buildpackage {
    pub buildpack: BuildpackageBuildpack,
    #[serde(default)]
    pub dependencies: Vec<BuildpackageDependency>,
    #[serde(default = "platform_default")]
    pub platform: Platform,
}

#[derive(Debug, Deserialize, Serialize, Eq, PartialEq, Clone)]
#[serde(deny_unknown_fields)]
pub struct BuildpackageBuildpack {
    #[serde(deserialize_with = "deserialize_uri_reference")]
    #[serde(serialize_with = "serialize_uri_reference")]
    pub uri: URIReference<'static>,
}

#[derive(Debug, Deserialize, Serialize, Eq, PartialEq, Clone)]
#[serde(deny_unknown_fields)]
pub struct BuildpackageDependency {
    #[serde(deserialize_with = "deserialize_uri_reference")]
    #[serde(serialize_with = "serialize_uri_reference")]
    pub uri: URIReference<'static>,
}

#[derive(Debug)]
pub enum BuildpackageDependencyError {
    InvalidUri,
}

impl TryFrom<PathBuf> for BuildpackageDependency {
    type Error = BuildpackageDependencyError;

    fn try_from(value: PathBuf) -> Result<Self, Self::Error> {
        let value = value.to_string_lossy().to_string();
        let uri = URIReference::try_from(value.as_str())
            .map_err(|_| BuildpackageDependencyError::InvalidUri)?;
        Ok(BuildpackageDependency {
            uri: uri.into_owned(),
        })
    }
}

#[derive(Debug, Deserialize, Serialize, Clone)]
#[serde(deny_unknown_fields)]
pub struct Platform {
    pub os: PlatformOs,
}

impl Platform {
    fn default() -> Platform {
        Platform { os: Linux }
    }
}

#[derive(Debug, Deserialize, Serialize, Eq, PartialEq, Clone)]
#[serde(rename_all = "lowercase")]
pub enum PlatformOs {
    Linux,
    Windows,
}

fn deserialize_uri_reference<'de, D>(deserializer: D) -> Result<URIReference<'static>, D::Error>
where
    D: Deserializer<'de>,
{
    let value = String::deserialize(deserializer)?;
    let uri = URIReference::try_from(value.as_str()).map_err(serde::de::Error::custom)?;
    Ok(uri.into_owned())
}

fn serialize_uri_reference<S>(uri: &URIReference, serializer: S) -> Result<S::Ok, S::Error>
where
    S: Serializer,
{
    let value = uri.to_string();
    serializer.serialize_str(value.as_str())
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::buildpackage::PlatformOs::Windows;

    #[test]
    fn it_parses_minimal() {
        let toml_str = r#"
[buildpack]
uri = "."
"#;

        let buildpackage = toml::from_str::<Buildpackage>(toml_str).unwrap();
        assert_eq!(
            buildpackage.buildpack.uri,
            URIReference::try_from(".").unwrap()
        );
        assert_eq!(buildpackage.platform.os, Linux);
    }

    #[test]
    fn it_parses_with_dependencies_and_platform() {
        let toml_str = r#"
[buildpack]
uri = "."

[[dependencies]]
uri = "libcnb:dependency_1"

[[dependencies]]
uri = "libcnb:dependency_2"

[platform]
os = "windows"
"#;

        let buildpackage = toml::from_str::<Buildpackage>(toml_str).unwrap();
        assert_eq!(
            buildpackage.buildpack.uri,
            URIReference::try_from(".").unwrap()
        );
        assert_eq!(buildpackage.platform.os, Windows);
        assert_eq!(
            buildpackage.dependencies,
            vec![
                BuildpackageDependency {
                    uri: URIReference::try_from("libcnb:dependency_1").unwrap()
                },
                BuildpackageDependency {
                    uri: URIReference::try_from("libcnb:dependency_2").unwrap()
                }
            ]
        );
    }

    #[test]
    fn it_serializes() {
        let buildpackage = Buildpackage {
            buildpack: BuildpackageBuildpack {
                uri: URIReference::try_from(".").unwrap(),
            },
            dependencies: vec![
                BuildpackageDependency {
                    uri: URIReference::try_from("libcnb:id").unwrap(),
                },
                BuildpackageDependency {
                    uri: URIReference::try_from("docker://docker.io/heroku/procfile-cnb:2.0.0")
                        .unwrap(),
                },
            ],
            platform: Platform { os: Linux },
        };

        let buildpackage_contents = toml::to_string(&buildpackage).unwrap();
        assert_eq!(
            buildpackage_contents,
            r#"
[buildpack]
uri = "."

[[dependencies]]
uri = "libcnb:id"

[[dependencies]]
uri = "docker://docker.io/heroku/procfile-cnb:2.0.0"

[platform]
os = "linux"
"#
            .trim_start()
        );
    }
}
