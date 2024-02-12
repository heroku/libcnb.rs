use serde::Deserialize;

#[derive(Debug, Eq, PartialEq, Clone, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Target {
    pub os: Option<String>,
    pub arch: Option<String>,
    pub variant: Option<String>,
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub distros: Vec<Distro>,
}

#[derive(Debug, Eq, PartialEq, Clone, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Distro {
    pub name: String,
    pub version: String,
}
