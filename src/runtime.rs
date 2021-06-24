use std::env;
use std::error::Error;
use std::path::PathBuf;
use std::process;
use std::process::exit;

use serde::de::DeserializeOwned;

use crate::build::BuildContext;
use crate::detect::{DetectContext, DetectResult};
use crate::error::{LibCnbError, LibCnbErrorHandle};
use crate::platform::Platform;
use crate::shared::{read_toml_file, write_toml_file};

pub fn cnb_runtime<P: Platform, BM: DeserializeOwned, E: Error>(
    detect_fn: impl Fn(DetectContext<P, BM>) -> Result<DetectResult, LibCnbError<E>>,
    build_fn: impl Fn(BuildContext<P, BM>) -> Result<(), LibCnbError<E>>,
    error_handler: impl LibCnbErrorHandle<E>,
) {
    let current_exe = std::env::current_exe().ok();
    let current_exe_file_name = current_exe
        .as_ref()
        .and_then(|path| path.file_name())
        .and_then(|file_name| file_name.to_str());

    #[cfg(any(target_family = "unix"))]
    let result = match current_exe_file_name {
        Some("detect") => cnb_runtime_detect(detect_fn),
        Some("build") => cnb_runtime_build(build_fn),
        Some(_) | None => exit(255),
    };

    if let Err(lib_cnb_error) = result {
        exit(error_handler.handle_error(lib_cnb_error));
    }
}

fn cnb_runtime_detect<
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

fn cnb_runtime_build<
    E: Error,
    F: Fn(BuildContext<P, BM>) -> Result<(), LibCnbError<E>>,
    BM: DeserializeOwned,
    P: Platform,
>(
    build_fn: F,
) -> Result<(), LibCnbError<E>> {
    let args = parse_build_args_or_exit();

    let layers_dir = args.layers_dir_path;

    let app_dir = env::current_dir().map_err(LibCnbError::CannotDetermineAppDirectory)?;

    let buildpack_dir = env::var("CNB_BUILDPACK_DIR")
        .map_err(LibCnbError::CannotDetermineBuildpackDirectory)
        .map(PathBuf::from)?;

    let stack_id: String = env::var("CNB_STACK_ID").map_err(LibCnbError::CannotDetermineStackId)?;

    let platform =
        P::from_path(&args.platform_dir_path).map_err(LibCnbError::CannotCreatePlatformFromPath)?;

    let buildpack_plan =
        read_toml_file(&args.buildpack_plan_path).map_err(LibCnbError::CannotReadBuildpackPlan)?;

    let buildpack_descriptor = read_toml_file(buildpack_dir.join("buildpack.toml"))
        .map_err(LibCnbError::CannotReadBuildpackDescriptor)?;

    let context = BuildContext {
        layers_dir,
        app_dir,
        buildpack_dir,
        stack_id,
        platform,
        buildpack_plan,
        buildpack_descriptor,
    };

    build_fn(context)
}

struct DetectArgs {
    pub platform_dir_path: PathBuf,
    pub build_plan_path: PathBuf,
}

struct BuildArgs {
    pub layers_dir_path: PathBuf,
    pub platform_dir_path: PathBuf,
    pub buildpack_plan_path: PathBuf,
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

fn parse_build_args_or_exit() -> BuildArgs {
    let args: Vec<String> = env::args().collect();
    match args.as_slice() {
        [_, layers_dir_path, platform_dir_path, buildpack_plan_path] => BuildArgs {
            layers_dir_path: PathBuf::from(layers_dir_path),
            platform_dir_path: PathBuf::from(platform_dir_path),
            buildpack_plan_path: PathBuf::from(buildpack_plan_path),
        },
        _ => {
            eprintln!("Usage: build <layers> <platform> <plan>");
            eprintln!("https://github.com/buildpacks/spec/blob/main/buildpack.md#build");
            process::exit(1);
        }
    }
}
