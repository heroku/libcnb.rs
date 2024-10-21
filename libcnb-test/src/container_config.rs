use std::collections::{HashMap, HashSet};
use std::path::{Path, PathBuf};

/// Config used when starting a container.
///
/// By default the container will run the CNB default process-type, however this can be
/// overridden using [`ContainerConfig::entrypoint`] and [`ContainerConfig::command`].
/// See: [CNB App Developer Guide: Run a multi-process app](https://buildpacks.io/docs/app-developer-guide/run-an-app/#run-a-multi-process-app)
///
/// # Example
/// ```no_run
/// use libcnb_test::{BuildConfig, ContainerConfig, TestRunner};
///
/// TestRunner::default().build(
///     BuildConfig::new("heroku/builder:22", "tests/fixtures/app"),
///     |context| {
///         // ...
///         context.start_container(
///             ContainerConfig::new()
///                 .env("PORT", "12345")
///                 .expose_port(12345),
///             |container| {
///                 // ...
///             },
///         );
///     },
/// );
/// ```
#[derive(Clone, Default)]
pub struct ContainerConfig {
    pub(crate) entrypoint: Option<String>,
    pub(crate) command: Option<Vec<String>>,
    pub(crate) env: HashMap<String, String>,
    pub(crate) exposed_ports: HashSet<u16>,
    pub(crate) volumes: HashMap<PathBuf, PathBuf>,
}

impl ContainerConfig {
    /// Creates an empty [`ContainerConfig`] instance.
    ///
    /// By default the container will run the CNB default process-type, however this can be
    /// overridden using [`ContainerConfig::entrypoint`] and [`ContainerConfig::command`].
    /// See: [CNB App Developer Guide: Run a multi-process app](https://buildpacks.io/docs/app-developer-guide/run-an-app/#run-a-multi-process-app)
    ///
    /// # Example
    /// ```no_run
    /// use libcnb_test::{BuildConfig, ContainerConfig, TestRunner};
    ///
    /// TestRunner::default().build(
    ///     BuildConfig::new("heroku/builder:22", "tests/fixtures/app"),
    ///     |context| {
    ///         // ...
    ///         context.start_container(
    ///             ContainerConfig::new()
    ///                 .env("PORT", "12345")
    ///                 .expose_port(12345),
    ///             |container| {
    ///                 // ...
    ///             },
    ///         );
    ///     },
    /// );
    /// ```
    #[must_use]
    pub fn new() -> Self {
        Self::default()
    }

    /// Override the image's `entrypoint` (which is the CNB default process-type).
    ///
    /// See: [CNB App Developer Guide: Run a multi-process app](https://buildpacks.io/docs/app-developer-guide/run-an-app/#run-a-multi-process-app)
    ///
    /// # Example
    /// ```no_run
    /// use libcnb_test::{BuildConfig, ContainerConfig, TestRunner};
    ///
    /// TestRunner::default().build(
    ///     BuildConfig::new("heroku/builder:22", "tests/fixtures/app"),
    ///     |context| {
    ///         // ...
    ///         context.start_container(ContainerConfig::new().entrypoint("worker"), |container| {
    ///             // ...
    ///         });
    ///     },
    /// );
    /// ```
    pub fn entrypoint(&mut self, entrypoint: impl Into<String>) -> &mut Self {
        self.entrypoint = Some(entrypoint.into());
        self
    }

    /// Set the container's `command` (CNB images have no default command).
    ///
    /// See: [CNB App Developer Guide: Run a multi-process app](https://buildpacks.io/docs/app-developer-guide/run-an-app/#run-a-multi-process-app)
    ///
    /// # Example
    /// ```no_run
    /// use libcnb_test::{BuildConfig, ContainerConfig, TestRunner};
    ///
    /// TestRunner::default().build(
    ///     BuildConfig::new("heroku/builder:22", "tests/fixtures/app"),
    ///     |context| {
    ///         // ...
    ///         context.start_container(
    ///             ContainerConfig::new().command(["--additional-arg1", "--additional-arg2"]),
    ///             |container| {
    ///                 // ...
    ///             },
    ///         );
    ///     },
    /// );
    /// ```
    pub fn command<I: IntoIterator<Item = S>, S: Into<String>>(&mut self, command: I) -> &mut Self {
        self.command = Some(command.into_iter().map(S::into).collect());
        self
    }

