use crate::buildpackage::PlatformOs::Linux;
use serde::{Deserialize, Serialize};

fn platform_default() -> Platform {
    Platform::default()
}

#[derive(Debug, Deserialize, Serialize, Clone)]
#[serde(deny_unknown_fields)]
pub struct Buildpackage {
    pub buildpack: BuildpackageUri,
    #[serde(default)]
    pub dependencies: Vec<BuildpackageUri>,
    #[serde(default = "platform_default")]
    pub platform: Platform,
}

#[derive(Debug, Deserialize, Serialize, Eq, PartialEq, Clone)]
#[serde(deny_unknown_fields)]
pub struct BuildpackageUri {
    pub uri: String,
}

#[derive(Debug, Deserialize, Serialize, Clone)]
#[serde(deny_unknown_fields)]
pub struct Platform {
    // TODO: this should be limited to (linux | windows)
    pub os: PlatformOs,
}

#[derive(Debug, Deserialize, Serialize, Eq, PartialEq, Clone)]
#[serde(rename_all = "lowercase")]
pub enum PlatformOs {
    Linux,
    Windows,
}

impl Platform {
    fn default() -> Platform {
        Platform { os: Linux }
    }
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
        assert_eq!(buildpackage.buildpack.uri, ".");
        assert_eq!(buildpackage.platform.os, Linux);
    }

    #[test]
    fn it_parses_with_dependencies_and_platform() {
        let toml_str = r#"
[buildpack]
uri = "."

[[dependencies]]
uri = "dependency_1"

[[dependencies]]
uri = "dependency_2"

[platform]
os = "windows"
"#;

        let buildpackage = toml::from_str::<Buildpackage>(toml_str).unwrap();
        assert_eq!(buildpackage.buildpack.uri, ".");
        assert_eq!(buildpackage.platform.os, Windows);
        assert_eq!(
            buildpackage.dependencies,
            vec![
                BuildpackageUri {
                    uri: String::from("dependency_1")
                },
                BuildpackageUri {
                    uri: String::from("dependency_2")
                }
            ]
        );
    }
}
