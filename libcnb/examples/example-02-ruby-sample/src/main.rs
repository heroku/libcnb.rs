use crate::layers::BundlerLayerLifecycle;
use crate::layers::RubyLayerLifecycle;
use libcnb::build::{BuildContext, BuildResult, BuildResultBuilder};
use libcnb::buildpack_main;
use libcnb::data::launch::{Launch, Process};
use libcnb::data::process_type;
use libcnb::detect::{DetectContext, DetectResult, DetectResultBuilder};
use libcnb::generic::GenericPlatform;
use libcnb::layer_lifecycle::execute_layer_lifecycle;
use libcnb::Buildpack;
use serde::Deserialize;

mod layers;

#[derive(Deserialize, Debug)]
struct RubyBuildpackMetadata {
    pub ruby_url: String,
}

struct RubyBuildpack;

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

        let ruby_env = execute_layer_lifecycle("ruby", RubyLayerLifecycle, &context)?;

        execute_layer_lifecycle("bundler", BundlerLayerLifecycle { ruby_env }, &context)?;

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

buildpack_main!(RubyBuildpack);
