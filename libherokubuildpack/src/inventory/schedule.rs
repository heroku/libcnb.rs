use crate::inventory::version::VersionRequirement;
use serde::de::DeserializeOwned;
use serde::{Deserialize, Serialize};
use std::fmt::Formatter;
use std::str::FromStr;

/// Represents a release schedule for a set of version requirements.
///
/// A schedule maps version requirements to release lifecycle data such as end-of-life
/// dates. It can be used by a buildpack to determine support status for a resolved version.
///
/// The schedule can be manipulated in-memory and then re-serialized to disk to facilitate
/// both reading and writing schedule files.
///
/// # Example
///
/// ```rust
/// use libherokubuildpack::inventory::schedule::{Schedule, Release};
/// use semver::{Version, VersionReq};
///
/// // Create a release and add it to a schedule
/// let new_release = Release {
///     requirement: VersionReq::parse("^1.0").unwrap(),
///     end_of_life: "2025-01-01".to_string(),
///     metadata: None,
/// };
/// let mut schedule = Schedule::<VersionReq, String, Option<()>>::new();
/// schedule.push(new_release.clone());
///
/// // Serialize the schedule to TOML
/// let schedule_toml = schedule.to_string();
/// assert_eq!(
///     r#"[[releases]]
/// requirement = "^1.0"
/// end_of_life = "2025-01-01"
/// "#,
///     schedule_toml
/// );
///
/// // Deserialize the schedule from TOML
/// let parsed_schedule = schedule_toml
///     .parse::<Schedule<VersionReq, String, Option<()>>>()
///     .unwrap();
///
/// // Resolve a release for a given version
/// let resolved_release = parsed_schedule.resolve(&Version::new(1, 2, 3)).unwrap();
/// assert_eq!(resolved_release.end_of_life, "2025-01-01");
/// ```
#[derive(Debug, Serialize, Deserialize)]
pub struct Schedule<R, E, M> {
    pub releases: Vec<Release<R, E, M>>,
}

/// A single entry in a [`Schedule`], covering versions that match `requirement`.
///
/// Each release carries an end-of-life value that can be used to determine
/// when a release is no longer supported.
///
/// Metadata can be used to store additional information about the release.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Release<R, E, M> {
    pub requirement: R,
    pub end_of_life: E,
    pub metadata: M,
}

impl<R, E, M> Default for Schedule<R, E, M> {
    fn default() -> Self {
        Self { releases: vec![] }
    }
}

impl<R, E, M> Schedule<R, E, M> {
    /// Creates a new empty schedule
    #[must_use]
    pub fn new() -> Self {
        Self::default()
    }

    /// Add a new release to the in-memory schedule
    pub fn push(&mut self, release: Release<R, E, M>) {
        self.releases.push(release);
    }

    /// Return a single release as the first match for the given version.
    ///
    /// If multiple releases match the version, the first one in declaration order is returned.
    /// This differs from [`Inventory::resolve`](super::Inventory::resolve), which returns the
    /// highest matching version.
    pub fn resolve<V>(&self, version: &V) -> Option<&Release<R, E, M>>
    where
        R: VersionRequirement<V>,
    {
        self.releases
            .iter()
            .find(|release| release.requirement.satisfies(version))
    }
}

#[derive(thiserror::Error, Debug)]
pub enum ParseScheduleError {
    #[error("TOML parsing error: {0}")]
    TomlError(toml::de::Error),
}

impl<R, E, M> FromStr for Schedule<R, E, M>
where
    R: DeserializeOwned,
    E: DeserializeOwned,
    M: DeserializeOwned,
{
    type Err = ParseScheduleError;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        toml::from_str(s).map_err(ParseScheduleError::TomlError)
    }
}

impl<R, E, M> std::fmt::Display for Schedule<R, E, M>
where
    R: Serialize,
    E: Serialize,
    M: Serialize,
{
    #![allow(clippy::unwrap_used)]
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        f.write_str(&toml::to_string(self).unwrap())
    }
}

#[cfg(test)]
mod test {
    use super::*;

    #[test]
    fn test_matching_release_resolution() {
        let mut schedule = Schedule::new();
        schedule.push(create_release("v1", "2025-01-01"));
        schedule.push(create_release("v2", "2026-01-01"));

        assert_eq!(
            "2026-01-01",
            &schedule
                .resolve(&String::from("v2"))
                .expect("should resolve matching release")
                .end_of_life,
        );
    }

    #[test]
    fn test_dont_resolve_release_with_wrong_version() {
        let mut schedule = Schedule::new();
        schedule.push(create_release("v1", "2025-01-01"));

        assert!(schedule.resolve(&String::from("v9")).is_none());
    }

    #[test]
    fn test_resolve_returns_first_match() {
        let mut schedule = Schedule::new();
        schedule.push(create_release("v1", "first"));
        schedule.push(create_release("v1", "second"));

        assert_eq!(
            "first",
            &schedule
                .resolve(&String::from("v1"))
                .expect("should resolve matching release")
                .end_of_life,
        );
    }

    fn create_release(requirement: &str, eol: &str) -> Release<String, String, Option<()>> {
        Release {
            requirement: requirement.to_string(),
            end_of_life: eol.to_string(),
            metadata: None,
        }
    }
}
