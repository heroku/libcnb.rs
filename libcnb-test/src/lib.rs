// Enable rustc and Clippy lints that are disabled by default.
// https://doc.rust-lang.org/rustc/lints/listing/allowed-by-default.html#unused-crate-dependencies
#![warn(unused_crate_dependencies)]
// https://rust-lang.github.io/rust-clippy/stable/index.html
#![warn(clippy::pedantic)]
// This lint is too noisy and enforces a style that reduces readability in many cases.
#![allow(clippy::module_name_repetitions)]

mod app;
mod build;
mod container_context;
mod container_port_mapping;
mod log;
mod macros;
mod pack;
mod test_config;
mod test_runner;
mod util;

pub use crate::container_context::{ContainerContext, PrepareContainerContext};
use crate::pack::{PackBuildCommand, PullPolicy};
pub use crate::test_config::*;
pub use crate::test_runner::*;
use bollard::image::RemoveImageOptions;
use std::borrow::Borrow;
use std::path::PathBuf;

/// Context for a currently executing test.
pub struct TestContext<'a> {
    /// Standard output of `pack`, interpreted as an UTF-8 string.
    pub pack_stdout: String,
    /// Standard error of `pack`, interpreted as an UTF-8 string.
    pub pack_stderr: String,
    /// The directory of the app this integration test uses.
    ///
    /// This is a copy of the `app_dir` directory passed to [`TestConfig::new`] and unique to
    /// this integration test run. It is safe to modify the directory contents inside the test.
    pub app_dir: PathBuf,
    /// The configuration used for this integration test.
    pub config: TestConfig,

    image_name: String,
    runner: &'a TestRunner,
}

impl<'a> TestContext<'a> {
    /// Prepares a new container with the image from the test.
    ///
    /// This will not create nor run the container immediately. Use the returned
    /// `PrepareContainerContext` to configure the container, then call
    /// [`start_with_default_process`](PrepareContainerContext::start_with_default_process) on it
    /// to actually create and start the container.
    ///
    /// # Example:
    ///
    /// ```no_run
    /// use libcnb_test::{TestConfig, TestRunner};
    ///
    /// TestRunner::default().run_test(
    ///     TestConfig::new("heroku/builder:22", "test-fixtures/empty-app"),
    ///     |context| {
    ///         context
    ///             .prepare_container()
    ///             .start_with_default_process(|container| {
    ///                 // ...
    ///             });
    ///     },
    /// );
    /// ```
    #[must_use]
    pub fn prepare_container(&self) -> PrepareContainerContext {
        PrepareContainerContext::new(self)
    }

    /// Starts a subsequent integration test run.
    ///
    /// This function behaves exactly like [`TestRunner::run_test`], but it will reuse the OCI image
    /// from the previous test, causing the CNB lifecycle to restore cached layers. It will use the
    /// same [`TestRunner`] as the previous test run.
    ///
    /// This function allows testing of subsequent builds, including caching logic and buildpack
    /// behaviour when build environment variables change, stacks are upgraded and more.
    ///
    /// Note that this function will consume the current context. This is because the image will
    /// be changed by the subsequent test, invalidating the context. Running a subsequent test must
    /// therefore be the last operation. You can nest subsequent runs if required.
    ///
    /// # Panics
    /// - When the app could not be copied
    /// - When this crate could not be packaged as a buildpack
    /// - When the `pack` command unexpectedly fails
    ///
    /// # Example
    /// ```no_run
    /// use libcnb_test::{assert_contains, TestRunner, TestConfig};
    ///
    /// TestRunner::default().run_test(
    ///     TestConfig::new("heroku/builder:22", "test-fixtures/app"),
    ///     |context| {
    ///         assert_contains!(context.pack_stdout, "---> Ruby Buildpack");
    ///         assert_contains!(context.pack_stdout, "---> Installing bundler");
    ///         assert_contains!(context.pack_stdout, "---> Installing gems");
    ///     },
    /// )
    /// ```
    pub fn run_test<C: Borrow<TestConfig>, F: FnOnce(TestContext)>(self, config: C, f: F) {
        self.runner
            .run_test_internal(self.image_name.clone(), config, f);
    }

    /// Starts a subsequent integration test run with inherited configuration.
    ///
    /// This function is the same as [`TestContext::run_test`] but automatically inherits the
    /// configuration from the previous test run. See [`TestContext::run_test`] for details.
    ///
    /// # Panics
    /// - When the app could not be copied
    /// - When this crate could not be packaged as a buildpack
    /// - When the `pack` command unexpectedly fails
    pub fn run_test_inherit_config<F: FnOnce(TestContext)>(self, f: F) {
        self.runner
            .run_test_internal(self.image_name.clone(), self.config.clone(), f);
    }
}

impl<'a> Drop for TestContext<'a> {
    fn drop(&mut self) {
        // We do not care if image removal succeeded or not. Panicking here would result in
        // SIGILL since this function might be called in a Tokio runtime.
        let _image_delete_result =
            self.runner
                .tokio_runtime
                .block_on(self.runner.docker.remove_image(
                    &self.image_name,
                    Some(RemoveImageOptions {
                        force: true,
                        ..RemoveImageOptions::default()
                    }),
                    None,
                ));
    }
}

// This runs the README.md as a doctest, ensuring the code examples in it are valid.
// It will not be part of the final crate.
#[cfg(doctest)]
#[doc = include_str!("../README.md")]
pub struct ReadmeDoctests;
