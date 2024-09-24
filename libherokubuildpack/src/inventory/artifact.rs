use crate::checksum::{Checksum, Digest};
use serde::de::DeserializeOwned;
use serde::{Deserialize, Serialize};
use std::fmt::Display;
use std::str::FromStr;

#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct Artifact<V, D, M> {
    #[serde(bound = "V: Serialize + DeserializeOwned")]
    pub version: V,
    pub os: Os,
    pub arch: Arch,
    pub url: String,
    #[serde(bound = "D: Digest")]
    pub checksum: Checksum<D>,
    #[serde(bound = "M: Serialize + DeserializeOwned")]
    pub metadata: M,
}

impl<V, D, M> PartialEq for Artifact<V, D, M>
where
    V: Eq,
    M: Eq,
{
    fn eq(&self, other: &Self) -> bool {
        self.version == other.version
            && self.os == other.os
            && self.arch == other.arch
            && self.url == other.url
            && self.checksum == other.checksum
            && self.metadata == other.metadata
    }
}

impl<V, D, M> Eq for Artifact<V, D, M>
where
    V: Eq,
    M: Eq,
{
}

#[derive(Debug, Serialize, Deserialize, Copy, Clone, Eq, PartialEq)]
#[serde(rename_all = "lowercase")]
pub enum Os {
    Darwin,
    Linux,
}

#[derive(Debug, Serialize, Deserialize, Copy, Clone, Eq, PartialEq)]
#[serde(rename_all = "lowercase")]
pub enum Arch {
    Amd64,
    Arm64,
}

impl Display for Os {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Os::Darwin => write!(f, "darwin"),
            Os::Linux => write!(f, "linux"),
        }
    }
}

#[derive(thiserror::Error, Debug)]
#[error("OS is not supported: {0}")]
pub struct UnsupportedOsError(String);

impl FromStr for Os {
    type Err = UnsupportedOsError;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        match s {
            "linux" => Ok(Os::Linux),
            "darwin" | "osx" => Ok(Os::Darwin),
            _ => Err(UnsupportedOsError(s.to_string())),
        }
    }
}

impl Display for Arch {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Arch::Amd64 => write!(f, "amd64"),
            Arch::Arm64 => write!(f, "arm64"),
        }
    }
}

#[derive(thiserror::Error, Debug)]
#[error("Arch is not supported: {0}")]
pub struct UnsupportedArchError(String);

impl FromStr for Arch {
    type Err = UnsupportedArchError;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        match s {
            "amd64" | "x86_64" => Ok(Arch::Amd64),
            "arm64" | "aarch64" => Ok(Arch::Arm64),
            _ => Err(UnsupportedArchError(s.to_string())),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::version::VersionRequirement;

    #[test]
    fn test_arch_display_format() {
        let archs = [(Arch::Amd64, "amd64"), (Arch::Arm64, "arm64")];

        for (input, expected) in archs {
            assert_eq!(expected, input.to_string());
        }
    }

    #[test]
    fn test_arch_parsing() {
        let archs = [
            ("amd64", Arch::Amd64),
            ("arm64", Arch::Arm64),
            ("x86_64", Arch::Amd64),
            ("aarch64", Arch::Arm64),
        ];
        for (input, expected) in archs {
            assert_eq!(expected, input.parse::<Arch>().unwrap());
        }

        assert!(matches!(
            "foo".parse::<Arch>().unwrap_err(),
            UnsupportedArchError(..)
        ));
    }

    #[test]
    fn test_os_display_format() {
        assert_eq!("linux", Os::Linux.to_string());
    }

    #[test]
    fn test_os_parsing() {
        assert_eq!(Os::Linux, "linux".parse::<Os>().unwrap());
        assert_eq!(Os::Darwin, "darwin".parse::<Os>().unwrap());
        assert_eq!(Os::Darwin, "osx".parse::<Os>().unwrap());

        assert!(matches!(
            "foo".parse::<Os>().unwrap_err(),
            UnsupportedOsError(..)
        ));
    }

    impl VersionRequirement<String> for String {
        fn satisfies(&self, version: &String) -> bool {
            self == version
        }
    }
}
