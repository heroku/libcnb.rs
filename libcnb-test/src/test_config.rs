use std::collections::HashMap;
use std::path::{Path, PathBuf};
use std::rc::Rc;

/// Configuration for a test.
#[derive(Clone)]
pub struct TestConfig {
    pub(crate) app_dir: PathBuf,
    pub(crate) target_triple: String,
    pub(crate) builder_name: String,
    pub(crate) buildpacks: Vec<BuildpackReference>,
    pub(crate) env: HashMap<String, String>,
    pub(crate) app_dir_preprocessor: Option<Rc<dyn Fn(PathBuf)>>,
    pub(crate) expected_pack_result: PackResult,
}

impl TestConfig {
    /// Creates a new test configuration.
    ///
    /// If the `app_dir` parameter is a relative path, it is treated as relative to the Cargo
    /// manifest directory ([`CARGO_MANIFEST_DIR`](https://doc.rust-lang.org/cargo/reference/environment-variables.html#environment-variables-cargo-sets-for-crates)),
    /// i.e. the package's root directory.
    pub fn new(builder_name: impl Into<String>, app_dir: impl AsRef<Path>) -> Self {
        TestConfig {
            app_dir: PathBuf::from(app_dir.as_ref()),
            target_triple: String::from("x86_64-unknown-linux-musl"),
            builder_name: builder_name.into(),
            buildpacks: vec![BuildpackReference::Crate],
            env: HashMap::new(),
            app_dir_preprocessor: None,
            expected_pack_result: PackResult::Success,
        }
    }

    /// Sets the buildpacks order.
    ///
    /// Defaults to [`BuildpackReference::Crate`].
    pub fn buildpacks(&mut self, buildpacks: impl Into<Vec<BuildpackReference>>) -> &mut Self {
        self.buildpacks = buildpacks.into();
        self
    }

    /// Sets the target triple used when compiling the buildpack.
    ///
    /// Defaults to `x86_64-unknown-linux-musl`.
    pub fn target_triple(&mut self, target_triple: impl Into<String>) -> &mut Self {
        self.target_triple = target_triple.into();
        self
    }

    /// Inserts or updates an environment variable mapping for the build process.
    ///
    /// Note: This does not set this environment variable for running containers, it's only
    /// available during the build.
    ///
    /// # Example
    /// ```no_run
    /// use libcnb_test::{TestConfig, TestRunner};
    ///
    /// TestRunner::default().run_test(
    ///     TestConfig::new("heroku/builder:22", "test-fixtures/app")
    ///         .env("ENV_VAR_ONE", "VALUE ONE")
    ///         .env("ENV_VAR_TWO", "SOME OTHER VALUE"),
    ///     |context| {
    ///         // ...
    ///     },
    /// )
    /// ```
    pub fn env(&mut self, k: impl Into<String>, v: impl Into<String>) -> &mut Self {
        self.env.insert(k.into(), v.into());
        self
    }

    /// Adds or updates multiple environment variable mappings for the build process.
    ///
    /// Note: This does not set environment variables for running containers, they're only
    /// available during the build.
    ///
    /// # Example
    /// ```no_run
    /// use libcnb_test::{TestConfig, TestRunner};
    ///
    /// TestRunner::default().run_test(
    ///     TestConfig::new("heroku/builder:22", "test-fixtures/app").envs(vec![
    ///         ("ENV_VAR_ONE", "VALUE ONE"),
    ///         ("ENV_VAR_TWO", "SOME OTHER VALUE"),
    ///     ]),
    ///     |context| {
    ///         // ...
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

    /// Sets an app directory preprocessor function.
    ///
    /// It will be run after the app directory has been copied for the current integration test run,
    /// the changes will not affect other integration test runs.
    ///
    /// Generally, we suggest using dedicated test fixtures. However, in some cases it is more
    /// economical to slightly modify a fixture programmatically before a test instead.
    ///
    /// # Example
    /// ```no_run
    /// use libcnb_test::{TestConfig, TestRunner};
    ///
    /// TestRunner::default().run_test(
    ///     TestConfig::new("heroku/builder:22", "test-fixtures/app").app_dir_preprocessor(
    ///         |app_dir| std::fs::remove_file(app_dir.join("Procfile")).unwrap(),
    ///     ),
    ///     |context| {
    ///         // ...
    ///     },
    /// );
    /// ```
    pub fn app_dir_preprocessor<F: 'static + Fn(PathBuf)>(&mut self, f: F) -> &mut Self {
        self.app_dir_preprocessor = Some(Rc::new(f));
        self
    }

    /// Sets the app directory.
    ///
    /// The app directory is normally set in the [`TestConfig::new`] call, but when sharing test
    /// configuration, it might be necessary to change the app directory but keep everything else
    /// the same.
    pub fn app_dir<P: Into<PathBuf>>(&mut self, path: P) -> &mut Self {
        self.app_dir = path.into();
        self
    }

    /// Set the expected `pack` command result.
    ///
    /// In some cases, users might want to explicitly test that a build fails and asserting against
    /// error output. When passed [`PackResult::Failure`], the test will fail if the pack build
    /// succeeds and vice-versa.
    ///
    /// Defaults to [`PackResult::Success`]
    pub fn expected_pack_result(&mut self, pack_result: PackResult) -> &mut Self {
        self.expected_pack_result = pack_result;
        self
    }
}

/// References a Cloud Native Buildpack
#[derive(Debug, Clone, Eq, PartialEq)]
pub enum BuildpackReference {
    /// References the buildpack in the Rust Crate currently being tested
    Crate,
    /// References another buildpack by id, local directory or tarball
    Other(String),
}

/// Result of a pack execution.
#[derive(Debug, Clone, Eq, PartialEq)]
pub enum PackResult {
    /// Pack executed successfully.
    Success,
    /// Pack execution failed.
    Failure,
}
