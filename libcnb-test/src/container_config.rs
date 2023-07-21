use std::collections::{HashMap, HashSet};

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
///     BuildConfig::new("heroku/builder:22", "test-fixtures/app"),
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
#[derive(Default)]
pub struct ContainerConfig {
    pub(crate) entrypoint: Option<Vec<String>>,
    pub(crate) command: Option<Vec<String>>,
    pub(crate) env: HashMap<String, String>,
    pub(crate) exposed_ports: HashSet<u16>,
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
    ///     BuildConfig::new("heroku/builder:22", "test-fixtures/app"),
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
    ///     BuildConfig::new("heroku/builder:22", "test-fixtures/app"),
    ///     |context| {
    ///         // ...
    ///         context.start_container(ContainerConfig::new().entrypoint(["worker"]), |container| {
    ///             // ...
    ///         });
    ///     },
    /// );
    /// ```
    pub fn entrypoint<I: IntoIterator<Item = S>, S: Into<String>>(
        &mut self,
        entrypoint: I,
    ) -> &mut Self {
        self.entrypoint = Some(entrypoint.into_iter().map(S::into).collect());
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
    ///     BuildConfig::new("heroku/builder:22", "test-fixtures/app"),
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
    ///     BuildConfig::new("heroku/builder:22", "test-fixtures/app"),
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
    ///     BuildConfig::new("heroku/builder:22", "test-fixtures/app"),
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

    /// Adds or updates multiple environment variable mappings for the container.
    ///
    /// # Example
    /// ```no_run
    /// use libcnb_test::{BuildConfig, ContainerConfig, TestRunner};
    ///
    /// TestRunner::default().build(
    ///     BuildConfig::new("heroku/builder:22", "test-fixtures/app"),
    ///     |context| {
    ///         // ...
    ///         context.start_container(
    ///             ContainerConfig::new().envs(vec![("PORT", "5678"), ("DEBUG", "true")]),
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
