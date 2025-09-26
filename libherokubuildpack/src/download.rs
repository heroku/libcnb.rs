use std::{fs, io};

#[derive(thiserror::Error, Debug)]
pub enum DownloadError {
    #[error("HTTP error while downloading file: {0}")]
    HttpError(#[from] reqwest::Error),

    #[error("I/O error while downloading file: {0}")]
    IoError(#[from] io::Error),
}

/// Downloads a file via HTTP(S) to a local path.
///
/// Verifies certificates with the operating system verifier to allow buildpack users to use their
/// own certificates when the buildpack makes requests. This can be useful in locked down corporate
/// environments.
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
    let client = reqwest::blocking::ClientBuilder::new()
        .use_rustls_tls()
        .build()?;

    let mut response = client.get(uri.as_ref()).send()?;
    let mut file = fs::File::create(destination.as_ref())?;

    io::copy(&mut response, &mut file)?;

    Ok(())
}

#[cfg(test)]
mod test {
    use super::download_file;
    use indoc::indoc;
    use tempfile::NamedTempFile;

    #[test]
    fn test_self_signed_certificate() {
        // Using unsafe to modify environment variables is not thread-safe but acceptable here
        // since this test needs to simulate custom certificate scenarios and is the only test
        // manipulating SSL_CERT_FILE. Since this test is testing that the function implicitly
        // gets global state from environment variables, there is no other way of testing this.
        #[allow(unsafe_code)]
        unsafe {
            std::env::remove_var("SSL_CERT_FILE");
        }

        let temp_file = NamedTempFile::new().unwrap();

        assert!(download_file("https://self-signed.badssl.com", temp_file.path()).is_err());

        let badssl_self_signed_cert_dir = tempfile::tempdir().unwrap();
        let badssl_self_signed_cert = badssl_self_signed_cert_dir
            .path()
            .join("badssl_self_signed_cert.pem");

        // https://github.com/rustls/rustls-native-certs/blob/main/tests/badssl-com-chain.pem
        std::fs::write(
            &badssl_self_signed_cert,
            indoc! { "
             -----BEGIN CERTIFICATE-----
             MIIDeTCCAmGgAwIBAgIJAMnA8BB8xT6wMA0GCSqGSIb3DQEBCwUAMGIxCzAJBgNV
             BAYTAlVTMRMwEQYDVQQIDApDYWxpZm9ybmlhMRYwFAYDVQQHDA1TYW4gRnJhbmNp
             c2NvMQ8wDQYDVQQKDAZCYWRTU0wxFTATBgNVBAMMDCouYmFkc3NsLmNvbTAeFw0y
             MTEwMTEyMDAzNTRaFw0yMzEwMTEyMDAzNTRaMGIxCzAJBgNVBAYTAlVTMRMwEQYD
             VQQIDApDYWxpZm9ybmlhMRYwFAYDVQQHDA1TYW4gRnJhbmNpc2NvMQ8wDQYDVQQK
             DAZCYWRTU0wxFTATBgNVBAMMDCouYmFkc3NsLmNvbTCCASIwDQYJKoZIhvcNAQEB
             BQADggEPADCCAQoCggEBAMIE7PiM7gTCs9hQ1XBYzJMY61yoaEmwIrX5lZ6xKyx2
             PmzAS2BMTOqytMAPgLaw+XLJhgL5XEFdEyt/ccRLvOmULlA3pmccYYz2QULFRtMW
             hyefdOsKnRFSJiFzbIRMeVXk0WvoBj1IFVKtsyjbqv9u/2CVSndrOfEk0TG23U3A
             xPxTuW1CrbV8/q71FdIzSOciccfCFHpsKOo3St/qbLVytH5aohbcabFXRNsKEqve
             ww9HdFxBIuGa+RuT5q0iBikusbpJHAwnnqP7i/dAcgCskgjZjFeEU4EFy+b+a1SY
             QCeFxxC7c3DvaRhBB0VVfPlkPz0sw6l865MaTIbRyoUCAwEAAaMyMDAwCQYDVR0T
             BAIwADAjBgNVHREEHDAaggwqLmJhZHNzbC5jb22CCmJhZHNzbC5jb20wDQYJKoZI
             hvcNAQELBQADggEBAC4DensZ5tCTeCNJbHABYPwwqLUFOMITKOOgF3t8EqOan0CH
             ST1NNi4jPslWrVhQ4Y3UbAhRBdqXl5N/NFfMzDosPpOjFgtifh8Z2s3w8vdlEZzf
             A4mYTC8APgdpWyNgMsp8cdXQF7QOfdnqOfdnY+pfc8a8joObR7HEaeVxhJs+XL4E
             CLByw5FR+svkYgCbQGWIgrM1cRpmXemt6Gf/XgFNP2PdubxqDEcnWlTMk8FCBVb1
             nVDSiPjYShwnWsOOshshCRCAiIBPCKPX0QwKDComQlRrgMIvddaSzFFTKPoNZjC+
             CUspSNnL7V9IIHvqKlRSmu+zIpm2VJCp1xLulk8=
             -----END CERTIFICATE-----
         "},
        )
        .unwrap();

        #[allow(unsafe_code)]
        unsafe {
            std::env::set_var("SSL_CERT_FILE", badssl_self_signed_cert);
        }

        assert!(download_file("https://self-signed.badssl.com", temp_file.path()).is_ok());

        #[allow(unsafe_code)]
        unsafe {
            std::env::remove_var("SSL_CERT_FILE");
        }
    }
}
