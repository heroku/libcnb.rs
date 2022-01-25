use rand::{thread_rng, Rng};

/// Generate a random Docker identifier.
///
/// It is suitable to be used as an image tag or container name.
///
/// See: [Docker Image Specification](https://github.com/moby/moby/blob/master/image/spec/v1.1.md)
pub(crate) fn random_docker_identifier() -> String {
    format!(
        "libcnbtest_{}",
        thread_rng()
            .sample_iter(&rand::distributions::Alphanumeric)
            .take(30)
            .map(char::from)
            .collect::<String>()
            .to_lowercase()
    )
}
