use serde::{de::DeserializeOwned, Serialize};
use std::{error::Error, fmt::Display, fs, io, path::Path};

pub fn write_toml_file(value: &impl Serialize, path: impl AsRef<Path>) -> io::Result<()> {
    // TODO: Fix Result type, remove unwrap
    fs::write(path, toml::to_string(value).unwrap())
}

pub fn read_toml_file<A: DeserializeOwned>(path: impl AsRef<Path>) -> io::Result<A> {
    // TODO: Fix Result type, remove unwrap
    let file_contents = fs::read_to_string(path)?;
    Ok(toml::from_str(file_contents.as_str()).unwrap())
}

pub trait BuildpackError: Display {}

impl<A: Error> BuildpackError for A {}
