use crate::layers::{BundlerLayer, RubyLayer};
use libcnb::build::{BuildContext, BuildResult, BuildResultBuilder};
use libcnb::data::launch::{Launch, Process};
use libcnb::data::{layer_name, process_type};
use libcnb::detect::{DetectContext, DetectResult, DetectResultBuilder};
use libcnb::generic::GenericPlatform;
use libcnb::layer_env::TargetLifecycle;
use libcnb::{buildpack_main, Buildpack, Env};

use crate::util::{DownloadError, UntarError};
use serde::Deserialize;
use std::process::ExitStatus;

mod layers;
mod util;

pub struct RubyBuildpack;

impl Buildpack for RubyBuildpack {
    type Platform = GenericPlatform;
    type Metadata = RubyBuildpackMetadata;
    type Error = RubyBuildpackError;

    fn detect(&self, context: DetectContext<Self>) -> libcnb::Result<DetectResult, Self::Error> {
        let result = if context.app_dir.join("Gemfile.lock").exists() {
            DetectResultBuilder::pass().build()
        } else {
            DetectResultBuilder::fail().build()
        };

        Ok(result)
    }

    fn build(&self, context: BuildContext<Self>) -> libcnb::Result<BuildResult, Self::Error> {
        println!("---> Ruby Buildpack");

        let ruby_layer = context.handle_layer(layer_name!("ruby"), RubyLayer)?;

        context.handle_layer(
            layer_name!("bundler"),
            BundlerLayer {
                ruby_env: ruby_layer.env.apply(TargetLifecycle::Build, &Env::new()),
            },
        )?;

        Ok(BuildResultBuilder::new()
            .launch(
                Launch::new()
                    .process(Process::new(
                        process_type!("web"),
                        "bundle",
                        vec!["exec", "ruby", "app.rb"],
                        false,
                        true,
                    ))
                    .process(Process::new(
                        process_type!("worker"),
                        "bundle",
                        vec!["exec", "ruby", "worker.rb"],
                        false,
                        false,
                    )),
            )
            .build())
    }
}

#[derive(Deserialize, Debug)]
pub struct RubyBuildpackMetadata {
    pub ruby_url: String,
}

#[derive(Debug)]
pub enum RubyBuildpackError {
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
