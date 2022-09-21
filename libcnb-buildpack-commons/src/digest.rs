use std::fs;
use std::io::Read;
use std::path::Path;

use sha2::{Digest, Sha256};

/// Obtains the SHA256 checksum of a file as a hex string
///
/// # Examples
/// ```
/// use libcnb_buildpack_commons::digest::sha256;
/// use std::fs::write;
/// use tempfile::tempdir;
///
/// let temp_dir = tempdir().unwrap();
/// let temp_file = temp_dir.path().join("test.txt");
///
/// write(&temp_file, "Hello World!").unwrap();
/// let sha256_sum = sha256(&temp_file).unwrap();
/// assert_eq!(sha256_sum, "7f83b1657ff1fc53b92dc18148a1d65dfc2d4b1fa3d677284addd200126d9069");
/// ```
pub fn sha256(path: impl AsRef<Path>) -> Result<String, std::io::Error> {
    let mut file = fs::File::open(path.as_ref())?;
    let mut buffer = [0x00; 10 * 1024];
    let mut sha256: Sha256 = sha2::Sha256::default();

    let mut read = file.read(&mut buffer)?;
    while read > 0 {
        Digest::update(&mut sha256, &buffer[..read]);
        read = file.read(&mut buffer)?;
    }

    Ok(format!("{:x}", sha256.finalize()))
}
