use std::{fs, io};

#[derive(thiserror::Error, Debug)]
pub enum DownloadError {
    // Boxed to prevent `large_enum_variant` errors since `ureq::Error` is massive.
    #[error("HTTP error while downloading file: {0}")]
    HttpError(#[from] Box<ureq::Error>),

    #[error("IO error while downloading file: {0}")]
    IoError(#[from] std::io::Error),
}

/// Downloads a file via HTTP(S) to a local path
///
/// # Examples
/// ```
/// use libcnb_buildpack_commons::download::download_file;
/// use libcnb_buildpack_commons::digest::sha256;
/// use tempfile::tempdir;
///
/// let temp_dir = tempdir().unwrap();
/// let temp_file = temp_dir.path().join("result.bin");
///
/// download_file("https://example.com/", &temp_file).unwrap();
/// assert_eq!(sha256(&temp_file).unwrap(), "ea8fac7c65fb589b0d53560f5251f74f9e9b243478dcb6b3ea79b5e36449c8d9");
/// ```
pub fn download_file(
    uri: impl AsRef<str>,
    destination: impl AsRef<std::path::Path>,
) -> Result<(), DownloadError> {
    let response = ureq::get(uri.as_ref()).call().map_err(Box::new)?;
    let mut reader = response.into_reader();
    let mut file = fs::File::create(destination.as_ref())?;
    io::copy(&mut reader, &mut file)?;

    Ok(())
}
