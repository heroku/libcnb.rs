use crate::inventory::version::VersionRequirement;
use serde::de::DeserializeOwned;
use serde::{Deserialize, Serialize};
use std::fmt::Formatter;
use std::str::FromStr;

/// A collection of version-specific warnings that can be resolved against a version.
///
/// Warnings are matched in declaration order, and all matching warnings are returned
/// by [`resolve`](Self::resolve).
///
/// The collection can be manipulated in-memory and then re-serialized to disk to facilitate
/// both reading and writing warning files.
///
/// # Example
///
/// ```rust
/// use libherokubuildpack::inventory::warning::{VersionWarning, VersionWarnings};
/// use semver::{Version, VersionReq};
///
/// // Create a warning and add it to the collection
/// let warning = VersionWarning {
///     requirement: VersionReq::parse("^1.0").unwrap(),
///     message: "Version 1.x will reach end-of-life on 2025-01-01.".to_string(),
/// };
/// let mut warnings = VersionWarnings::<VersionReq>::new();
/// warnings.push(warning.clone());
///
/// // Serialize to TOML
/// let warnings_toml = warnings.to_string();
/// assert_eq!(
///     r#"[[warnings]]
/// requirement = "^1.0"
/// message = "Version 1.x will reach end-of-life on 2025-01-01."
/// "#,
///     warnings_toml
/// );
///
/// // Deserialize from TOML
/// let parsed = warnings_toml.parse::<VersionWarnings<VersionReq>>().unwrap();
///
/// // Resolve warnings for a given version
/// let matched = parsed.resolve(&Version::new(1, 2, 3));
/// assert_eq!(matched.len(), 1);
/// assert_eq!(
///     matched[0].message,
///     "Version 1.x will reach end-of-life on 2025-01-01."
/// );
/// ```
#[derive(Debug, Serialize, Deserialize)]
pub struct VersionWarnings<R> {
    pub warnings: Vec<VersionWarning<R>>,
}

/// A single warning associated with a version requirement.
///
/// When a resolved version satisfies the [`requirement`](Self::requirement), the
/// [`message`](Self::message) should be displayed to the user.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct VersionWarning<R> {
    pub requirement: R,
    pub message: String,
}

impl<R> Default for VersionWarnings<R> {
    fn default() -> Self {
        Self { warnings: vec![] }
    }
}

impl<R> VersionWarnings<R> {
    /// Creates a new empty collection of version warnings.
    #[must_use]
    pub fn new() -> Self {
        Self::default()
    }

    /// Add a new warning to the in-memory collection.
    pub fn push(&mut self, warning: VersionWarning<R>) {
        self.warnings.push(warning);
    }

    /// Return all warnings whose requirement matches the given version.
    ///
    /// If no warnings match, an empty vector is returned.
    pub fn resolve<V>(&self, version: &V) -> Vec<&VersionWarning<R>>
    where
        R: VersionRequirement<V>,
    {
        self.warnings
            .iter()
            .filter(|warning| warning.requirement.satisfies(version))
            .collect()
    }
}

#[derive(thiserror::Error, Debug)]
pub enum ParseWarningsError {
    #[error("TOML parsing error: {0}")]
    TomlError(toml::de::Error),
}

impl<R> FromStr for VersionWarnings<R>
where
    R: DeserializeOwned,
{
    type Err = ParseWarningsError;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        toml::from_str(s).map_err(ParseWarningsError::TomlError)
    }
}

impl<R> std::fmt::Display for VersionWarnings<R>
where
    R: Serialize,
{
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        f.write_str(&toml::to_string(self).expect("should serialize to TOML string"))
    }
}

#[cfg(test)]
mod test {
    use super::*;

    #[test]
    fn test_matching_warning_resolution() {
        let mut warnings = VersionWarnings::new();
        warnings.push(create_warning("v1", "Warning for v1"));
        warnings.push(create_warning("v2", "Warning for v2"));

        let matched = warnings.resolve(&String::from("v2"));
        assert_eq!(matched.len(), 1);
        assert_eq!(matched[0].message, "Warning for v2");
    }

    #[test]
    fn test_no_matching_warnings() {
        let mut warnings = VersionWarnings::new();
        warnings.push(create_warning("v1", "Warning for v1"));

        let matched = warnings.resolve(&String::from("v9"));
        assert!(matched.is_empty());
    }

    #[test]
    fn test_resolve_returns_all_matches() {
        let mut warnings = VersionWarnings::new();
        warnings.push(create_warning("v1", "first"));
        warnings.push(create_warning("v1", "second"));

        let matched = warnings.resolve(&String::from("v1"));
        assert_eq!(matched.len(), 2);
        assert_eq!(matched[0].message, "first");
        assert_eq!(matched[1].message, "second");
    }

    #[test]
    fn test_toml_round_trip() {
        let mut warnings = VersionWarnings::new();
        warnings.push(create_warning("v1", "Warning for v1"));

        let toml_string = warnings.to_string();
        let parsed: VersionWarnings<String> = toml_string.parse().unwrap();

        assert_eq!(parsed.warnings.len(), 1);
        assert_eq!(parsed.warnings[0].requirement, "v1");
        assert_eq!(parsed.warnings[0].message, "Warning for v1");
    }

    fn create_warning(requirement: &str, message: &str) -> VersionWarning<String> {
        VersionWarning {
            requirement: requirement.to_string(),
            message: message.to_string(),
        }
    }
}
