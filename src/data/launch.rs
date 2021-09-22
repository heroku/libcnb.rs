use crate::data::bom;
use crate::data::defaults;
use lazy_static::lazy_static;
use regex::Regex;
use serde::{Deserialize, Serialize};
use std::str::FromStr;
use thiserror;

#[derive(Deserialize, Serialize, Debug)]
pub struct Launch {
    #[serde(skip_serializing_if = "Vec::is_empty")]
    pub bom: bom::Bom,
    #[serde(skip_serializing_if = "Vec::is_empty")]
    pub labels: Vec<Label>,
    #[serde(skip_serializing_if = "Vec::is_empty")]
    pub processes: Vec<Process>,
    #[serde(skip_serializing_if = "Vec::is_empty")]
    pub slices: Vec<Slice>,
}

/// Data Structure for the launch.toml file.
///
/// # Examples
/// ```
/// use libcnb::data::launch;
/// let mut launch_toml = launch::Launch::new();
/// let web = launch::Process::new("web", "bundle", vec!["exec", "ruby", "app.rb"],
/// false, false).unwrap();
///
/// launch_toml.processes.push(web);
/// assert!(toml::to_string(&launch_toml).is_ok());
/// ```
impl Launch {
    pub fn new() -> Self {
        Launch {
            bom: bom::Bom::new(),
            labels: Vec::new(),
            processes: Vec::new(),
            slices: Vec::new(),
        }
    }

    pub fn process(mut self, process: Process) -> Self {
        self.processes.push(process);
        self
    }
}

impl Default for Launch {
    fn default() -> Self {
        Launch::new()
    }
}

#[derive(Deserialize, Serialize, Debug)]
pub struct Label {
    pub key: String,
    pub value: String,
}

#[derive(Deserialize, Serialize, Debug)]
pub struct Process {
    pub r#type: ProcessType,
    pub command: String,
    pub args: Vec<String>,
    pub direct: bool,
    #[serde(default = "defaults::r#false")]
    pub default: bool,
}

impl Process {
    pub fn new(
        r#type: impl AsRef<str>,
        command: impl Into<String>,
        args: impl IntoIterator<Item = impl Into<String>>,
        direct: bool,
        default: bool,
    ) -> Result<Self, ProcessTypeError> {
        Ok(Process {
            r#type: ProcessType::from_str(r#type.as_ref())?,
            command: command.into(),
            args: args.into_iter().map(|i| i.into()).collect(),
            direct,
            default,
        })
    }
}

#[derive(Deserialize, Serialize, Debug)]
pub struct Slice {
    pub paths: Vec<String>,
}

/// launch.toml Process Type. This is a newtype wrapper around a String. It MUST only contain numbers, letters, and the characters ., _, and -. Use [`std::str::FromStr`] to create a new instance of this struct.
///
/// # Examples
/// ```
/// use std::str::FromStr;
/// use libcnb::data::launch::ProcessType;
///
/// let valid = ProcessType::from_str("foo-Bar_9");
/// assert_eq!(valid.unwrap().as_str(), "foo-Bar_9");
///
/// let invalid = ProcessType::from_str("!nv4lid");
/// assert!(invalid.is_err());
/// ```
#[derive(Deserialize, Serialize, Debug, PartialEq, Eq)]
pub struct ProcessType(String);

impl ProcessType {
    pub fn as_str(&self) -> &str {
        &self.0
    }
}

impl FromStr for ProcessType {
    type Err = ProcessTypeError;

    fn from_str(value: &str) -> Result<Self, Self::Err> {
        lazy_static! {
            static ref RE: Regex = Regex::new(r"^[[:alnum:]\._-]+$").unwrap();
        }

        let string = String::from(value);
        if RE.is_match(value) {
            Ok(ProcessType(string))
        } else {
            Err(ProcessTypeError::InvalidProcessType(string))
        }
    }
}

#[derive(thiserror::Error, Debug)]
pub enum ProcessTypeError {
    #[error(
        "Found `{0}` but value MUST only contain numbers, letters, and the characters ., _, and -."
    )]
    InvalidProcessType(String),
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::str::FromStr;

    #[test]
    fn test_process_type_eq() {
        assert_eq!(
            ProcessType::from_str("web").unwrap(),
            ProcessType::from_str("web").unwrap()
        );
        assert_ne!(
            ProcessType::from_str("web").unwrap(),
            ProcessType::from_str("nope").unwrap()
        )
    }

    #[test]
    fn test_process_type_with_special_chars() {
        assert!(ProcessType::from_str("java_jar").is_ok());
        assert!(ProcessType::from_str("java-jar").is_ok());
        assert!(ProcessType::from_str("java.jar").is_ok());

        assert!(ProcessType::from_str("java~jar").is_err());
    }
}
