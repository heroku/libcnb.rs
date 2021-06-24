use serde::{de::DeserializeOwned, Serialize};
use std::{error::Error, fmt::Display, fs, path::Path};

#[derive(thiserror::Error, Debug)]
pub enum TomlFileError {
    #[error("IO error while reading/writing TOML file: {0}")]
    IoError(#[from] std::io::Error),

    #[error("TOML deserialization error while reading TOML file: {0}")]
    TomlDeserializationError(#[from] toml::de::Error),

    #[error("TOML serialization error while writing TOML file: {0}")]
    TomlSerializationError(#[from] toml::ser::Error),
}

pub fn write_toml_file(
    value: &impl Serialize,
    path: impl AsRef<Path>,
) -> Result<(), TomlFileError> {
    fs::write(path, toml::to_string(value)?)?;

    Ok(())
}

pub fn read_toml_file<A: DeserializeOwned>(path: impl AsRef<Path>) -> Result<A, TomlFileError> {
    let contents = fs::read_to_string(path)?;
    Ok(toml::from_str(&contents)?)
}

pub trait BuildpackError: Display {}

impl<A: Error> BuildpackError for A {}
