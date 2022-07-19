use std::iter::repeat_with;

/// Generate a random Docker identifier.
///
/// It is suitable to be used as an image tag or container name.
///
/// See: [Docker Image Specification](https://github.com/moby/moby/blob/master/image/spec/v1.1.md)
pub(crate) fn random_docker_identifier() -> String {
    format!(
        "libcnbtest_{}",
        repeat_with(fastrand::lowercase)
            .take(30)
            .collect::<String>()
    )
}

pub(crate) const CNB_LAUNCHER_BINARY: &str = "launcher";
