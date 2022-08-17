use crate::bom::Bom;
use crate::newtypes::libcnb_newtype;
use serde::{Deserialize, Serialize};

/// Data Structure for the launch.toml file.
#[derive(Deserialize, Serialize, Clone, Debug, Default)]
#[serde(deny_unknown_fields)]
pub struct Launch {
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub bom: Bom,
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub labels: Vec<Label>,
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub processes: Vec<Process>,
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub slices: Vec<Slice>,
}

/// A non-consuming builder for [`Launch`] values.
///
/// # Examples
/// ```
/// use libcnb_data::launch::{LaunchBuilder, ProcessBuilder};
/// use libcnb_data::process_type;
///
/// let launch_toml = LaunchBuilder::new()
///     .process(
///         ProcessBuilder::new(process_type!("web"), "bundle")
///             .args(vec!["exec", "ruby", "app.rb"])
///             .build(),
///     )
///     .build();
///
/// assert!(toml::to_string(&launch_toml).is_ok());
/// ```
#[derive(Default)]
pub struct LaunchBuilder {
    launch: Launch,
}

impl LaunchBuilder {
    #[must_use]
    pub fn new() -> Self {
        Self::default()
    }

    /// Adds a process to the launch configuration.
    pub fn process<P: Into<Process>>(&mut self, process: P) -> &mut Self {
        self.launch.processes.push(process.into());
        self
    }

    /// Adds multiple processes to the launch configuration.
    pub fn processes<I: IntoIterator<Item = P>, P: Into<Process>>(
        &mut self,
        processes: I,
    ) -> &mut Self {
        for process in processes {
            self.process(process);
        }

        self
    }

    /// Adds a BOM to the launch configuration.
    pub fn bom<B: Into<Bom>>(&mut self, bom: B) -> &mut Self {
        self.launch.bom = bom.into();
        self
    }

    /// Adds a label to the launch configuration.
    pub fn label<L: Into<Label>>(&mut self, label: L) -> &mut Self {
        self.launch.labels.push(label.into());
        self
    }

    /// Adds multiple processes to the launch configuration.
    pub fn labels<I: IntoIterator<Item = L>, L: Into<Label>>(&mut self, labels: I) -> &mut Self {
        for label in labels {
            self.label(label);
        }

        self
    }

    /// Adds a slice to the launch configuration.
    pub fn slice<S: Into<Slice>>(&mut self, slice: S) -> &mut Self {
        self.launch.slices.push(slice.into());
        self
    }

    /// Adds multiple slices to the launch configuration.
    pub fn slices<I: IntoIterator<Item = S>, S: Into<Slice>>(&mut self, slices: I) -> &mut Self {
        for slice in slices {
            self.slice(slice);
        }

        self
    }

    /// Builds the `Launch` based on the configuration of this builder.
    #[must_use]
    pub fn build(&self) -> Launch {
        self.launch.clone()
    }
}

#[derive(Deserialize, Serialize, Clone, Debug)]
#[serde(deny_unknown_fields)]
pub struct Label {
    pub key: String,
    pub value: String,
}

#[derive(Deserialize, Serialize, Debug, Clone, PartialEq, Eq)]
#[serde(deny_unknown_fields)]
pub struct Process {
    pub r#type: ProcessType,
    pub command: String,
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub args: Vec<String>,
    #[serde(default, skip_serializing_if = "std::ops::Not::not")]
    pub direct: bool,
    #[serde(default, skip_serializing_if = "std::ops::Not::not")]
    pub default: bool,
}

pub struct ProcessBuilder {
    process: Process,
}

/// A non-consuming builder for [`Process`] values.
///
/// # Examples
/// ```
/// # use libcnb_data::process_type;
/// # use libcnb_data::launch::ProcessBuilder;
/// ProcessBuilder::new(process_type!("web"), "java")
///     .arg("-jar")
///     .arg("target/application-1.0.0.jar")
///     .default(true)
///     .build();
/// ```
impl ProcessBuilder {
    /// Constructs a new `ProcessBuilder` with the following defaults:
    ///
    /// * No arguments to the process
    /// * `direct` is `false`
    /// * `default` is `false`
    pub fn new(r#type: ProcessType, command: impl Into<String>) -> Self {
        Self {
            process: Process {
                r#type,
                command: command.into(),
                args: Vec::new(),
                direct: false,
                default: false,
            },
        }
    }

