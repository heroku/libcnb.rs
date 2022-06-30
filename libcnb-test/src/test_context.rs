use crate::{PrepareContainerContext, TestConfig, TestRunner};
use bollard::image::RemoveImageOptions;
use std::borrow::Borrow;

/// Context for a currently executing test.
pub struct TestContext<'a> {
    /// Standard output of `pack`, interpreted as an UTF-8 string.
    pub pack_stdout: String,
    /// Standard error of `pack`, interpreted as an UTF-8 string.
    pub pack_stderr: String,
    /// The configuration used for this integration test.
    pub config: TestConfig,

    pub(crate) image_name: String,
    pub(crate) runner: &'a TestRunner,
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
    ///     TestConfig::new("heroku/builder:22", "test-fixtures/app"),
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
    /// from the previous test, causing the CNB lifecycle to restore any cached layers. It will use the
    /// same [`TestRunner`] as the previous test run.
    ///
    /// This function allows testing of subsequent builds, including caching logic and buildpack
    /// behaviour when build environment variables change, stacks are upgraded and more.
    ///
    /// Note that this function will consume the current context. This is because the image will
    /// be changed by the subsequent test, invalidating the context. Running a subsequent test must
    /// therefore be the last operation. You can nest subsequent runs if required.
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
