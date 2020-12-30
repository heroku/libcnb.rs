use crate::Error as CrateError;
use serde::{de::DeserializeOwned, Serialize};
use std::{error::Error, fmt::Display, fs, path::Path};

pub fn write_toml_file(value: &impl Serialize, path: impl AsRef<Path>) -> Result<(), CrateError> {
    fs::write(path, toml::to_string(value)?)?;

    Ok(())
}

pub fn read_toml_file<A: DeserializeOwned>(path: impl AsRef<Path>) -> Result<A, CrateError> {
    let contents = fs::read_to_string(path)?;
    Ok(toml::from_str(&contents)?)
}

pub trait BuildpackError: Display {}

impl<A: Error> BuildpackError for A {}
