use crate::newtypes::libcnb_newtype;
use serde::Serialize;
use std::collections::HashMap;

#[derive(Serialize, Clone)]
pub struct ExecDProgramOutput(HashMap<ExecDProgramOutputKey, String>);

impl ExecDProgramOutput {
    #[must_use]
    pub fn new(map: HashMap<ExecDProgramOutputKey, String>) -> Self {
        Self(map)
    }
}

impl<K: Into<ExecDProgramOutputKey>, V: Into<String>, A: IntoIterator<Item = (K, V)>> From<A>
    for ExecDProgramOutput
{
    fn from(a: A) -> Self {
        Self(
            a.into_iter()
                .map(|(key, value)| (key.into(), value.into()))
                .collect(),
        )
    }
}

libcnb_newtype!(
    exec_d,
    /// Construct a [`ExecDProgramOutputKey`] value at compile time.
    ///
    /// Passing a string that is not a valid `ExecDProgramOutputKey` value will yield a compilation
    /// error.
    ///
    /// # Examples:
    /// ```
    /// use libcnb_data::exec_d::ExecDProgramOutputKey;
    /// use libcnb_data::exec_d_program_output_key;
    ///
    /// let key: ExecDProgramOutputKey = exec_d_program_output_key!("PATH");
    /// ```
    exec_d_program_output_key,
    /// A key of from exec.d program output
    ///
    /// It MUST only contain numbers, letters, and the characters `_` and `-`.
    ///
    /// Use the [`exec_d_program_output_key`](crate::exec_d_program_output_key) macro to construct
    /// a `ExecDProgramOutputKey` from a literal string. To parse a dynamic string into a
    /// `ExecDProgramOutputKey`, use [`str::parse`](str::parse).
    ///
    /// # Examples
    /// ```
    /// use libcnb_data::exec_d::ExecDProgramOutputKey;
    /// use libcnb_data::exec_d_program_output_key;
    ///
    /// let from_literal = exec_d_program_output_key!("ENV_VAR");
    ///
    /// let input = "ENV_VAR";
    /// let from_dynamic: ExecDProgramOutputKey = input.parse().unwrap();
    /// assert_eq!(from_dynamic, from_literal);
    ///
    /// let input = "!nv4lid";
    /// let invalid: Result<ExecDProgramOutputKey, _> = input.parse();
    /// assert!(invalid.is_err());
    /// ```
    ExecDProgramOutputKey,
    ExecDProgramOutputKeyError,
    r"^[A-Za-z0-9_-]+$"
);

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn exec_d_program_output_key_validation_valid() {
        assert!("FOO".parse::<ExecDProgramOutputKey>().is_ok());
        assert!("foo".parse::<ExecDProgramOutputKey>().is_ok());
        assert!("FOO_BAR".parse::<ExecDProgramOutputKey>().is_ok());
        assert!("foo_bar".parse::<ExecDProgramOutputKey>().is_ok());
        assert!("123".parse::<ExecDProgramOutputKey>().is_ok());
        assert!("FOO-bar".parse::<ExecDProgramOutputKey>().is_ok());
        assert!("foo-BAR".parse::<ExecDProgramOutputKey>().is_ok());
    }

    #[test]
    fn exec_d_program_output_key_validation_invalid() {
        assert_eq!(
            "FOO BAR".parse::<ExecDProgramOutputKey>(),
            Err(ExecDProgramOutputKeyError::InvalidValue(String::from(
                "FOO BAR"
            )))
        );

        assert_eq!(
            "FÜCHSCHEN".parse::<ExecDProgramOutputKey>(),
            Err(ExecDProgramOutputKeyError::InvalidValue(String::from(
                "FÜCHSCHEN"
            )))
        );

        assert_eq!(
            "🦊".parse::<ExecDProgramOutputKey>(),
            Err(ExecDProgramOutputKeyError::InvalidValue(String::from("🦊")))
        );

        assert_eq!(
            "".parse::<ExecDProgramOutputKey>(),
            Err(ExecDProgramOutputKeyError::InvalidValue(String::new()))
        );
    }
}