    /// Adds an argument to the process.
    ///
    /// Only one argument can be passed per use. So instead of:
    /// ```
    /// # use libcnb_data::process_type;
    /// # libcnb_data::launch::ProcessBuilder::new(process_type!("web"), "command")
    /// .arg("-C /path/to/repo")
    /// # ;
    /// ```
    ///
    /// usage would be:
    ///
    /// ```
    /// # use libcnb_data::process_type;
    /// # libcnb_data::launch::ProcessBuilder::new(process_type!("web"), "command")
    /// .arg("-C")
    /// .arg("/path/to/repo")
    /// # ;
    /// ```
    ///
    /// To pass multiple arguments see [`args`](Self::args).
    pub fn arg(&mut self, arg: impl Into<String>) -> &mut Self {
        self.process.args.push(arg.into());
        self
    }

    /// Adds multiple arguments to pass to the process.
    ///
    /// To pass a single argument see [`arg`](Self::arg).
    pub fn args(&mut self, args: impl IntoIterator<Item = impl Into<String>>) -> &mut Self {
        for arg in args {
            self.arg(arg);
        }

        self
    }

    /// Sets the `direct` flag on the process.
    ///
    /// If this is true, the lifecycle will launch the command directly, rather than via a shell.
    pub fn direct(&mut self, value: bool) -> &mut Self {
        self.process.direct = value;
        self
    }

    /// Sets the `default` flag on the process.
    ///
    /// Indicates that the process type should be selected as the buildpack-provided
    /// default during the export phase.
    pub fn default(&mut self, value: bool) -> &mut Self {
        self.process.default = value;
        self
    }

    /// Builds the `Process` based on the configuration of this builder.
    #[must_use]
    pub fn build(&self) -> Process {
        self.process.clone()
    }
}

#[derive(Deserialize, Serialize, Clone, Debug)]
#[serde(deny_unknown_fields)]
pub struct Slice {
    /// Path globs for this slice.
    ///
    /// These globs need to follow the pattern syntax defined in the [Go standard library](https://golang.org/pkg/path/filepath/#Match)
    /// and only match files/directories inside the application directory.
    #[serde(rename = "paths")]
    pub path_globs: Vec<String>,
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
    fn launch_builder_add_processes() {
        let launch = LaunchBuilder::new()
            .process(ProcessBuilder::new(process_type!("web"), "web_command").build())
            .processes(vec![
                ProcessBuilder::new(process_type!("another"), "another_command").build(),
                ProcessBuilder::new(process_type!("worker"), "worker_command").build(),
            ])
            .build();

        assert_eq!(
            launch.processes,
            vec![
                ProcessBuilder::new(process_type!("web"), "web_command").build(),
                ProcessBuilder::new(process_type!("another"), "another_command").build(),
                ProcessBuilder::new(process_type!("worker"), "worker_command").build(),
            ]
        );
    }

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

    #[test]
    fn process_with_default_values_deserialization() {
        let toml_str = r#"
type = "web"
command = "foo"
"#;

        assert_eq!(
            toml::from_str::<Process>(toml_str),
            Ok(Process {
                r#type: process_type!("web"),
                command: String::from("foo"),
                args: vec![],
                direct: false,
                default: false
            })
        );
    }

    #[test]
    fn process_with_default_values_serialization() {
        let process = ProcessBuilder::new(process_type!("web"), "foo").build();

        let string = toml::to_string(&process).unwrap();
        assert_eq!(
            string,
            r#"type = "web"
command = "foo"
"#
        );
    }

    #[test]
    fn process_with_some_default_values_serialization() {
        let process = ProcessBuilder::new(process_type!("web"), "foo")
            .default(true)
            .build();

        let string = toml::to_string(&process).unwrap();
        assert_eq!(
            string,
            r#"type = "web"
command = "foo"
default = true
"#
        );
    }

    #[test]
    fn process_builder() {
        let mut process_builder = ProcessBuilder::new(process_type!("web"), "java");

        assert_eq!(
            process_builder.build(),
            Process {
                r#type: process_type!("web"),
                command: String::from("java"),
                args: vec![],
                direct: false,
                default: false
            }
        );

        process_builder.default(true);

        assert_eq!(
            process_builder.build(),
            Process {
                r#type: process_type!("web"),
                command: String::from("java"),
                args: vec![],
                direct: false,
                default: true
            }
        );

        process_builder.direct(true);

        assert_eq!(
            process_builder.build(),
            Process {
                r#type: process_type!("web"),
                command: String::from("java"),
                args: vec![],
                direct: true,
                default: true
            }
        );
    }

    #[test]
    fn process_builder_args() {
        assert_eq!(
            ProcessBuilder::new(process_type!("web"), "java")
                .arg("foo")
                .args(vec!["baz", "eggs"])
                .arg("bar")
                .build(),
            Process {
                r#type: process_type!("web"),
                command: String::from("java"),
                args: vec![
                    String::from("foo"),
                    String::from("baz"),
                    String::from("eggs"),
                    String::from("bar"),
                ],
                direct: false,
                default: false
            }
        );
    }
}
