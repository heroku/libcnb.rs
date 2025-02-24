use serde::{Serialize, de::DeserializeOwned};
use std::{fs, path::Path};

/// An error that occurred during reading or writing a TOML file.
#[derive(thiserror::Error, Debug)]
pub enum TomlFileError {
    #[error("I/O error while reading/writing TOML file: {0}")]
    IoError(#[from] std::io::Error),

    #[error("TOML deserialization error while reading TOML file: {0}")]
    TomlDeserializationError(#[from] toml::de::Error),

    #[error("TOML serialization error while writing TOML file: {0}")]
    TomlSerializationError(#[from] toml::ser::Error),
}

/// Serializes the given value as TOML and writes it to the given file path.
///
/// # Errors
///
/// Will return `Err` if the file couldn't be written or the value couldn't be serialized as a TOML string.
pub fn write_toml_file(
    value: &impl Serialize,
    path: impl AsRef<Path>,
) -> Result<(), TomlFileError> {
    fs::write(path, toml::to_string(value)?)?;

    Ok(())
}

/// Reads the file at the given path and parses it as `A`.
///
/// # Errors
///
/// Will return `Err` if the file couldn't be read or its contents couldn't be deserialized.
pub fn read_toml_file<A: DeserializeOwned>(path: impl AsRef<Path>) -> Result<A, TomlFileError> {
    let contents = fs::read_to_string(path)?;
    Ok(toml::from_str(&contents)?)
}
