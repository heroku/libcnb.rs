use hex::FromHexError;
use serde::{Deserialize, Serialize};
use std::marker::PhantomData;
use std::str::FromStr;

#[derive(Debug, Clone, Eq)]
pub struct Checksum<D> {
    pub name: String,
    pub value: Vec<u8>,
    digest: PhantomData<D>,
}

impl<D> PartialEq for Checksum<D> {
    fn eq(&self, other: &Self) -> bool {
        (self.name == other.name) && (self.value == other.value)
    }
}

#[derive(thiserror::Error, Debug, PartialEq)]
pub enum ChecksumParseError {
    #[error("Checksum prefix is missing")]
    MissingPrefix,
    #[error("Checksum prefix \"{0}\" is incompatible")]
    IncompatiblePrefix(String),
    #[error("Checksum value cannot be parsed as hex string: {0}")]
    InvalidValue(FromHexError),
    #[error("Checksum value length {0} is invalid")]
    InvalidChecksumLength(usize),
}

impl<D> FromStr for Checksum<D>
where
    D: Digest,
{
    type Err = ChecksumParseError;

    fn from_str(value: &str) -> Result<Self, Self::Err> {
        let (name, value) = value
            .split_once(':')
            .ok_or(ChecksumParseError::MissingPrefix)
            .and_then(|(key, value)| {
                hex::decode(value)
                    .map_err(ChecksumParseError::InvalidValue)
                    .map(|value| (String::from(key), value))
            })?;

        if !D::name_compatible(&name) {
            Err(ChecksumParseError::IncompatiblePrefix(name))
        } else if !D::length_compatible(value.len()) {
            Err(ChecksumParseError::InvalidChecksumLength(value.len()))
        } else {
            Ok(Checksum {
                name,
                value,
                digest: PhantomData,
            })
        }
    }
}

pub trait Digest {
    fn name_compatible(name: &str) -> bool;
    fn length_compatible(len: usize) -> bool;
}

impl<D> Serialize for Checksum<D>
where
    D: Digest,
{
    fn serialize<T>(&self, serializer: T) -> Result<T::Ok, T::Error>
    where
        T: serde::Serializer,
    {
        serializer.serialize_str(&format!("{}:{}", self.name, hex::encode(&self.value)))
    }
}

impl<'de, D> Deserialize<'de> for Checksum<D>
where
    D: Digest,
{
    fn deserialize<T>(deserializer: T) -> Result<Self, T::Error>
    where
        T: serde::Deserializer<'de>,
    {
        String::deserialize(deserializer)
            .and_then(|string| string.parse::<Self>().map_err(serde::de::Error::custom))
    }
}

#[cfg(test)]
pub(crate) mod tests {
    use super::*;
    use serde_test::{assert_de_tokens_error, assert_tokens, Token};

    #[derive(Debug)]
    pub(crate) struct BogusDigest;

    impl BogusDigest {
        pub(crate) fn checksum(hex_string: &str) -> Checksum<Self> {
            Checksum {
                name: String::from("bogus"),
                value: hex::decode(hex_string).unwrap(),
                digest: Default::default(),
            }
        }
    }

    impl Digest for BogusDigest {
        fn name_compatible(name: &str) -> bool {
            name == "bogus"
        }

        fn length_compatible(len: usize) -> bool {
            len == 4
        }
    }

    #[test]
    fn test_checksum_serialization() {
        assert_tokens(
            &BogusDigest::checksum("cafebabe"),
            &[Token::BorrowedStr("bogus:cafebabe")],
        );
    }

    #[test]
    fn test_invalid_checksum_deserialization() {
        assert_de_tokens_error::<Checksum<BogusDigest>>(
            &[Token::BorrowedStr("baz:cafebabe")],
            "Checksum prefix \"baz\" is incompatible",
        );
    }

    #[test]
    fn test_invalid_checksum_size() {
        assert_eq!(
            "bogus:123456".parse::<Checksum<BogusDigest>>(),
            Err(ChecksumParseError::InvalidChecksumLength(3))
        );
    }

    #[test]
    fn test_invalid_hex_input() {
        assert!(matches!(
            "bogus:quux".parse::<Checksum<BogusDigest>>(),
            Err(ChecksumParseError::InvalidValue(
                FromHexError::InvalidHexCharacter { c: 'q', index: 0 }
            ))
        ));
    }

    #[test]
    fn test_checksum_parse_and_serialize() {
        let checksum = "bogus:cafebabe".parse::<Checksum<BogusDigest>>().unwrap();
        assert_tokens(&checksum, &[Token::BorrowedStr("bogus:cafebabe")]);
    }
}
