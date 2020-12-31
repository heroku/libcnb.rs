use flate2::read::GzDecoder;
use libcnb::build::{cnb_runtime_build, GenericBuildContext};
use std::{
    collections::HashMap,
    fs::File,
    io,
    path::Path,
    process::{Command, Stdio},
};
use tar::Archive;
use tempfile::NamedTempFile;

const RUBY_URL: &str =
    "https://s3-external-1.amazonaws.com/heroku-buildpack-ruby/heroku-18/ruby-2.5.1.tgz";

fn main() -> anyhow::Result<()> {
    cnb_runtime_build(build);

    Ok(())
}

// need to add a logger / printing to stdout?
fn build(ctx: GenericBuildContext) -> anyhow::Result<()> {
    println!("---> Ruby Buildpack");

    println!("---> Download and extracting Ruby");
    let mut ruby_layer = ctx.layer("ruby")?;
    ruby_layer.mut_content_metadata().launch = true;
    ruby_layer.write_content_metadata()?;
    {
        let ruby_tgz = NamedTempFile::new()?;
        download(RUBY_URL, ruby_tgz.path())?;
        untar(ruby_tgz.path(), ruby_layer.as_path())?;
    }

    let mut ruby_env: HashMap<String, String> = HashMap::new();
    ruby_env.insert(
        String::from("PATH"),
        format!(
            "{}:$PATH",
            ruby_layer.as_path().join("bin").as_path().to_str().unwrap()
        ),
    );
    ruby_env.insert(
        String::from("LD_LIBRARY_PATH"),
        format!(
            r#"${{LD_LIBRARY_PATH:+${{LD_LIBRARY_PATH}}:}}"{}""#,
            ruby_layer
                .as_path()
                .join("layer")
                .as_path()
                .to_str()
                .unwrap()
        ),
    );
    println!("---> Installing bundler");
    Command::new("gem")
        .args(&["install", "bundler", "--no-ri", "--no-rdoc"])
        .envs(&ruby_env)
        .stdout(Stdio::inherit())
        .stderr(Stdio::inherit())
        .spawn()?;

    println!("---> Installing gems");
    Command::new("bundle")
        .arg("install")
        .envs(&ruby_env)
        .stdout(Stdio::inherit())
        .stderr(Stdio::inherit())
        .spawn()?;

    Ok(())
}

fn download(uri: impl AsRef<str>, dst: impl AsRef<Path>) -> anyhow::Result<()> {
    let response = reqwest::blocking::get(uri.as_ref())?;
    let content = response.text()?;
    let mut file = File::create(dst.as_ref())?;
    io::copy(&mut content.as_bytes(), &mut file)?;

    Ok(())
}

fn untar(file: impl AsRef<Path>, dst: impl AsRef<Path>) -> anyhow::Result<()> {
    let tar_gz = File::open(file.as_ref())?;
    let tar = GzDecoder::new(tar_gz);
    let mut archive = Archive::new(tar);
    archive.unpack(dst.as_ref())?;

    Ok(())
}
