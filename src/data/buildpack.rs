use serde::Deserialize;

#[derive(Deserialize, Debug)]
pub struct BuildpackToml {
    pub api: String,
    pub buildpack: Buildpack,
    pub stacks: Vec<Stack>,
    pub metadata: toml::value::Table,
}

#[derive(Deserialize, Debug)]
pub struct Buildpack {
    // MUST only contain numbers, letters, and the characters ., /, and -.
    // MUST NOT be config or app
    pub id: String,
    pub name: String,
    pub version: String,
    pub homepage: Option<String>,
    pub clear_env: bool,
}

#[derive(Deserialize, Debug)]
pub struct Stack {
    pub id: String,
    pub mixins: Vec<String>,
}
