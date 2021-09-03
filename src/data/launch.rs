use crate::data::bom;
use lazy_static::lazy_static;
use regex::Regex;
use serde::Serialize;
use std::str::FromStr;
use thiserror;

#[derive(Serialize, Debug)]
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
/// false).unwrap();
///
/// launch_toml.processes.push(web);
/// let toml_string = toml::to_string(&launch_toml);
/// assert!(toml_string.is_ok());
/// assert_eq!(toml_string.unwrap(), r#"
/// [[processes]]
/// type = "web"
/// command = "bundle"
/// args = ["exec", "ruby", "app.rb"]
/// direct = false
/// "#.trim_start());
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

#[derive(Serialize, Debug)]
pub struct Label {
    pub key: String,
    pub value: String,
}

#[derive(Serialize, Debug)]
pub struct Process {
    pub r#type: ProcessType,
    pub command: String,
    pub args: Vec<String>,
    pub direct: bool,
}

impl Process {
    pub fn new(
        r#type: impl AsRef<str>,
        command: impl Into<String>,
        args: impl IntoIterator<Item = impl Into<String>>,
        direct: bool,
    ) -> Result<Self, ProcessTypeError> {
        Ok(Process {
            r#type: ProcessType::from_str(r#type.as_ref())?,
            command: command.into(),
            args: args.into_iter().map(|i| i.into()).collect(),
            direct,
        })
    }
}

#[derive(Serialize, Debug)]
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
#[derive(Serialize, Debug)]
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
            static ref RE: Regex = Regex::new(r"^[[:alnum:]_-]+$").unwrap();
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
