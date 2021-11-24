use crate::bom;
use crate::newtypes::libcnb_newtype;
use serde::{Deserialize, Serialize};

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
/// use libcnb_data::launch;
/// use libcnb_data::process_type;
///
/// let mut launch_toml = launch::Launch::new();
/// let web = launch::Process::new(process_type!("web"), "bundle", vec!["exec", "ruby", "app.rb"],
/// false, false);
///
/// launch_toml.processes.push(web);
/// assert!(toml::to_string(&launch_toml).is_ok());
/// ```
impl Launch {
    pub fn new() -> Self {
        Self {
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
        Self::new()
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
    #[serde(default)]
    pub default: bool,
}

impl Process {
    pub fn new(
        r#type: ProcessType,
        command: impl Into<String>,
        args: impl IntoIterator<Item = impl Into<String>>,
        direct: bool,
        default: bool,
    ) -> Self {
        Self {
            r#type,
            command: command.into(),
            args: args.into_iter().map(std::convert::Into::into).collect(),
            direct,
            default,
        }
    }
}

#[derive(Deserialize, Serialize, Debug)]
pub struct Slice {
    pub paths: Vec<String>,
}

libcnb_newtype!(
    launch,
    /// Construct a [`ProcessType`] value at compile time.
    ///
    /// Passing a string that is not a valid `ProcessType` value will yield a compilation error.
    ///
    /// # Examples:
    /// ```
    /// use libcnb_data::launch::ProcessType;
    /// use libcnb_data::process_type;
    ///
    /// let process_type: ProcessType = process_type!("web");
    /// ```
    process_type,
    /// The type of a process.
    ///
    /// It MUST only contain numbers, letters, and the characters `.`, `_`, and `-`.
    ///
    /// Use the [`process_type`](crate::process_type) macro to construct a `ProcessType` from a
    /// literal string. To parse a dynamic string into a `ProcessType`, use
    /// [`str::parse`](str::parse).
    ///
    /// # Examples
    /// ```
    /// use libcnb_data::launch::ProcessType;
    /// use libcnb_data::process_type;
    ///
    /// let from_literal = process_type!("web");
    ///
    /// let input = "web";
    /// let from_dynamic: ProcessType = input.parse().unwrap();
    /// assert_eq!(from_dynamic, from_literal);
    ///
    /// let input = "!nv4lid";
    /// let invalid: Result<ProcessType, _> = input.parse();
    /// assert!(invalid.is_err());
    /// ```
    ProcessType,
    ProcessTypeError,
    r"^[[:alnum:]._-]+$"
);

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn process_type_validation_valid() {
        assert!("web".parse::<ProcessType>().is_ok());
        assert!("Abc123._-".parse::<ProcessType>().is_ok());
    }

    #[test]
    fn process_type_validation_invalid() {
        assert_eq!(
            "worker/foo".parse::<ProcessType>(),
            Err(ProcessTypeError::InvalidValue(String::from("worker/foo")))
        );
        assert_eq!(
            "worker:foo".parse::<ProcessType>(),
            Err(ProcessTypeError::InvalidValue(String::from("worker:foo")))
        );
        assert_eq!(
            "worker foo".parse::<ProcessType>(),
            Err(ProcessTypeError::InvalidValue(String::from("worker foo")))
        );
        assert_eq!(
            "".parse::<ProcessType>(),
            Err(ProcessTypeError::InvalidValue(String::new()))
        );
    }
}
