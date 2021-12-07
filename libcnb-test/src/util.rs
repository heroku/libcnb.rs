use rand::{thread_rng, Rng};

pub fn random_docker_identifier() -> String {
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
