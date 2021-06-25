use std::env;
use std::path::PathBuf;
use std::process;
use std::process::exit;

use serde::de::DeserializeOwned;

use crate::build::BuildContext;
use crate::detect::{DetectContext, DetectOutcome};
use crate::error::{Error, ErrorHandler};
use crate::platform::Platform;
use crate::toml_file::{read_toml_file, write_toml_file};
use crate::Result;

/// Main entry point for this framework.
///
/// # Example
/// ```no_run
/// use libcnb::{GenericErrorHandler, DetectOutcome, Error, GenericBuildContext, GenericDetectContext, Result};
///
/// fn detect(context: GenericDetectContext) -> Result<DetectOutcome, std::io::Error> {
///     // ...
///     Ok(DetectOutcome::Fail)
/// }
///
/// fn build(context: GenericBuildContext) -> Result<(), std::io::Error> {
///    // ...
///    Ok(())
/// }
///
/// fn main() {
///    libcnb::cnb_runtime(detect, build, GenericErrorHandler);
/// }
/// ```
pub fn cnb_runtime<P: Platform, BM: DeserializeOwned, E: std::error::Error>(
    detect_fn: impl Fn(DetectContext<P, BM>) -> Result<DetectOutcome, E>,
    build_fn: impl Fn(BuildContext<P, BM>) -> Result<(), E>,
    error_handler: impl ErrorHandler<E>,
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
    E: std::error::Error,
    F: FnOnce(DetectContext<P, BM>) -> Result<DetectOutcome, E>,
>(
    detect_fn: F,
) -> Result<(), E> {
    let args = parse_detect_args_or_exit();

    let app_dir = env::current_dir().map_err(Error::CannotDetermineAppDirectory)?;

    let buildpack_dir = env::var("CNB_BUILDPACK_DIR")
        .map_err(Error::CannotDetermineBuildpackDirectory)
        .map(PathBuf::from)?;

    let stack_id: String = env::var("CNB_STACK_ID").map_err(Error::CannotDetermineStackId)?;

    let platform =
        P::from_path(&args.platform_dir_path).map_err(Error::CannotCreatePlatformFromPath)?;

    let buildpack_descriptor = read_toml_file(buildpack_dir.join("buildpack.toml"))
        .map_err(Error::CannotReadBuildpackDescriptor)?;

    let build_plan_path = args.build_plan_path;

    let detect_context = DetectContext {
        app_dir,
        buildpack_dir,
        stack_id,
        platform,
        buildpack_descriptor,
    };

    match detect_fn(detect_context)? {
        DetectOutcome::Pass(build_plan) => {
            write_toml_file(&build_plan, build_plan_path).map_err(Error::CannotWriteBuildPlan)?;
            process::exit(0)
        }
        DetectOutcome::Fail => process::exit(100),
    }
}

fn cnb_runtime_build<
    E: std::error::Error,
    F: Fn(BuildContext<P, BM>) -> Result<(), E>,
    BM: DeserializeOwned,
    P: Platform,
>(
    build_fn: F,
) -> Result<(), E> {
    let args = parse_build_args_or_exit();

    let layers_dir = args.layers_dir_path;

    let app_dir = env::current_dir().map_err(Error::CannotDetermineAppDirectory)?;

    let buildpack_dir = env::var("CNB_BUILDPACK_DIR")
        .map_err(Error::CannotDetermineBuildpackDirectory)
        .map(PathBuf::from)?;

    let stack_id: String = env::var("CNB_STACK_ID").map_err(Error::CannotDetermineStackId)?;

    let platform =
        P::from_path(&args.platform_dir_path).map_err(Error::CannotCreatePlatformFromPath)?;

    let buildpack_plan =
        read_toml_file(&args.buildpack_plan_path).map_err(Error::CannotReadBuildpackPlan)?;

    let buildpack_descriptor = read_toml_file(buildpack_dir.join("buildpack.toml"))
        .map_err(Error::CannotReadBuildpackDescriptor)?;

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
