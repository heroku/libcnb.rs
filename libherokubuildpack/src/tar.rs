use flate2::read::GzDecoder;
use std::fs::File;
use std::io::Seek;
use std::path::Path;
use tar::Archive;

/// Decompresses and untars a given .tar.gz file to the given directory.
pub fn decompress_tarball(
    tarball: &mut File,
    destination: impl AsRef<Path>,
) -> Result<(), std::io::Error> {
    tarball.rewind()?;
    let mut archive = Archive::new(GzDecoder::new(tarball));
    archive.unpack(destination)
}
