use crate::data::bom;
use lazy_static::lazy_static;
use regex::Regex;
use serde::Serialize;
use std::convert::TryFrom;

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

struct ProcessType(String);

impl TryFrom<String> for ProcessType {
    type Error = &'static str;

    fn try_from(value: String) -> Result<Self, Self::Error> {
        lazy_static! {
            static ref RE: Regex = Regex::new("^[a-zA-Z0-9_-]+$").unwrap();
        }

        if RE.is_match(&value) {
            Ok(ProcessType(value))
        } else {
            Err("")
        }
    }
}
