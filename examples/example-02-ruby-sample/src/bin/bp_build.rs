use std::{
    collections::HashMap,
    env, fs, io,
    path::Path,
    process::{Command, Stdio},
};

use example_02_ruby_sample::ruby_layer::RubyLayer;
use flate2::read::GzDecoder;
use libcnb::data::layer::LayerContentMetadata;
use libcnb::layer_lifecycle::execute_layer_lifecycle;
use libcnb::{
    build::{cnb_runtime_build, GenericBuildContext},
    data,
};
use serde::{Deserialize, Serialize};
use sha2::Digest;
use tar::Archive;
use tempfile::NamedTempFile;

const RUBY_URL: &str =
    "https://s3-external-1.amazonaws.com/heroku-buildpack-ruby/heroku-18/ruby-2.5.1.tgz";

fn main() -> anyhow::Result<()> {
    cnb_runtime_build(build);

    Ok(())
}

// need to add a logger / printing to stdout?
fn build(context: GenericBuildContext<Option<toml::value::Table>>) -> anyhow::Result<()> {
    println!("---> Ruby Buildpack");
    println!("---> Download and extracting Ruby");

    execute_layer_lifecycle("ruby", RubyLayer {}, context)?;

    let mut ruby_env: HashMap<String, String> = HashMap::new();

    let ruby_bin_path = format!(
        "{}/.gem/ruby/2.6.6/bin",
        env::var("HOME").unwrap_or(String::new())
    );

    ruby_env.insert(
        String::from("PATH"),
        format!(
            "{}:{}:{}",
            ruby_layer.as_ref().join("bin").as_path().to_str().unwrap(),
            ruby_bin_path,
            env::var("PATH").unwrap_or(String::new()),
        ),
    );

    ruby_env.insert(
        String::from("LD_LIBRARY_PATH"),
        format!(
            "{}:{}",
            env::var("LD_LIBRARY_PATH").unwrap_or(String::new()),
            ruby_layer
                .as_ref()
                .join("layer")
                .as_path()
                .to_str()
                .unwrap()
        ),
    );

    println!("---> Installing bundler");
    {
        let cmd = Command::new("gem")
            .args(&["install", "bundler", "--no-ri", "--no-rdoc"])
            .envs(&ruby_env)
            .spawn()?
            .wait()?;

        if !cmd.success() {
            anyhow::anyhow!("Could not install bundler");
        }
    }

    let bundler_layer_existed = context.layer_exists("bundler");
    let local_gemfile_checksum = sha256_checksum(context.app_dir.join("Gemfile.lock"))?;

    let bundler_layer = context.read_or_new_layer(
        "bundler",
        LayerContentMetadata::default()
            .cache(true)
            .build(true)
            .metadata(BundlerLayerMetadata {
                gemfile_lock_checksum: local_gemfile_checksum.clone(),
            }),
    )?;

    let bundler_layer_path = bundler_layer.as_ref();
    let bundler_layer_binstubs_path = bundler_layer_path.join("bin");

    if bundler_layer_existed
        && bundler_layer
            .content_metadata
            .metadata
            .gemfile_lock_checksum
            != local_gemfile_checksum
    {
        println!("---> Reusing gems");
        Command::new("bundle")
            .args(&[
                "config",
                "--local",
                "path",
                bundler_layer_path.to_str().unwrap(),
            ])
            .envs(&ruby_env)
            .spawn()?
            .wait()?;

        Command::new("bundle")
            .args(&[
                "config",
                "--local",
                "bin",
                bundler_layer_binstubs_path.as_path().to_str().unwrap(),
            ])
            .envs(&ruby_env)
            .spawn()?
            .wait()?;
    } else {
        println!("---> Installing gems");
        let cmd = Command::new("bundle")
            .args(&[
                "install",
                "--path",
                bundler_layer_path.to_str().unwrap(),
                "--binstubs",
                bundler_layer_binstubs_path.as_path().to_str().unwrap(),
            ])
            .envs(&ruby_env)
            .stdout(Stdio::inherit())
            .stderr(Stdio::inherit())
            .spawn()?
            .wait()?;
        if !cmd.success() {
            anyhow::anyhow!("Could not bundle install");
        }
    }

    let mut launch_toml = data::launch::Launch::new();
    let web = data::launch::Process::new("web", "bundle", vec!["exec", "ruby", "app.rb"], false)?;
    let worker =
        data::launch::Process::new("worker", "bundle", vec!["exec", "ruby", "worker.rb"], false)?;
    launch_toml.processes.push(web);
    launch_toml.processes.push(worker);

    context.write_launch(launch_toml)?;

    Ok(())
}

#[derive(Serialize, Deserialize)]
struct BundlerLayerMetadata {
    gemfile_lock_checksum: String,
}
