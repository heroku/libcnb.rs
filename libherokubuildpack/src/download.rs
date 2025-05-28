use http::{HeaderName, header::CONTENT_LENGTH};
use std::num::ParseIntError;
use std::{fs, io};

#[derive(thiserror::Error, Debug)]
pub enum DownloadError {
    // Boxed to prevent `large_enum_variant` errors since `ureq::Error` is massive.
    #[error("HTTP error while downloading file: {0}")]
    HttpError(#[from] Box<ureq::Error>),

    #[error("I/O error while downloading file: {0}")]
    IoError(#[from] std::io::Error),

    #[error("Missing required header: `{0}`")]
    MissingRequiredHeader(HeaderName),

    #[error("Failed to convert header value for `{0}` into a string")]
    HeaderEncodingError(HeaderName),

    #[error("Cannot parse into an integer: {0}")]
    CannotParseInteger(ParseIntError),

    #[error("Expected `{expected}` bytes received `{received}`")]
    UnexpectedBytes {
        expected: u64,
        received: u64,
        path_deleted: Result<(), std::io::Error>,
    },
}

impl DownloadError {
    /// Do not suggest a retry if the failure is due to disk error or if the path could not be cleaned
    pub fn retry_suggested(&self) -> bool {
        !matches!(
            self,
            DownloadError::UnexpectedBytes {
                path_deleted: Err(_),
                ..
            } | DownloadError::IoError(_)
        )
    }
}

/// Downloads a file via HTTP(S) to a local path
///
/// # Examples
/// ```
/// use libherokubuildpack::digest::sha256;
/// use libherokubuildpack::download::download_file;
/// use tempfile::tempdir;
///
/// let temp_dir = tempdir().unwrap();
/// let temp_file = temp_dir.path().join("result.bin");
///
/// download_file("https://example.com/", &temp_file).unwrap();
/// assert_eq!(
///     sha256(&temp_file).unwrap(),
///     "ea8fac7c65fb589b0d53560f5251f74f9e9b243478dcb6b3ea79b5e36449c8d9"
/// );
/// ```
pub fn download_file(
    uri: impl AsRef<str>,
    destination: impl AsRef<std::path::Path>,
) -> Result<(), DownloadError> {
    let response = ureq::get(uri.as_ref()).call().map_err(Box::new)?;
    let expected: u64 = response
        .headers()
        .get(CONTENT_LENGTH)
        .ok_or_else(|| DownloadError::MissingRequiredHeader(CONTENT_LENGTH))?
        .to_str()
        .map_err(|_| DownloadError::HeaderEncodingError(CONTENT_LENGTH))?
        .parse()
        .map_err(DownloadError::CannotParseInteger)?;

    let mut file = fs::File::create(destination.as_ref())?;
    let received = io::copy(&mut response.into_body().into_reader(), &mut file)?;
    // Ensure file is closed
    drop(file);
    if received == expected {
        Ok(())
    } else {
        let path_deleted = fs::remove_file(&destination);
        Err(DownloadError::UnexpectedBytes {
            expected,
            received,
            path_deleted,
        })
    }
}
