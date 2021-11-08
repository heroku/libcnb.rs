use std::collections::HashMap;
use std::process::{Command, Stdio};

use crate::layers::bundler::BundlerLayerLifecycle;
use crate::layers::ruby::RubyLayerLifecycle;
use libcnb::data::build_plan::BuildPlan;
use libcnb::layer_lifecycle::execute_layer_lifecycle;
use libcnb::{cnb_runtime, BuildContext, DetectContext, DetectOutcome, GenericPlatform};
use libcnb::{data, Buildpack};
use serde::Deserialize;
mod layers;

struct RubyBuildpack;
impl Buildpack for RubyBuildpack {
    type Platform = GenericPlatform;
    type Metadata = RubyBuildpackMetadata;
    type Error = anyhow::Error;

    fn detect(&self, context: DetectContext<Self>) -> libcnb::Result<DetectOutcome, Self::Error> {
        let outcome = if context.app_dir.join("Gemfile.lock").exists() {
            DetectOutcome::Pass(BuildPlan::new())
        } else {
            DetectOutcome::Fail
        };

        Ok(outcome)
    }

    fn build(&self, context: BuildContext<Self>) -> libcnb::Result<(), Self::Error> {
        println!("---> Ruby Buildpack");
        println!("---> Download and extracting Ruby");

        let ruby_env = execute_layer_lifecycle("ruby", RubyLayerLifecycle, &context)?;

        println!("---> Installing bundler");
        install_bundler(&ruby_env)?;
        execute_layer_lifecycle("bundler", BundlerLayerLifecycle { ruby_env }, &context)?;

        write_launch(&context)?;
        Ok(())
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

fn write_launch<B: Buildpack>(context: &BuildContext<B>) -> anyhow::Result<()> {
    let mut launch_toml = data::launch::Launch::new();
    let web =
        data::launch::Process::new("web", "bundle", vec!["exec", "ruby", "app.rb"], false, true)?;

    let worker = data::launch::Process::new(
        "worker",
        "bundle",
        vec!["exec", "ruby", "worker.rb"],
        false,
        false,
    )?;

    launch_toml.processes.push(web);
    launch_toml.processes.push(worker);

    context.write_launch(launch_toml)?;
    Ok(())
}
