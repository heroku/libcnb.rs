use std::collections::HashMap;
use std::env;
use std::env::VarsOs;
use std::ffi::{OsStr, OsString};

/// Generic collection of environment variables.
///
/// # Examples
/// ```
/// use libcnb::Env;
/// use std::process::Command;
///
/// let mut env = Env::new();
/// env.insert("FOO", "BAR");
/// env.insert("BAZ", "BLAH");
///
/// let output = Command::new("printenv")
///     .env_clear()
///     .envs(&env)
///     .output()
///     .unwrap();
///
/// assert_eq!(
///     "BAZ=BLAH\nFOO=BAR\n",
///     String::from_utf8_lossy(&output.stdout)
/// );
/// ```
#[derive(Clone, Debug, Default, Eq, PartialEq)]
pub struct Env {
    inner: HashMap<OsString, OsString>,
}

impl Env {
    /// Creates a new `Env` from all the environment variables of the current process.
    ///
    /// The returned `Env` contains a snapshot of the process's environment
    /// variables at the time of this invocation. Modifications to environment
    /// variables afterwards will not be reflected in the returned value.
    ///
    /// See [`std::env::vars_os`]
    #[must_use]
    pub fn from_current() -> Self {
        env::vars_os().into()
    }

    /// Creates an empty `Env` struct.
    #[must_use]
    pub fn new() -> Self {
        Self::default()
    }

    /// Inserts a key-value pair into the environment, overriding the value if `key` was already
    /// present.
    pub fn insert(&mut self, key: impl Into<OsString>, value: impl Into<OsString>) -> &mut Self {
        self.inner.insert(key.into(), value.into());
        self
    }

    /// Returns the value corresponding to the given key.
    #[must_use]
    pub fn get(&self, key: impl AsRef<OsStr>) -> Option<&OsString> {
        self.inner.get(key.as_ref())
    }

    /// Returns the value corresponding to the given key, interpreted as Unicode data.
    ///
    /// Any non-Unicode sequences are replaced with
    /// [`U+FFFD REPLACEMENT CHARACTER`][U+FFFD].
    ///
    /// [U+FFFD]: std::char::REPLACEMENT_CHARACTER
    ///
    /// See [`OsStr::to_string_lossy`] for more details.
    #[must_use]
    pub fn get_string_lossy(&self, key: impl AsRef<OsStr>) -> Option<String> {
        self.get(key)
            .map(|os_string| os_string.to_string_lossy().to_string())
    }

    /// Returns true if the environment contains a value for the specified key.
    #[must_use]
    pub fn contains_key(&self, key: impl AsRef<OsStr>) -> bool {
        self.inner.contains_key(key.as_ref())
    }

    #[must_use]
    pub fn iter(&self) -> std::collections::hash_map::Iter<'_, OsString, OsString> {
        self.inner.iter()
    }
}

impl From<VarsOs> for Env {
    fn from(vars_os: VarsOs) -> Self {
        Self {
            inner: vars_os.collect(),
        }
    }
}

impl<'a> IntoIterator for &'a Env {
    type Item = (&'a OsString, &'a OsString);
    type IntoIter = std::collections::hash_map::Iter<'a, OsString, OsString>;

    fn into_iter(self) -> Self::IntoIter {
        self.iter()
    }
}

#[cfg(test)]
mod tests {
    #[test]
    #[cfg(target_family = "unix")]
    fn into_iterator() {
        use crate::Env;
        use std::process::Command;

        let mut env = Env::new();
        env.insert("FOO", "FOO");
        env.insert("FOO", "BAR");
        env.insert("BAZ", "BLAH");

        let output = Command::new("printenv")
            .env_clear()
            .envs(&env)
            .output()
            .unwrap();

        assert_eq!(
            "BAZ=BLAH\nFOO=BAR\n",
            String::from_utf8_lossy(&output.stdout)
        );
    }
}
