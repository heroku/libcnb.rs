use crate::{
    data::{buildpack::BuildpackToml, buildpack_plan::BuildpackPlan},
    platform::{GenericPlatform, Platform},
    shared::{read_toml_file, BuildpackError},
};
use std::{env, path::PathBuf, process};

pub fn cnb_runtime_build<
    E: BuildpackError,
    F: Fn(BuildContext<P>) -> Result<(), E>,
    P: Platform,
>(
    build_fn: F,
) {
    let app_dir = env::current_dir().expect("Could not determine current working directory!");

    let buildpack_dir: PathBuf = env::var("CNB_BUILDPACK_DIR")
        .expect("Could not determine buildpack directory!")
        .into();

    let stack_id: String = env::var("CNB_STACK_ID")
        .expect("Could not determine CNB stack id!")
        .into();

    let args: Vec<String> = env::args().collect();
    if args.len() != 4 {
        eprintln!("Usage: build <layers> <platform> <plan>");
        process::exit(1);
    }

    let layers_dir: PathBuf = args.get(1).unwrap().into();

    let platform = {
        let platform_dir = PathBuf::from(args.get(2).unwrap());

        if !platform_dir.is_dir() {
            eprintln!("Second argument must be a readable platform directory!");
            process::exit(1);
        }

        match P::from_path(platform_dir.as_path()) {
            Ok(platform) => platform,
            Err(error) => {
                eprintln!(
                    "Could not create platform from platform directory: {}",
                    error
                );
                process::exit(1);
            }
        }
    };

    let buildpack_plan = {
        let buildpack_plan_path: PathBuf = args.get(3).unwrap().into();
        match read_toml_file(buildpack_plan_path) {
            Ok(Some(buildpack_plan)) => buildpack_plan,
            Ok(None) => {
                eprintln!("Buildpack plan is malformed!");
                process::exit(1);
            }
            Err(error) => {
                eprintln!("Could not read buildpack plan: {}", error);
                process::exit(1);
            }
        }
    };

    let buildpack_toml_path = buildpack_dir.join("buildpack.toml");
    let buildpack_descriptor = match read_toml_file(buildpack_toml_path) {
        Ok(Some(buildpack_descriptor)) => buildpack_descriptor,
        Ok(None) => {
            eprintln!("Buildpack descriptor is malformed!");
            process::exit(1);
        }
        Err(error) => {
            eprintln!("Could not read buildpack descriptor: {}", error);
            process::exit(1);
        }
    };

    let context = BuildContext {
        layers_dir,
        app_dir,
        buildpack_dir,
        stack_id,
        platform,
        buildpack_plan,
        buildpack_descriptor,
    };

    match build_fn(context) {
        Err(error) => {
            eprintln!("Unhandled error during build: {}", error);
            process::exit(-1);
        }
        _ => process::exit(0),
    };
}

pub struct BuildContext<P> {
    layers_dir: PathBuf,
    pub app_dir: PathBuf,
    pub buildpack_dir: PathBuf,
    pub stack_id: String,
    pub platform: P,
    pub buildpack_plan: BuildpackPlan,
    pub buildpack_descriptor: BuildpackToml,
}

pub type GenericBuildContext = BuildContext<GenericPlatform>;
