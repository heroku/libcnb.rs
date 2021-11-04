use std::collections::HashMap;
use std::process::{Command, Stdio};

use anyhow::Error;
use libcnb::{BuildContext, cnb_runtime, DetectContext, DetectOutcome, GenericErrorHandler, GenericPlatform};
use libcnb::data::build_plan::BuildPlan;
use libcnb::data;
use libcnb::layer_lifecycle::execute_layer_lifecycle;
use serde::Deserialize;

use crate::layers::bundler::BundlerLayerLifecycle;
use crate::layers::ruby;
use crate::layers::ruby::RubyLayerLifecycle;

mod layers;

fn main() {
    cnb_runtime(detect, build, GenericErrorHandler)
}

fn detect(context: DetectContext<GenericPlatform, RubyBuildpackMetadata>) -> libcnb::Result<DetectOutcome, anyhow::Error> {
    let outcome = if context.app_dir.join("Gemfile.lock").exists() {
        DetectOutcome::Pass(BuildPlan::new())
    } else {
        DetectOutcome::Fail
    };

    Ok(outcome)
}

fn build(context: BuildContext<GenericPlatform, RubyBuildpackMetadata>) -> libcnb::Result<(), anyhow::Error> {
    println!("---> Ruby Buildpack");
    println!("---> Download and extracting Ruby");

    let ruby_env = execute_layer_lifecycle("ruby", RubyLayerLifecycle, &context)?;

    println!("---> Installing bundler");
    install_bundler(&ruby_env)?;
    execute_layer_lifecycle("bundler", BundlerLayerLifecycle { ruby_env }, &context)?;

    write_launch(&context);
    Ok(())
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

fn write_launch(context: &BuildContext<GenericPlatform, RubyBuildpackMetadata>) -> anyhow::Result<()> {
    let mut launch_toml = data::launch::Launch::new();
    let web = data::launch::Process::new("web", "bundle", vec!["exec", "ruby", "app.rb"], false)?;
    let worker =
        data::launch::Process::new("worker", "bundle", vec!["exec", "ruby", "worker.rb"], false)?;
    launch_toml.processes.push(web);
    launch_toml.processes.push(worker);

    context.write_launch(launch_toml)?;
    Ok(())
}
