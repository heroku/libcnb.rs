pub mod artifact;
pub mod checksum;
pub mod version;

#[cfg(feature = "inventory-semver")]
mod semver;
#[cfg(feature = "inventory-sha2")]
mod sha2;
mod unit;

use crate::inventory::artifact::{Arch, Artifact, Os};
use crate::inventory::checksum::Digest;
use crate::inventory::version::ArtifactRequirement;
use serde::de::DeserializeOwned;
use serde::{Deserialize, Serialize};
use std::cmp::Ordering;
use std::fmt::Formatter;
use std::str::FromStr;

/// Represents an inventory of artifacts.
#[derive(Debug, Serialize, Deserialize)]
pub struct Inventory<V, D, M> {
    #[serde(bound = "V: Serialize + DeserializeOwned, D: Digest, M: Serialize + DeserializeOwned")]
    pub artifacts: Vec<Artifact<V, D, M>>,
}

impl<V, D, M> Default for Inventory<V, D, M> {
    fn default() -> Self {
        Self { artifacts: vec![] }
    }
}

impl<V, D, M> Inventory<V, D, M> {
    #[must_use]
    pub fn new() -> Self {
        Self::default()
    }

    pub fn push(&mut self, artifact: Artifact<V, D, M>) {
        self.artifacts.push(artifact);
    }

    pub fn resolve<R>(&self, os: Os, arch: Arch, requirement: &R) -> Option<&Artifact<V, D, M>>
    where
        V: Ord,
        R: ArtifactRequirement<V, M>,
    {
        self.artifacts
            .iter()
            .filter(|artifact| {
                artifact.os == os
                    && artifact.arch == arch
                    && requirement.satisfies_version(&artifact.version)
                    && requirement.satisfies_metadata(&artifact.metadata)
            })
            .max_by_key(|artifact| &artifact.version)
    }

    pub fn partial_resolve<R>(
        &self,
        os: Os,
        arch: Arch,
        requirement: &R,
    ) -> Option<&Artifact<V, D, M>>
    where
        V: PartialOrd,
        R: ArtifactRequirement<V, M>,
    {
        #[inline]
        fn partial_max_by_key<I, F, A>(iterator: I, f: F) -> Option<I::Item>
        where
            I: Iterator,
            F: Fn(&I::Item) -> A,
            A: PartialOrd,
        {
            iterator.fold(None, |acc, item| match acc {
                None => Some(item),
                Some(acc) => match f(&item).partial_cmp(&f(&acc)) {
                    Some(Ordering::Greater | Ordering::Equal) => Some(item),
                    None | Some(Ordering::Less) => Some(acc),
                },
            })
        }

        partial_max_by_key(
            self.artifacts.iter().filter(|artifact| {
                artifact.os == os
                    && artifact.arch == arch
                    && requirement.satisfies_version(&artifact.version)
                    && requirement.satisfies_metadata(&artifact.metadata)
            }),
            |artifact| &artifact.version,
        )
    }
}

#[derive(thiserror::Error, Debug)]
pub enum ParseInventoryError {
    #[error("TOML parsing error: {0}")]
    TomlError(toml::de::Error),
}

impl<V, D, M> FromStr for Inventory<V, D, M>
where
    V: Serialize + DeserializeOwned,
    D: Digest,
    M: Serialize + DeserializeOwned,
{
    type Err = ParseInventoryError;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        toml::from_str(s).map_err(ParseInventoryError::TomlError)
    }
}

impl<V, D, M> std::fmt::Display for Inventory<V, D, M>
where
    V: Serialize + DeserializeOwned,
    D: Digest,
    M: Serialize + DeserializeOwned,
{
    #![allow(clippy::unwrap_used)]
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        f.write_str(&toml::to_string(self).unwrap())
    }
}

#[cfg(test)]
mod test {
    use crate::inventory::artifact::{Arch, Artifact, Os};
    use crate::inventory::checksum::tests::BogusDigest;
    use crate::inventory::Inventory;

    #[test]
    fn test_matching_artifact_resolution() {
        let mut inventory = Inventory::new();
        inventory.push(create_artifact("foo", Os::Linux, Arch::Arm64));

        assert_eq!(
            "foo",
            &inventory
                .resolve(Os::Linux, Arch::Arm64, &String::from("foo"))
                .expect("should resolve matching artifact")
                .version,
        );
    }

    #[test]
    fn test_dont_resolve_artifact_with_wrong_arch() {
        let mut inventory = Inventory::new();
        inventory.push(create_artifact("foo", Os::Linux, Arch::Arm64));

        assert!(inventory
            .resolve(Os::Linux, Arch::Amd64, &String::from("foo"))
            .is_none());
    }

    #[test]
    fn test_dont_resolve_artifact_with_wrong_version() {
        let mut inventory = Inventory::new();
        inventory.push(create_artifact("foo", Os::Linux, Arch::Arm64));

        assert!(inventory
            .resolve(Os::Linux, Arch::Arm64, &String::from("bar"))
            .is_none());
    }

    fn create_artifact(version: &str, os: Os, arch: Arch) -> Artifact<String, BogusDigest, ()> {
        Artifact {
            version: String::from(version),
            os,
            arch,
            url: "https://example.com".to_string(),
            checksum: BogusDigest::checksum("cafebabe"),
            metadata: (),
        }
    }
}
