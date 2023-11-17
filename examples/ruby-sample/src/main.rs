use crate::layers::{BundlerLayer, RubyLayer};
use crate::util::{DownloadError, UntarError};
use libcnb::build::{BuildContext, BuildResult, BuildResultBuilder};
use libcnb::data::launch::{LaunchBuilder, ProcessBuilder};
use libcnb::data::{layer_name, process_type};
use libcnb::detect::{DetectContext, DetectResult, DetectResultBuilder};
use libcnb::generic::GenericPlatform;
use libcnb::layer_env::Scope;
use libcnb::{buildpack_main, Buildpack};
use serde::Deserialize;
use std::process::ExitStatus;

// Suppress warnings due to the `unused_crate_dependencies` lint not handling integration tests well.
#[cfg(test)]
use libcnb_test as _;

mod layers;
mod util;

pub(crate) struct RubyBuildpack;

impl Buildpack for RubyBuildpack {
    type Platform = GenericPlatform;
    type Metadata = RubyBuildpackMetadata;
    type Error = RubyBuildpackError;

    fn detect(&self, context: DetectContext<Self>) -> libcnb::Result<DetectResult, Self::Error> {
        if context.app_dir.join("Gemfile.lock").exists() {
            DetectResultBuilder::pass().build()
        } else {
            DetectResultBuilder::fail().build()
        }
    }

    fn build(&self, context: BuildContext<Self>) -> libcnb::Result<BuildResult, Self::Error> {
        println!("---> Ruby Buildpack");

        let ruby_layer = context.handle_layer(layer_name!("ruby"), RubyLayer)?;

        context.handle_layer(
            layer_name!("bundler"),
            BundlerLayer {
                ruby_env: ruby_layer.env.apply_to_empty(Scope::Build),
            },
        )?;

        BuildResultBuilder::new()
            .launch(
                LaunchBuilder::new()
                    .process(
                        ProcessBuilder::new(process_type!("web"), ["bundle"])
                            .args(["exec", "ruby", "app.rb"])
                            .default(true)
                            .build(),
                    )
                    .process(
                        ProcessBuilder::new(process_type!("worker"), ["bundle"])
                            .args(["exec", "ruby", "worker.rb"])
                            .build(),
                    )
                    .build(),
            )
            .build()
    }
}

#[derive(Deserialize, Debug)]
#[serde(deny_unknown_fields)]
pub(crate) struct RubyBuildpackMetadata {
    pub(crate) ruby_url: String,
}

#[derive(Debug)]
pub(crate) enum RubyBuildpackError {
    RubyDownloadError(DownloadError),
    RubyUntarError(UntarError),
    CouldNotCreateTemporaryFile(std::io::Error),
    CouldNotGenerateChecksum(std::io::Error),
    GemInstallBundlerCommandError(std::io::Error),
    GemInstallBundlerUnexpectedExitStatus(ExitStatus),
    BundleInstallCommandError(std::io::Error),
    BundleInstallUnexpectedExitStatus(ExitStatus),
    BundleConfigCommandError(std::io::Error),
    BundleConfigUnexpectedExitStatus(ExitStatus),
}

buildpack_main!(RubyBuildpack);
