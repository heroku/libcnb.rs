use serde::{de::DeserializeOwned, Serialize};
use std::{fs, path::Path};

/// An error that occurred during reading or writing a TOML file.
#[derive(thiserror::Error, Debug)]
pub enum TomlFileError {
    #[error("IO error while reading/writing TOML file: {0}")]
    IoError(#[from] std::io::Error),

    #[error("TOML deserialization error while reading TOML file: {0}")]
    TomlDeserializationError(#[from] toml::de::Error),

    #[error("TOML serialization error while writing TOML file: {0}")]
    TomlSerializationError(#[from] toml::ser::Error),
}

/// # Errors
///
/// This error is a `TomlFileError` that is either an `IoError` or `TomlSerializationError` from writing a toml file.
pub fn write_toml_file(
    value: &impl Serialize,
    path: impl AsRef<Path>,
) -> Result<(), TomlFileError> {
    fs::write(path, toml::to_string(value)?)?;

    Ok(())
}

/// # Errors
///
/// This error is a `TomlFileError` that is either an `IoError` or `TomlDeserializationError` from writing a toml file.
pub fn read_toml_file<A: DeserializeOwned>(path: impl AsRef<Path>) -> Result<A, TomlFileError> {
    let contents = fs::read_to_string(path)?;
    Ok(toml::from_str(&contents)?)
}
