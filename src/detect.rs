use std::error::Error;
use std::fmt::Debug;
use std::{env, path::PathBuf, process};

use serde::de::DeserializeOwned;

use crate::data::buildpack::BuildpackToml;
use crate::error::LibCnbError;
use crate::shared::read_toml_file;
use crate::{data::build_plan::BuildPlan, platform::Platform, shared::write_toml_file};

pub fn cnb_runtime_detect<
    P: Platform,
    BM: DeserializeOwned,
    E: Error,
    F: FnOnce(DetectContext<P, BM>) -> Result<DetectResult, LibCnbError<E>>,
>(
    detect_fn: F,
) -> Result<(), LibCnbError<E>> {
    let args = parse_detect_args_or_exit();

    let app_dir = env::current_dir().map_err(LibCnbError::CannotDetermineAppDirectory)?;

    let buildpack_dir = env::var("CNB_BUILDPACK_DIR")
        .map_err(LibCnbError::CannotDetermineBuildpackDirectory)
        .map(PathBuf::from)?;

    let stack_id: String = env::var("CNB_STACK_ID").map_err(LibCnbError::CannotDetermineStackId)?;

    let platform =
        P::from_path(&args.platform_dir_path).map_err(LibCnbError::CannotCreatePlatformFromPath)?;

    let buildpack_descriptor = read_toml_file(buildpack_dir.join("buildpack.toml"))
        .map_err(LibCnbError::CannotReadBuildpackDescriptor)?;

    let build_plan_path = args.build_plan_path;

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
            write_toml_file(&build_plan, build_plan_path)
                .map_err(LibCnbError::CannotWriteBuildPlan)?;

            process::exit(0)
        }
    };
}

struct DetectArgs {
    pub platform_dir_path: PathBuf,
    pub build_plan_path: PathBuf,
}

fn parse_detect_args_or_exit() -> DetectArgs {
    let args: Vec<String> = env::args().collect();
    match args.as_slice() {
        [_, platform_dir_path, build_plan_path] => DetectArgs {
            platform_dir_path: PathBuf::from(platform_dir_path),
            build_plan_path: PathBuf::from(build_plan_path),
        },
        _ => {
            eprintln!("Usage: detect <platform_dir> <buildplan>");
            eprintln!("https://github.com/buildpacks/spec/blob/main/buildpack.md#detection");
            process::exit(1);
        }
    }
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
