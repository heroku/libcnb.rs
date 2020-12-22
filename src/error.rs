#[derive(thiserror::Error, Debug, PartialEq)]
pub enum Error {
    #[error(
        "Found `{0}` but value MUST only contain numbers, letters, and the characters ., _, and -."
    )]
    InvalidProcessType(String),
    #[error("Found `{0}` but value MUST only contain numbers, letters, and the characters ., /, and -. Value MUST NOT be 'config' or 'app'.")]
    InvalidBuildpackId(String),
}
