use crate::data::buildpack::BuildpackToml;
use crate::shared::read_toml_file;
use crate::{
    data::build_plan::BuildPlan, platform::Platform, shared::write_toml_file, LibCnbError,
};
use serde::de::DeserializeOwned;
use std::error::Error;
use std::fmt::Debug;
use std::{env, path::PathBuf, process};

pub fn cnb_runtime_detect<
    P: Platform,
    BM: DeserializeOwned,
    E: Error,
    F: FnOnce(DetectContext<P, BM>) -> Result<DetectResult, LibCnbError<E>>,
>(
    detect_fn: F,
) -> Result<(), LibCnbError<E>> {
    let app_dir = env::current_dir().expect("Could not determine current working directory!");

    let buildpack_dir: PathBuf = env::var("CNB_BUILDPACK_DIR")
        .expect("Could not determine buildpack directory!")
        .into();

    let args: Vec<String> = env::args().collect();
    if args.len() != 3 {
        eprintln!("Usage: detect <platform_dir> <buildplan>");
        process::exit(1);
    }

    let platform = {
        let platform_dir = PathBuf::from(args.get(1).unwrap());

        if !platform_dir.is_dir() {
            eprintln!("First argument must be a readable platform directory!");
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

    let stack_id = env::var("CNB_STACK_ID").expect("Could not determine CNB stack id!");

    let build_plan_path: PathBuf = PathBuf::from(args.get(2).unwrap());

    let buildpack_descriptor = match read_toml_file(buildpack_dir.join("buildpack.toml")) {
        Ok(buildpack_descriptor) => buildpack_descriptor,
        Err(error) => {
            eprintln!("Could not read buildpack descriptor: {}", error);
            process::exit(1);
        }
    };

    let detect_context = DetectContext {
        app_dir,
        buildpack_dir,
        stack_id,
        platform,
        buildpack_descriptor,
    };

    match detect_fn(detect_context) {
        Ok(DetectResult::Fail) | Err(_) => process::exit(100),
        Ok(DetectResult::Error(code)) => process::exit(code),
        Ok(DetectResult::Pass(build_plan)) => {
            if let Err(error) = write_toml_file(&build_plan, build_plan_path) {
                eprintln!("Could not write buildplan to disk: {}", error);
                process::exit(1);
            }

            process::exit(0)
        }
    };
}

pub struct DetectContext<P: Platform, BM> {
    pub app_dir: PathBuf,
    pub buildpack_dir: PathBuf,
    pub stack_id: String,
    pub platform: P,
    pub buildpack_descriptor: BuildpackToml<BM>,
}

#[derive(Debug)]
pub enum DetectResult {
    Pass(BuildPlan),
    Fail,
    Error(i32),
}
