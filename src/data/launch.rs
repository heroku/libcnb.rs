use crate::data::bom;
use serde::Serialize;

#[derive(Serialize, Debug)]
pub struct Launch {
    pub bom: bom::Bom,
    pub labels: Vec<Label>,
    pub processes: Vec<Process>,
    pub slices: Vec<String>,
}

#[derive(Serialize, Debug)]
pub struct Label {
    pub key: String,
    pub value: String,
}

#[derive(Serialize, Debug)]
pub struct Process {
    // MUST only contain numbers, letters, and the characters ., _, and -.
    pub r#type: String,
    pub command: String,
    pub args: Vec<String>,
    pub direct: bool,
}

impl Process {
    pub fn new(
        r#type: impl Into<String>,
        command: impl Into<String>,
        args: impl IntoIterator<Item = impl Into<String>>,
        direct: bool,
    ) -> Self {
        Process {
            r#type: r#type.into(),
            command: command.into(),
            args: args.into_iter().map(|i| i.into()).collect(),
            direct,
        }
    }
}

pub struct Slice {
    pub paths: Vec<String>,
}
