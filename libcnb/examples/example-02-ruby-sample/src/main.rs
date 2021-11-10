use crate::layers::{BundlerLayer, RubyLayer};
use libcnb::build::{BuildContext, BuildResult, BuildResultBuilder};
use libcnb::data::launch::{Launch, Process};
use libcnb::detect::{DetectContext, DetectResult, DetectResultBuilder};
use libcnb::generic::GenericPlatform;
use libcnb::layer_env::TargetLifecycle;
use libcnb::{buildpack_main, Buildpack, Env};

use serde::Deserialize;

mod layers;

#[derive(Deserialize, Debug)]
pub struct RubyBuildpackMetadata {
    pub ruby_url: String,
}

pub struct RubyBuildpack;

impl Buildpack for RubyBuildpack {
    type Platform = GenericPlatform;
    type Metadata = RubyBuildpackMetadata;
    type Error = anyhow::Error;

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

        let ruby_layer = context.handle_layer("ruby", RubyLayer)?;

        // TODO: Why isn't this in a layer?
        // println!("---> Installing bundler");
        // install_bundler(&ruby_env)?;

        context.handle_layer(
            "bundler",
            BundlerLayer {
                ruby_env: ruby_layer.env.apply(TargetLifecycle::Build, &Env::new()),
            },
        )?;

        Ok(BuildResultBuilder::new()
            .launch(
                Launch::new()
                    .process(Process::new(
                        "web",
                        "bundle",
                        vec!["exec", "ruby", "app.rb"],
                        false,
                        true,
                    )?)
                    .process(Process::new(
                        "worker",
                        "bundle",
                        vec!["exec", "ruby", "worker.rb"],
                        false,
                        false,
                    )?),
            )
            .build())
    }
}

buildpack_main!(RubyBuildpack);
