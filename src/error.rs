#[derive(thiserror::Error, Debug)]
pub enum Error {
    #[error("Found `{0}` but value MUST be in form <major>.<minor> or <major>, where <major> is equivalent to <major>.0.")]
    InvalidBuildpackApi(String),
    #[error("Found `{0}` but value MUST only contain numbers, letters, and the characters ., /, and -. Value MUST NOT be 'config' or 'app'.")]
    InvalidBuildpackId(String),
    #[error(
        "Found `{0}` but value MUST only contain numbers, letters, and the characters ., _, and -."
    )]
    InvalidProcessType(String),
    #[error(
        "Found `{0}` but value MUST only contain numbers, letters, and the characters ., /, and -."
    )]
    InvalidStackId(String),
    #[error("could not serialize into TOML")]
    TomlSerError(#[from] toml::ser::Error),
    #[error("could not deserialize from TOML")]
    TomlDeError(#[from] toml::de::Error),
    #[error("I/O Error: {0}")]
    IoError(#[from] std::io::Error),
}
