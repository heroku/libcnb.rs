use std::env;
use std::ffi::OsStr;
use std::path::{Path, PathBuf};
use std::process;
use std::process::exit;

use serde::de::DeserializeOwned;

use crate::build::{BuildContext, InnerBuildResult};
use crate::buildpack::Buildpack;
use crate::data::buildpack::BuildpackToml;
use crate::detect::{DetectContext, InnerDetectResult};
use crate::error::Error;
use crate::platform::Platform;
use crate::toml_file::{read_toml_file, write_toml_file};
use crate::{Result, LIBCNB_SUPPORTED_BUILDPACK_API};
use std::fmt::{Debug, Display};

/// Main entry point for this framework.
///
/// The Buildpack API requires us to have separate entry points for each of `bin/{detect,build}`.
/// In order to save the compile time and buildpack size of having two very similar binaries, a
/// single binary is built instead, with the filename by which it is invoked being used to determine
/// the mode in which it is being run. The desired filenames are then created as symlinks or
/// hard links to this single binary.
///
/// Currently symlinks are recommended over hard hard links due to [buildpacks/pack#1286](https://github.com/buildpacks/pack/issues/1286).
pub fn libcnb_runtime<B: Buildpack>(buildpack: B) {
    match read_buildpack_toml::<B::Metadata, B::Error>() {
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
            exit(buildpack.handle_error(lib_cnb_error));
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
        Some("detect") => libcnb_runtime_detect(&buildpack),
        Some("build") => libcnb_runtime_build(&buildpack),
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
        exit(buildpack.handle_error(lib_cnb_error));
    }
}

fn libcnb_runtime_detect<B: Buildpack>(buildpack: &B) -> Result<(), B::Error> {
    let args = parse_detect_args_or_exit();

    let app_dir = env::current_dir().map_err(Error::CannotDetermineAppDirectory)?;

    let stack_id: String = env::var("CNB_STACK_ID").map_err(Error::CannotDetermineStackId)?;

    let platform = B::Platform::from_path(&args.platform_dir_path)
        .map_err(Error::CannotCreatePlatformFromPath)?;

    let build_plan_path = args.build_plan_path;

    let detect_context = DetectContext {
        app_dir,
        stack_id,
        platform,
        buildpack_dir: read_buildpack_dir()?,
        buildpack_descriptor: read_buildpack_toml()?,
    };

    match buildpack.detect(detect_context)?.0 {
        InnerDetectResult::Fail => process::exit(100),
        InnerDetectResult::Pass { build_plan } => {
            if let Some(build_plan) = build_plan {
                write_toml_file(&build_plan, build_plan_path)
                    .map_err(Error::CannotWriteBuildPlan)?;
            }

            process::exit(0)
        }
    }
}

fn libcnb_runtime_build<B: Buildpack>(buildpack: &B) -> Result<(), B::Error> {
    let args = parse_build_args_or_exit();

    let layers_dir = args.layers_dir_path;

    let app_dir = env::current_dir().map_err(Error::CannotDetermineAppDirectory)?;

    let stack_id: String = env::var("CNB_STACK_ID").map_err(Error::CannotDetermineStackId)?;

    let platform = B::Platform::from_path(&args.platform_dir_path)
        .map_err(Error::CannotCreatePlatformFromPath)?;

    let buildpack_plan =
        read_toml_file(&args.buildpack_plan_path).map_err(Error::CannotReadBuildpackPlan)?;

    let build_result = buildpack.build(BuildContext {
        layers_dir: layers_dir.clone(),
        app_dir,
        stack_id,
        platform,
        buildpack_plan,
        buildpack_dir: read_buildpack_dir()?,
        buildpack_descriptor: read_buildpack_toml()?,
    })?;

    match build_result.0 {
        InnerBuildResult::Pass { launch, store } => {
            if let Some(launch) = launch {
                write_toml_file(&launch, layers_dir.join("launch.toml"))
                    .map_err(Error::CannotWriteLaunch)?;
            };

            if let Some(store) = store {
                write_toml_file(&store, layers_dir.join("store.toml"))
                    .map_err(Error::CannotWriteStore)?;
            };

            process::exit(0)
        }
    }
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