    /// Exposes a given port of the container to the host machine.
    ///
    /// The given port is mapped to a random port on the host machine. Use
    /// [`crate::ContainerContext::address_for_port`] to obtain the local port for a mapped port.
    ///
    /// # Example
    /// ```no_run
    /// use libcnb_test::{BuildConfig, ContainerConfig, TestRunner};
    ///
    /// TestRunner::default().build(
    ///     BuildConfig::new("heroku/builder:22", "tests/fixtures/app"),
    ///     |context| {
    ///         // ...
    ///         context.start_container(
    ///             ContainerConfig::new()
    ///                 .env("PORT", "12345")
    ///                 .expose_port(12345),
    ///             |container| {
    ///                 let address_on_host = container.address_for_port(12345);
    ///                 // ...
    ///             },
    ///         );
    ///     },
    /// );
    /// ```
    pub fn expose_port(&mut self, port: u16) -> &mut Self {
        self.exposed_ports.insert(port);
        self
    }

    /// Inserts or updates an environment variable mapping for the container.
    ///
    /// # Example
    /// ```no_run
    /// use libcnb_test::{BuildConfig, ContainerConfig, TestRunner};
    ///
    /// TestRunner::default().build(
    ///     BuildConfig::new("heroku/builder:22", "tests/fixtures/app"),
    ///     |context| {
    ///         // ...
    ///         context.start_container(
    ///             ContainerConfig::new()
    ///                 .env("PORT", "5678")
    ///                 .env("DEBUG", "true"),
    ///             |container| {
    ///                 // ...
    ///             },
    ///         );
    ///     },
    /// );
    /// ```
    pub fn env(&mut self, key: impl Into<String>, value: impl Into<String>) -> &mut Self {
        self.env.insert(key.into(), value.into());
        self
    }

    /// Mounts a named volume `source` into the container `destination`. Useful for integration
    /// tests that depend on persistent storage shared between container executions.
    ///
    /// See: [Docker CLI, Mount Volume](https://docs.docker.com/reference/cli/docker/container/run/#volume)
    ///
    /// # Example
    /// ```no_run
    /// use libcnb_test::{BuildConfig, ContainerConfig, TestRunner};
    /// use std::path::PathBuf;
    ///
    /// TestRunner::default().build(
    ///     BuildConfig::new("heroku/builder:22", "tests/fixtures/app"),
    ///     |context| {
    ///         // ...
    ///         context.start_container(
    ///             ContainerConfig::new().volume(PathBuf::from("/shared/cache"), PathBuf::from("/workspace/cache")),
    ///             |container| {
    ///                 // ...
    ///             },
    ///         );
    ///     },
    /// );
    /// ```
    pub fn volume(&mut self, source: impl AsRef<Path>, destination: impl AsRef<Path>) -> &mut Self {
        self.volumes.insert(
            source.as_ref().to_path_buf(),
            destination.as_ref().to_path_buf(),
        );
        self
    }

    /// Adds or updates multiple environment variable mappings for the container.
    ///
    /// # Example
    /// ```no_run
    /// use libcnb_test::{BuildConfig, ContainerConfig, TestRunner};
    ///
    /// TestRunner::default().build(
    ///     BuildConfig::new("heroku/builder:22", "tests/fixtures/app"),
    ///     |context| {
    ///         // ...
    ///         context.start_container(
    ///             ContainerConfig::new().envs([("PORT", "5678"), ("DEBUG", "true")]),
    ///             |container| {
    ///                 // ...
    ///             },
    ///         );
    ///     },
    /// );
    /// ```
    pub fn envs<K: Into<String>, V: Into<String>, I: IntoIterator<Item = (K, V)>>(
        &mut self,
        envs: I,
    ) -> &mut Self {
        envs.into_iter().for_each(|(key, value)| {
            self.env(key.into(), value.into());
        });

        self
    }
}
