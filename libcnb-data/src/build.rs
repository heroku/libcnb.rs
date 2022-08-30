use serde::Serialize;

#[derive(Serialize, Debug)]
pub struct Build {
    pub unmet: Vec<String>,
}
