use std::env;
use std::ffi::OsStr;
use std::path::{Path, PathBuf};
use std::process;
use std::process::exit;

use serde::de::DeserializeOwned;

use crate::build::BuildContext;
use crate::data::buildpack::BuildpackToml;
use crate::data::stack_id::StackId;
use crate::detect::{DetectContext, DetectOutcome};
use crate::error::{Error, ErrorHandler};
use crate::platform::Platform;
use crate::toml_file::{read_toml_file, write_toml_file};
use crate::{Result, LIBCNB_SUPPORTED_BUILDPACK_API};
use std::fmt::{Debug, Display};
use std::str::FromStr;

/// Main entry point for this framework.
///
/// The Buildpack API requires us to have separate entry points for each of `bin/{detect,build}`.
/// In order to save the compile time and buildpack size of having two very similar binaries, a
/// single binary is built instead, with the filename by which it is invoked being used to determine
/// the mode in which it is being run. The desired filenames are then created as symlinks or
/// hard links to this single binary.
///
/// Currently symlinks are recommended over hard hard links due to [buildpacks/pack#1286](https://github.com/buildpacks/pack/issues/1286).
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
pub fn cnb_runtime<P: Platform, BM: DeserializeOwned, E: Debug + Display>(
    detect_fn: impl Fn(DetectContext<P, BM>) -> Result<DetectOutcome, E>,
    build_fn: impl Fn(BuildContext<P, BM>) -> Result<(), E>,
    error_handler: impl ErrorHandler<E>,
) {
    match read_buildpack_toml::<BM, E>() {
        Ok(buildpack_toml) => {
            if buildpack_toml.api != LIBCNB_SUPPORTED_BUILDPACK_API {
                eprintln!("Error: Cloud Native Buildpack API mismatch");
                eprintln!(
                    "This buildpack ({}) uses Cloud Native Buildpacks API version {}.",
                    &buildpack_toml.buildpack.name, &buildpack_toml.api,
                );

                eprintln!(
                    "But the underlying libcnb.rs library requires CNB API {}.",
                    LIBCNB_SUPPORTED_BUILDPACK_API
                );

                exit(254)
            }
        }
        Err(lib_cnb_error) => {
            exit(error_handler.handle_error(lib_cnb_error));
        }
    }

    // Using `std::env::args()` instead of `std::env::current_exe()` since the latter resolves
    // symlinks to their target on some platforms, whereas we need the original filename.
    let current_exe = env::args().next();
    let current_exe_file_name = current_exe
        .as_ref()
        .map(Path::new)
        .and_then(Path::file_name)
        .and_then(OsStr::to_str);

    #[cfg(any(target_family = "unix"))]
    let result = match current_exe_file_name {
        Some("detect") => cnb_runtime_detect(detect_fn),
        Some("build") => cnb_runtime_build(build_fn),
        other => {
            eprintln!(
                "Error: Expected the name of this executable to be 'detect' or 'build', but it was '{}'",
                other.unwrap_or("<unknown>")
            );

            eprintln!("The executable name is used to determine the current buildpack phase.");
            eprintln!("You might want to create 'detect' and 'build' links to this executable and run those instead.");
            exit(255)
        }
    };

    if let Err(lib_cnb_error) = result {
        exit(error_handler.handle_error(lib_cnb_error));
    }
}

fn cnb_runtime_detect<
    P: Platform,
    BM: DeserializeOwned,
    E: Debug + Display,
    F: FnOnce(DetectContext<P, BM>) -> Result<DetectOutcome, E>,
>(
    detect_fn: F,
) -> Result<(), E> {
    let args = parse_detect_args_or_exit();

    let app_dir = env::current_dir().map_err(Error::CannotDetermineAppDirectory)?;

    let stack_id: StackId = env::var("CNB_STACK_ID")
        .map_err(Error::CannotDetermineStackId)
        .and_then(|stack_id_string| {
            StackId::from_str(&stack_id_string).map_err(Error::StackIdError)
        })?;

    let platform =
        P::from_path(&args.platform_dir_path).map_err(Error::CannotCreatePlatformFromPath)?;

    let build_plan_path = args.build_plan_path;

    let detect_context = DetectContext {
        app_dir,
        stack_id,
        platform,
        buildpack_dir: read_buildpack_dir()?,
        buildpack_descriptor: read_buildpack_toml()?,
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
    E: Debug + Display,
    F: Fn(BuildContext<P, BM>) -> Result<(), E>,
    BM: DeserializeOwned,
    P: Platform,
>(
    build_fn: F,
) -> Result<(), E> {
    let args = parse_build_args_or_exit();

    let layers_dir = args.layers_dir_path;

    let app_dir = env::current_dir().map_err(Error::CannotDetermineAppDirectory)?;

    let stack_id: StackId = env::var("CNB_STACK_ID")
        .map_err(Error::CannotDetermineStackId)
        .and_then(|stack_id_string| {
            StackId::from_str(&stack_id_string).map_err(Error::StackIdError)
        })?;

    let platform =
        P::from_path(&args.platform_dir_path).map_err(Error::CannotCreatePlatformFromPath)?;

    let buildpack_plan =
        read_toml_file(&args.buildpack_plan_path).map_err(Error::CannotReadBuildpackPlan)?;

    let context = BuildContext {
        layers_dir,
        app_dir,
        stack_id,
        platform,
        buildpack_plan,
        buildpack_dir: read_buildpack_dir()?,
        buildpack_descriptor: read_buildpack_toml()?,
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
    if let [_, platform_dir_path, build_plan_path] = args.as_slice() {
        DetectArgs {
            platform_dir_path: PathBuf::from(platform_dir_path),
            build_plan_path: PathBuf::from(build_plan_path),
        }
    } else {
        eprintln!("Usage: detect <platform_dir> <buildplan>");
        eprintln!("https://github.com/buildpacks/spec/blob/main/buildpack.md#detection");
        process::exit(1);
    }
}

fn parse_build_args_or_exit() -> BuildArgs {
    let args: Vec<String> = env::args().collect();
    if let [_, layers_dir_path, platform_dir_path, buildpack_plan_path] = args.as_slice() {
        BuildArgs {
            layers_dir_path: PathBuf::from(layers_dir_path),
            platform_dir_path: PathBuf::from(platform_dir_path),
            buildpack_plan_path: PathBuf::from(buildpack_plan_path),
        }
    } else {
        eprintln!("Usage: build <layers> <platform> <plan>");
        eprintln!("https://github.com/buildpacks/spec/blob/main/buildpack.md#build");
        process::exit(1);
    }
}

fn read_buildpack_dir<E: Display + Debug>() -> Result<PathBuf, E> {
    env::var("CNB_BUILDPACK_DIR")
        .map_err(Error::CannotDetermineBuildpackDirectory)
        .map(PathBuf::from)
}

fn read_buildpack_toml<BM: DeserializeOwned, E: Display + Debug>() -> Result<BuildpackToml<BM>, E> {
    read_buildpack_dir().and_then(|buildpack_dir| {
        read_toml_file(buildpack_dir.join("buildpack.toml"))
            .map_err(Error::CannotReadBuildpackDescriptor)
    })
}
