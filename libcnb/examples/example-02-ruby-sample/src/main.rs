use std::collections::HashMap;
use std::process::{Command, Stdio};

use crate::layers::bundler::BundlerLayerLifecycle;
use crate::layers::ruby::RubyLayerLifecycle;
use libcnb::build::{BuildContext, BuildOutcome, BuildOutcomeBuilder};
use libcnb::data::launch::{Launch, Process};
use libcnb::detect::{DetectContext, DetectOutcome, DetectOutcomeBuilder};
use libcnb::layer_lifecycle::execute_layer_lifecycle;
use libcnb::Buildpack;
use libcnb::{cnb_runtime, GenericPlatform};
use serde::Deserialize;

mod layers;

struct RubyBuildpack;
impl Buildpack for RubyBuildpack {
    type Platform = GenericPlatform;
    type Metadata = RubyBuildpackMetadata;
    type Error = anyhow::Error;

    fn detect(&self, context: DetectContext<Self>) -> libcnb::Result<DetectOutcome, Self::Error> {
        let outcome = if context.app_dir.join("Gemfile.lock").exists() {
            DetectOutcomeBuilder::pass().build()
        } else {
            DetectOutcomeBuilder::fail().build()
        };

        Ok(outcome)
    }

    fn build(&self, context: BuildContext<Self>) -> libcnb::Result<BuildOutcome, Self::Error> {
        println!("---> Ruby Buildpack");
        println!("---> Download and extracting Ruby");

        let ruby_env = execute_layer_lifecycle("ruby", RubyLayerLifecycle, &context)?;

        println!("---> Installing bundler");
        install_bundler(&ruby_env)?;
        execute_layer_lifecycle("bundler", BundlerLayerLifecycle { ruby_env }, &context)?;

        Ok(BuildOutcomeBuilder::new()
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

fn main() {
    cnb_runtime(RubyBuildpack)
}

#[derive(Deserialize, Debug)]
struct RubyBuildpackMetadata {
    pub ruby_url: String,
}

fn install_bundler(ruby_env: &HashMap<String, String>) -> anyhow::Result<()> {
    let cmd = Command::new("gem")
        .args(&["install", "bundler", "--no-ri", "--no-rdoc"])
        .envs(ruby_env)
        .stdout(Stdio::inherit())
        .stderr(Stdio::inherit())
        .spawn()?
        .wait()?;

    if cmd.success() {
        Ok(())
    } else {
        Err(anyhow::anyhow!("Could not install bundler"))
    }
}
