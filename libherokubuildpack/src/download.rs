use crate::http::{HttpError, RequestLogger, ResponseExt, get};
use std::path::Path;

#[deprecated(
    note = "This has been replaced by `HttpError` and currently does nothing other than wrap `libherokubuildpack::HttpError`"
)]
#[derive(thiserror::Error, Debug)]
pub enum DownloadError {
    #[error(transparent)]
    HttpError(HttpError),
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
#[deprecated(note = "Use `libherokubuildpack::http::get(uri)
    .request_logger(libherokubuildpack::http::RequestLogger { ... })
    .call_sync()
    .and_then(|res| res.download_to_file_sync(destination)` instead")]
#[allow(deprecated)]
pub fn download_file(
    uri: impl AsRef<str>,
    destination: impl AsRef<Path>,
) -> Result<(), DownloadError> {
    get(uri.as_ref())
        // uses a no-op request logger since the previous implementation did not log anything
        .request_logger(RequestLogger {
            on_request_start: Box::new(|_| ()),
            on_request_end: Box::new(|(), _| {}),
        })
        .call_sync()
        .and_then(|res| res.download_to_file_sync(destination))
        .map_err(DownloadError::HttpError)
}
