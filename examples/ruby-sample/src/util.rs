use flate2::read::GzDecoder;
use sha2::Digest;
use std::fs;
use std::io;
use std::path::Path;
use std::process::{Command, ExitStatus};
use tar::Archive;

pub fn download(uri: impl AsRef<str>, destination: impl AsRef<Path>) -> Result<(), DownloadError> {
    let mut response_reader = ureq::get(uri.as_ref())
        .call()
        .map_err(|err| DownloadError::RequestError(Box::new(err)))?
        .into_reader();

    let mut destination_file = fs::File::create(destination.as_ref())
        .map_err(DownloadError::CouldNotCreateDestinationFile)?;

    io::copy(&mut response_reader, &mut destination_file)
        .map_err(DownloadError::CouldNotWriteDestinationFile)?;

    Ok(())
}

#[derive(Debug)]
pub enum DownloadError {
    // Boxed to prevent `large_enum_variant` errors since `ureq::Error` is massive.
    RequestError(Box<ureq::Error>),
    CouldNotCreateDestinationFile(std::io::Error),
    CouldNotWriteDestinationFile(std::io::Error),
}

pub fn untar(path: impl AsRef<Path>, destination: impl AsRef<Path>) -> Result<(), UntarError> {
    let file = fs::File::open(path.as_ref()).map_err(UntarError::CouldNotOpenFile)?;

    Archive::new(GzDecoder::new(file))
        .unpack(destination.as_ref())
        .map_err(UntarError::CouldNotUnpack)
}

#[derive(Debug)]
pub enum UntarError {
    CouldNotOpenFile(std::io::Error),
    CouldNotUnpack(std::io::Error),
}

pub fn sha256_checksum(path: impl AsRef<Path>) -> Result<String, std::io::Error> {
    fs::read(path).map(|bytes| format!("{:x}", sha2::Sha256::digest(bytes)))
}

/// Helper to run very simple commands where we just need to handle IO errors and non-zero exit
/// codes. Not very useful in complex scenarios, but can cut down the amount of code in simple
/// cases.
pub fn run_simple_command<E, F: FnOnce(std::io::Error) -> E, F2: FnOnce(ExitStatus) -> E>(
    command: &mut Command,
    io_error_fn: F,
    exit_status_fn: F2,
) -> Result<ExitStatus, E> {
    command
        .spawn()
        .and_then(|mut child| child.wait())
        .map_err(io_error_fn)
        .and_then(|exit_status| {
            if exit_status.success() {
                Ok(exit_status)
            } else {
                Err(exit_status_fn(exit_status))
            }
        })
}
