// Enable rustc and Clippy lints that are disabled by default.
// https://doc.rust-lang.org/rustc/lints/listing/allowed-by-default.html#unused-crate-dependencies
#![warn(unused_crate_dependencies)]
// https://rust-lang.github.io/rust-clippy/stable/index.html
#![warn(clippy::pedantic)]
// This lint is too noisy and enforces a style that reduces readability in many cases.
#![allow(clippy::module_name_repetitions)]
#![allow(clippy::default_trait_access)]

mod app;
mod build;
mod container_context;
mod container_port_mapping;
mod pack;
mod util;

pub use crate::container_context::{ContainerContext, ContainerExecResult};

use crate::pack::PackBuildCommand;
use bollard::container::{Config, CreateContainerOptions, StartContainerOptions};
use bollard::image::RemoveImageOptions;
use bollard::Docker;

use std::path::{Path, PathBuf};
use std::process::{Command, Stdio};

/// Main type for libcnb integration tests.
///
/// # Dependencies
/// Integration tests require external tools to be available on the host to run:
/// - [pack](https://buildpacks.io/docs/tools/pack/)
/// - [Docker](https://www.docker.com/)
///
/// # Example
/// ```no_run
/// use libcnb_test::{IntegrationTest, BuildpackReference};
///
/// # fn call_test_fixture_service(addr: std::net::SocketAddr, payload: &str) -> Result<String, ()> {
/// #    unimplemented!()
/// # }
/// IntegrationTest::new("heroku/buildpacks:20", "test-fixtures/app")
///     .buildpacks(vec![
///         BuildpackReference::Other(String::from("heroku/openjdk")),
///         BuildpackReference::Crate,
///     ])
///     .run(|context| {
///         assert!(context.pack_stdout.contains("---> Maven Buildpack"));
///         assert!(context.pack_stdout.contains("---> Installing Maven"));
///         assert!(context.pack_stdout.contains("---> Running mvn package"));
///
///         context.start_container(&[12345], |container| {
///             assert_eq!(
///                 call_test_fixture_service(
///                     container.address_for_port(12345).unwrap(),
///                     "Hagbard Celine"
///                 )
///                 .unwrap(),
///                 "enileC drabgaH"
///             );
///         });
///     });
/// ```
pub struct IntegrationTest {
    app_dir: PathBuf,
    target_triple: String,
    builder_name: String,
    buildpacks: Vec<BuildpackReference>,
    docker: Docker,
    tokio_runtime: tokio::runtime::Runtime,
}

/// References a Cloud Native Buildpack
pub enum BuildpackReference {
    /// References this buildpack in the Rust Crate under test
    Crate,
    /// References another buildpack by id, local directory or tarball
    Other(String),
}

impl IntegrationTest {
    /// Creates a new integration test.
    ///
    /// # Panics
    /// - When the connection to Docker failed
    /// - When the internal Tokio runtime could not be created
    pub fn new(builder_name: impl Into<String>, app_dir: impl AsRef<Path>) -> Self {
        IntegrationTest {
            app_dir: PathBuf::from(app_dir.as_ref()),
            target_triple: String::from("x86_64-unknown-linux-musl"),
            builder_name: builder_name.into(),
            buildpacks: vec![BuildpackReference::Crate],
            docker: Docker::connect_with_local_defaults()
                .expect("Could not connect to local Docker deamon"),
            tokio_runtime: tokio::runtime::Runtime::new()
                .expect("Could not create internal Tokio runtime"),
        }
    }

    /// Sets the buildpacks order.
    ///
    /// Defaults to [`BuildpackReference::Crate`].
    pub fn buildpacks(&mut self, buildpacks: impl Into<Vec<BuildpackReference>>) -> &mut Self {
        self.buildpacks = buildpacks.into();
        self
    }

    /// Sets the target triple.
    ///
    /// Defaults to `x86_64-unknown-linux-musl`.
    pub fn target_triple(&mut self, target_triple: impl Into<String>) -> &mut Self {
        self.target_triple = target_triple.into();
        self
    }

