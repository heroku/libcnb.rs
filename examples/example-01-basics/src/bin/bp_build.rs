use libcnb;
use libcnb::build::BuildContext;
use libcnb::platform::{GenericPlatform, Platform};
use libcnb::shared::BuildpackError;
use std::error::Error;

fn main() {
    libcnb::build::cnb_runtime_build(build);
}

fn build(context: BuildContext<GenericPlatform>) -> Result<(), std::io::Error> {
    println!("Build runs!");
    Ok(())
}
