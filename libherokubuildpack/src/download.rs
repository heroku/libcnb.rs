/// Unfortunately the rust-ification of buildpacks introduces a wealth of
/// incompatibilities and unexpected behavior. In original buildpacks a web
/// download [was a curl command with reasonable options](https://github.com/heroku/heroku-buildpack-nodejs/blob/76b36fcf5daa1b1aed17b3616a57f99d2ef05a29/lib/binaries.sh#L98C1-L99C1):
/// curl "$url" -L --silent --fail --retry 5 --retry-max-time 15 --retry-connrefused --connect-timeout 5 -o target
/// Unfortunately, the new rust-implementation dropped the retry, max-time and whatnot
/// and therefore things [like NodeJS downloads fail to install](https://github.com/heroku/buildpacks-nodejs/issues/868). 
/// But on the other hand, Rust provides memory safety and cross-platform compile targets.
/// This is a copilot suggestion to bring parity.
use std::{fs, io, thread, time::Duration};

#[derive(thiserror::Error, Debug)]
pub enum DownloadError {
    #[error("HTTP error while downloading file: {0}")]
    HttpError(#[from] Box<ureq::Error>),

    #[error("I/O error while downloading file: {0}")]
    IoError(#[from] std::io::Error),
}

/// Downloads a file via HTTP(S) to a local path with retry logic
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
    let max_retries = 5;
    let retry_delay = Duration::from_secs(2);
    let mut attempts = 0;

    while attempts < max_retries {
        attempts += 1;
        match attempt_download(&uri, &destination) {
            Ok(_) => return Ok(()),
            Err(e) if attempts < max_retries => {
                eprintln!("Attempt {}/{} failed: {}. Retrying in {:?}...", attempts, max_retries, e, retry_delay);
                thread::sleep(retry_delay);
            }
            Err(e) => return Err(e),
        }
    }

    Err(DownloadError::HttpError(Box::new(ureq::Error::new("Max retries reached"))))
}

fn attempt_download(
    uri: &impl AsRef<str>,
    destination: &impl AsRef<std::path::Path>,
) -> Result<(), DownloadError> {
    let response = ureq::get(uri.as_ref())
        .timeout_connect(5_000) // 5 seconds timeout
        .timeout_read(15_000) // 15 seconds max time
        .call()
        .map_err(Box::new)?;
    let mut reader = response.into_reader();
    let mut file = fs::File::create(destination.as_ref())?;
    io::copy(&mut reader, &mut file)?;

    Ok(())
}