    /// Starts a new integration test run.
    ///
    /// This function will copy the application to a temporary directory, cross-compiles this crate,
    /// packages it as a buildpack and then invokes [pack](https://buildpacks.io/docs/tools/pack/)
    /// to build a new Docker image with the buildpacks specified by this integration test instance.
    ///
    /// Since this function is supposed to only be used in integration tests, failures are not
    /// signalled via [`Result`](Result) values. Instead, this function panics whenever an unexpected error
    /// occurred to simplify testing code.
    ///
    /// # Panics
    /// - When the app could not be copied
    /// - When this crate could not be packed as a buildpack
    /// - When the "pack" command unexpectedly fails
    ///
    /// # Example
    /// ```no_run
    /// use libcnb_test::IntegrationTest;
    ///
    /// IntegrationTest::new("heroku/buildpacks:20", "test-fixtures/app")
    ///     .run(|context| {
    ///         assert!(context.pack_stdout.contains("---> Ruby Buildpack"));
    ///         assert!(context.pack_stdout.contains("---> Installing bundler"));
    ///         assert!(context.pack_stdout.contains("---> Installing gems"));
    ///     })
    /// ```
    pub fn run<F: FnOnce(IntegrationTestContext)>(&mut self, f: F) {
        let temp_app_dir =
            app::copy_app(&self.app_dir).expect("Could not copy app to temporary location");

        let temp_crate_buildpack_dir =
            build::package_crate_buildpack(&self.target_triple, temp_app_dir.path())
                .expect("Could not package current crate as buildpack");

        let image_name = util::random_docker_identifier();

        let mut pack_command =
            PackBuildCommand::new(&self.builder_name, temp_app_dir.path(), &image_name);

        for buildpack in &self.buildpacks {
            match buildpack {
                BuildpackReference::Crate => pack_command.buildpack(&temp_crate_buildpack_dir),
                BuildpackReference::Other(id) => pack_command.buildpack(id.clone()),
            };
        }

        let output = Command::from(pack_command)
            .stdout(Stdio::piped())
            .stderr(Stdio::piped())
            .spawn()
            .expect("Could not spawn external 'pack' process")
            .wait_with_output()
            .expect("Error while waiting on external 'pack' process");

        f(IntegrationTestContext {
            pack_stdout: String::from_utf8_lossy(&output.stdout).into_owned(),
            pack_stderr: String::from_utf8_lossy(&output.stderr).into_owned(),
            image_name,
            app_dir: PathBuf::from(temp_app_dir.path()),
            integration_test: self,
        });
    }
}

/// Context for a currently executing integration test.
pub struct IntegrationTestContext<'a> {
    /// Standard output of `pack`, interpreted as an UTF-8 string.
    pub pack_stdout: String,
    /// Standard error of `pack`, interpreted as an UTF-8 string.
    pub pack_stderr: String,
    /// The name of the image this integration test created.
    pub image_name: String,
    /// The directory of the app this integration test uses.
    ///
    /// This is a copy of the `app_dir` directory passed to [`IntegrationTest::new`] and unique to
    /// this integration test run. It is safe to modify the directory contents inside the test.
    pub app_dir: PathBuf,

    integration_test: &'a IntegrationTest,
}

impl<'a> IntegrationTestContext<'a> {
    /// Starts a new container with the image from the integration test.
    ///
    /// # Panics
    /// - When the container could not be created
    /// - When the container could not be started
    pub fn start_container<F: FnOnce(ContainerContext)>(&self, exposed_ports: &[u16], f: F) {
        let container_name = util::random_docker_identifier();

        self.integration_test.tokio_runtime.block_on(async {
            self.integration_test
                .docker
                .create_container(
                    Some(CreateContainerOptions {
                        name: container_name.clone(),
                    }),
                    Config {
                        image: Some(self.image_name.clone()),
                        ..container_port_mapping::port_mapped_container_config(exposed_ports)
                    },
                )
                .await
                .expect("Could not create container");

            self.integration_test
                .docker
                .start_container(&container_name, None::<StartContainerOptions<String>>)
                .await
                .expect("Could not start container");
        });

        f(ContainerContext {
            container_name,
            integration_test_context: self,
        });
    }
}

impl<'a> Drop for IntegrationTestContext<'a> {
    #[allow(unused_must_use)]
    fn drop(&mut self) {
        // We do not care if image removal succeeded or not. Panicking here would result in
        // SIGILL since this function might be called in a Tokio runtime.
        self.integration_test
            .tokio_runtime
            .block_on(self.integration_test.docker.remove_image(
                &self.image_name,
                Some(RemoveImageOptions {
                    force: true,
                    ..Default::default()
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
