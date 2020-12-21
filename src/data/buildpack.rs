use serde::Deserialize;

#[derive(Deserialize, Debug)]
pub struct BuildpackToml {
    // MUST be in form <major>.<minor> or <major>, where <major> is equivalent to <major>.0.
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
    // MUST be in the form <X>.<Y>.<Z> where X, Y, and Z are non-negative integers and must not contain leading zeroes
    pub version: String,
    pub homepage: Option<String>,
    pub clear_env: bool,
}

#[derive(Deserialize, Debug)]
pub struct Stack {
    pub id: String,
    pub mixins: Vec<String>,
}
