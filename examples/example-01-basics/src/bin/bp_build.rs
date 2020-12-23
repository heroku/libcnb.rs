use libcnb;
use std::error::Error;
use libcnb::shared::GenericPlatform;
use libcnb::shared::Platform;
use libcnb::shared::BuildFromPath;
use libcnb::build::BuildContext;

fn main() {
    libcnb::build::cnb_runtime_build(build);
}

fn build(context: BuildContext<GenericPlatform>) -> Result<(), Box<dyn Error>> {
    println!("Build runs!");
    Ok(())
}
