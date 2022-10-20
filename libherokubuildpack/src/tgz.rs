use flate2::read::GzDecoder;
use sha2::{
    digest::{generic_array::GenericArray, OutputSizeUser},
    Digest, Sha256,
};
use std::path::PathBuf;
use std::{io::Read, path::StripPrefixError};
use tar::Archive;

#[derive(Debug, Clone, Default)]
pub struct Fetcher {
    remote_uri: String,
    dest_dir: std::path::PathBuf,
    strip_prefix: Option<String>,
    filter_prefixes: Option<Vec<String>>,
    verify_digest: Option<String>,
}

impl Fetcher {
    #[must_use]
    pub fn new<S: Into<String>, P: Into<PathBuf>>(remote_uri: S, dest_dir: P) -> Self {
        Self {
            remote_uri: remote_uri.into(),
            dest_dir: dest_dir.into(),
            strip_prefix: Option::default(),
            filter_prefixes: Option::default(),
            verify_digest: Option::default(),
        }
    }

    #[must_use]
    pub fn verify_digest<S: Into<String>>(&mut self, digest: S) -> &mut Self {
        self.verify_digest = Some(digest.into());
        self
    }

    #[must_use]
    pub fn filter_prefixes<I: IntoIterator<Item = S>, S: Into<String>>(
        &mut self,
        prefixes: I,
    ) -> &mut Self {
        let mut filter_prefixes: Vec<String> = vec![];
        for prefix in prefixes {
            filter_prefixes.push(prefix.into());
        }
        self.filter_prefixes = Some(filter_prefixes);
        self
    }

    #[must_use]
    pub fn strip_prefix<S: Into<String>>(&mut self, strip_prefix: S) -> &mut Self {
        self.strip_prefix = Some(strip_prefix.into());
        self
    }

    /// Fetches a tarball from a url, strips component paths, filters path prefixes,
    /// extracts files to a location, and verifies a sha256 checksum. Care is taken
    /// not to write temporary files or read the entire contents into memory. In an
    /// error scenario, any archive contents already extracted will not be removed.
    ///
    /// # Errors
    ///
    /// See `Error` for an enumeration of error scenarios.
    pub fn fetch(&self) -> Result<(), Error> {
        let body = ureq::get(&self.remote_uri)
            .call()
            .map_err(Box::new)?
            .into_reader();
        let mut archive = Archive::new(GzDecoder::new(DigestingReader::new(body, Sha256::new())));
        for entry in archive.entries().map_err(Error::Entries)? {
            let mut file = entry.map_err(Error::Entry)?;
            let path = file.path().map_err(Error::Path)?.into_owned();

            let target_path = self.dest_dir.join(
                self.strip_prefix
                    .as_ref()
                    .map(|prefix| path.strip_prefix(prefix))
                    .unwrap_or(Ok(&path))
                    .map_err(Error::Prefix)?,
            );
            if self.filter_prefixes.as_ref().map_or(true, |prefixes| {
                prefixes
                    .iter()
                    .any(|prefix| target_path.starts_with(self.dest_dir.join(prefix)))
            }) {
                file.unpack(&target_path).map_err(Error::Unpack)?;
            }
        }
        let actual_digest = format!("{:x}", archive.into_inner().into_inner().finalize());
        self.verify_digest.as_ref().map_or(Ok(()), |verify_digest| {
            (verify_digest == &actual_digest)
                .then_some(())
                .ok_or_else(|| Error::Checksum(verify_digest.clone(), actual_digest))
        })
    }
}

struct DigestingReader<R: Read, H: sha2::Digest> {
    r: R,
    h: H,
}

impl<R: Read, H: sha2::Digest> DigestingReader<R, H> {
    pub fn new(reader: R, hasher: H) -> DigestingReader<R, H> {
        DigestingReader {
            r: reader,
            h: hasher,
        }
    }
    pub fn finalize(self) -> GenericArray<u8, <H as OutputSizeUser>::OutputSize> {
        self.h.finalize()
    }
}

impl<R: Read, H: sha2::Digest> Read for DigestingReader<R, H> {
    fn read(&mut self, buf: &mut [u8]) -> std::io::Result<usize> {
        let n = self.r.read(buf)?;
        self.h.update(&buf[..n]);
        Ok(n)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_fetch_strip_filter_extract_verify() {
        let dest = tempfile::tempdir()
            .expect("Couldn't create test tmpdir")
            .into_path();

        Fetcher::new(
            "https://mirrors.edge.kernel.org/pub/software/scm/git/git-0.01.tar.gz",
            dest.clone(),
        )
        .strip_prefix("git-0.01")
        .filter_prefixes(vec!["README"])
        .verify_digest("9bdf8a4198b269c5cbe4263b1f581aae885170a6cb93339a2033cb468e57dcd3")
        .fetch()
        .expect("Expected to fetch, strip, filter, extract, and verify");

        let diff_path = dest.join("show-diff.c");
        assert!(
            !diff_path.exists(),
            "expeted show-diff.c to not exist at {diff_path:?}"
        );

        let readme_path = dest.join("README");
        assert!(
            readme_path.exists(),
            "expected readme to exist at {readme_path:?}"
        );
    }

    #[test]
    fn test_fetch_extract() {
        let dest = tempfile::tempdir()
            .expect("Couldn't create test tmpdir")
            .into_path();

        Fetcher::new(
            "https://mirrors.edge.kernel.org/pub/software/scm/git/git-0.01.tar.gz",
            dest.clone(),
        )
        .fetch()
        .expect("Expected to fetch and extract");

        let diff_path = dest.join("git-0.01").join("show-diff.c");
        assert!(
            diff_path.exists(),
            "expeted show-diff.c to exist at {diff_path:?}"
        );

        let readme_path = dest.join("git-0.01").join("README");
        assert!(
            readme_path.exists(),
            "expected readme to exist at {readme_path:?}"
        );
    }
}

#[derive(thiserror::Error, Debug)]
pub enum Error {
    #[error("HTTP error while fetching archive: {0}")]
    Http(#[from] Box<ureq::Error>),

    #[error("Error reading archive entries: {0}")]
    Entries(std::io::Error),

    #[error("Error reading archive entry: {0}")]
    Entry(std::io::Error),

    #[error("Error reading archive file path: {0}")]
    Path(std::io::Error),

    #[error("Failed to validate archive checksum; verify {0}, but found {1}")]
    Checksum(String, String),

    #[error("Error writing archive entry: {0}")]
    Unpack(std::io::Error),

    #[error("Error stripping archive entry prefix: {0}")]
    Prefix(StripPrefixError),
}
