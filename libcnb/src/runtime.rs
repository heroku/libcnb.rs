use crate::build::{BuildContext, InnerBuildResult};
use crate::buildpack::Buildpack;
use crate::data::buildpack::BuildpackApi;
use crate::detect::{DetectContext, InnerDetectResult};
use crate::error::Error;
use crate::platform::Platform;
use crate::sbom::cnb_sbom_path;
#[cfg(feature = "trace")]
use crate::tracing::init_tracing;
use crate::util::is_not_found_error_kind;
use crate::{LIBCNB_SUPPORTED_BUILDPACK_API, Target, TomlFileError, exit_code};
use libcnb_common::toml_file::{read_toml_file, write_toml_file};
use libcnb_data::buildpack::ComponentBuildpackDescriptor;
use libcnb_data::store::Store;
use serde::Deserialize;
use serde::de::DeserializeOwned;
use std::ffi::OsStr;
use std::fmt::Debug;
use std::path::{Path, PathBuf};
use std::process::exit;
use std::{env, fs};

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
/// Don't implement this directly and use the [`buildpack_main`] macro instead!
#[doc(hidden)]
pub fn libcnb_runtime<B: Buildpack>(buildpack: &B) {
    // Before we do anything else, we must validate that the Buildpack's API version
    // matches that supported by libcnb, to improve the UX in cases where the lifecycle
    // passes us arguments or env vars we don't expect, due to changes between API versions.
    // We use a cut-down buildpack descriptor type, to ensure we can still read the API
    // version even if the rest of buildpack.toml doesn't match the spec (or the buildpack's
    // chosen custom `metadata` type).
    match read_buildpack_descriptor::<BuildpackDescriptorApiOnly, B::Error>() {
        Ok(buildpack_descriptor) => {
            if buildpack_descriptor.api != LIBCNB_SUPPORTED_BUILDPACK_API {
                eprintln!("Error: Cloud Native Buildpack API mismatch");
                eprintln!(
                    "This buildpack uses Cloud Native Buildpacks API version {} (specified in buildpack.toml).",
                    &buildpack_descriptor.api,
                );
                eprintln!(
                    "However, the underlying libcnb.rs library only supports CNB API {LIBCNB_SUPPORTED_BUILDPACK_API}."
                );
                exit(exit_code::GENERIC_CNB_API_VERSION_ERROR)
            }
        }
        Err(libcnb_error) => {
            // This case will likely never occur, since Pack/lifecycle validates each buildpack's
            // `buildpack.toml` before the buildpack even runs, so the file being missing or the
            // `api` key not being set will have already resulted in an error much earlier.
            eprintln!("Error: Unable to determine Buildpack API version");
            eprintln!("Cause: {libcnb_error}");
            exit(exit_code::GENERIC_CNB_API_VERSION_ERROR);
        }
    }

    let args: Vec<String> = env::args().collect();

    // Using `std::env::args()` instead of `std::env::current_exe()` since the latter resolves
    // symlinks to their target on some platforms, whereas we need the original filename.
    let current_exe = args.first();
    let current_exe_file_name = current_exe
        .map(Path::new)
        .and_then(Path::file_name)
        .and_then(OsStr::to_str);

    let result = match current_exe_file_name {
        Some("detect") => libcnb_runtime_detect(
            buildpack,
            DetectArgs::parse(&args).unwrap_or_else(|parse_error| match parse_error {
                DetectArgsParseError::InvalidArguments => {
                    eprintln!("Usage: detect <platform_dir> <buildplan>");
                    eprintln!(
                        "https://github.com/buildpacks/spec/blob/main/buildpack.md#detection"
                    );
                    exit(exit_code::GENERIC_UNSPECIFIED_ERROR);
                }
            }),
        ),
        Some("build") => libcnb_runtime_build(
            buildpack,
            BuildArgs::parse(&args).unwrap_or_else(|parse_error| match parse_error {
                BuildArgsParseError::InvalidArguments => {
                    eprintln!("Usage: build <layers> <platform> <plan>");
                    eprintln!("https://github.com/buildpacks/spec/blob/main/buildpack.md#build");
                    exit(exit_code::GENERIC_UNSPECIFIED_ERROR);
                }
            }),
        ),
        other => {
            eprintln!(
                "Error: Expected the name of this executable to be 'detect' or 'build', but it was '{}'",
                other.unwrap_or("<unknown>")
            );
            eprintln!("The executable name is used to determine the current buildpack phase.");
            eprintln!(
                "You might want to create 'detect' and 'build' links to this executable and run those instead."
            );
            exit(exit_code::GENERIC_UNEXPECTED_EXECUTABLE_NAME_ERROR)
        }
    };

    match result {
        Ok(code) => exit(code),
        Err(libcnb_error) => {
            buildpack.on_error(libcnb_error);
            exit(exit_code::GENERIC_UNSPECIFIED_ERROR);
        }
    }
}

/// Detect entry point for this framework.
///
/// Exposed only to allow for advanced use-cases where detect is programmatically invoked.
#[doc(hidden)]
pub fn libcnb_runtime_detect<B: Buildpack>(
    buildpack: &B,
    args: DetectArgs,
) -> crate::Result<i32, B::Error> {
    let app_dir = env::current_dir().map_err(Error::CannotDetermineAppDirectory)?;

    let buildpack_dir = read_buildpack_dir()?;

    let buildpack_descriptor: ComponentBuildpackDescriptor<<B as Buildpack>::Metadata> =
        read_buildpack_descriptor()?;

    #[cfg(feature = "trace")]
    let _trace_guard = init_tracing(&buildpack_descriptor.buildpack, "detect");
    #[cfg(feature = "trace")]
    let _span_guard = tracing::span!(tracing::Level::INFO, "libcnb-detect").entered();

    #[cfg(feature = "trace")]
    let trace_error =
        |error: &Error<<B as Buildpack>::Error>| tracing::error!(?error, "libcnb-detect-error");
    #[cfg(not(feature = "trace"))]
    let trace_error = |_: &Error<<B as Buildpack>::Error>| {};

    let platform = B::Platform::from_path(&args.platform_dir_path)
        .map_err(Error::CannotCreatePlatformFromPath)
        .inspect_err(trace_error)?;

    let build_plan_path = args.build_plan_path;

    let target = context_target().inspect_err(trace_error)?;

    let detect_context = DetectContext {
        app_dir,
        buildpack_dir,
        target,
        platform,
        buildpack_descriptor,
    };

    let detect_result = buildpack.detect(detect_context).inspect_err(trace_error)?;

    match detect_result.0 {
        InnerDetectResult::Fail => {
            #[cfg(feature = "trace")]
            tracing::event!(tracing::Level::INFO, "libcnb-detect-failed");
            Ok(exit_code::DETECT_DETECTION_FAILED)
        }
        InnerDetectResult::Pass { build_plan } => {
            if let Some(build_plan) = build_plan {
                write_toml_file(&build_plan, build_plan_path)
                    .map_err(Error::CannotWriteBuildPlan)
                    .inspect_err(trace_error)?;
            }
            #[cfg(feature = "trace")]
            tracing::event!(tracing::Level::INFO, "libcnb-detect-passed");
            Ok(exit_code::DETECT_DETECTION_PASSED)
        }
    }
}

/// Build entry point for this framework.
///
/// Exposed only to allow for advanced use-cases where build is programmatically invoked.
#[doc(hidden)]
pub fn libcnb_runtime_build<B: Buildpack>(
    buildpack: &B,
    args: BuildArgs,
) -> crate::Result<i32, B::Error> {
    let layers_dir = args.layers_dir_path;

    let app_dir = env::current_dir().map_err(Error::CannotDetermineAppDirectory)?;

    let buildpack_dir = read_buildpack_dir()?;

    let buildpack_descriptor: ComponentBuildpackDescriptor<<B as Buildpack>::Metadata> =
        read_buildpack_descriptor()?;

    #[cfg(feature = "trace")]
    let _trace_guard = init_tracing(&buildpack_descriptor.buildpack, "build");
    #[cfg(feature = "trace")]
    let _span_guard = tracing::span!(tracing::Level::INFO, "libcnb-build").entered();

    #[cfg(feature = "trace")]
    let trace_error =
        |error: &Error<<B as Buildpack>::Error>| tracing::error!(?error, "libcnb-build-error");
    #[cfg(not(feature = "trace"))]
    let trace_error = |_: &Error<<B as Buildpack>::Error>| {};

    let platform = Platform::from_path(&args.platform_dir_path)
        .map_err(Error::CannotCreatePlatformFromPath)
        .inspect_err(trace_error)?;

    let buildpack_plan = read_toml_file(&args.buildpack_plan_path)
        .map_err(Error::CannotReadBuildpackPlan)
        .inspect_err(trace_error)?;

    let store = match read_toml_file::<Store>(layers_dir.join("store.toml")) {
        Err(TomlFileError::IoError(io_error)) if is_not_found_error_kind(&io_error) => Ok(None),
        other => other.map(Some),
    }
    .map_err(Error::CannotReadStore)
    .inspect_err(trace_error)?;

    let target = context_target().inspect_err(trace_error)?;

    let build_context = BuildContext {
        layers_dir: layers_dir.clone(),
        app_dir,
        platform,
        target,
        buildpack_plan,
        buildpack_dir,
        buildpack_descriptor,
        store,
    };

    let build_result = buildpack.build(build_context).inspect_err(trace_error)?;

    match build_result.0 {
        InnerBuildResult::Pass {
            launch,
            store,
            build_sboms,
            launch_sboms,
        } => {
            if let Some(launch) = launch {
                write_toml_file(&launch, layers_dir.join("launch.toml"))
                    .map_err(Error::CannotWriteLaunch)
                    .inspect_err(trace_error)?;
            }

            if let Some(store) = store {
                write_toml_file(&store, layers_dir.join("store.toml"))
                    .map_err(Error::CannotWriteStore)
                    .inspect_err(trace_error)?;
            }

            for build_sbom in build_sboms {
                fs::write(
                    cnb_sbom_path(&build_sbom.format, &layers_dir, "build"),
                    &build_sbom.data,
                )
                .map_err(Error::CannotWriteBuildSbom)
                .inspect_err(trace_error)?;
            }

            for launch_sbom in launch_sboms {
                fs::write(
                    cnb_sbom_path(&launch_sbom.format, &layers_dir, "launch"),
                    &launch_sbom.data,
                )
                .map_err(Error::CannotWriteLaunchSbom)
                .inspect_err(trace_error)?;
            }

            #[cfg(feature = "trace")]
            tracing::event!(tracing::Level::INFO, "libcnb-build-success");
            Ok(exit_code::GENERIC_SUCCESS)
        }
    }
}

// A partial representation of buildpack.toml that contains only the Buildpack API version,
// so that the version can still be read when the buildpack descriptor doesn't match the
// supported spec version.
#[derive(Deserialize)]
struct BuildpackDescriptorApiOnly {
    api: BuildpackApi,
}

#[doc(hidden)]
pub struct DetectArgs {
    pub platform_dir_path: PathBuf,
    pub build_plan_path: PathBuf,
}

impl DetectArgs {
    pub fn parse(args: &[String]) -> Result<Self, DetectArgsParseError> {
        if let [_, platform_dir_path, build_plan_path] = args {
            Ok(Self {
                platform_dir_path: PathBuf::from(platform_dir_path),
                build_plan_path: PathBuf::from(build_plan_path),
            })
        } else {
            Err(DetectArgsParseError::InvalidArguments)
        }
    }
}

#[derive(Debug)]
#[doc(hidden)]
pub enum DetectArgsParseError {
    InvalidArguments,
}

#[doc(hidden)]
pub struct BuildArgs {
    pub layers_dir_path: PathBuf,
    pub platform_dir_path: PathBuf,
    pub buildpack_plan_path: PathBuf,
}

impl BuildArgs {
    pub fn parse(args: &[String]) -> Result<Self, BuildArgsParseError> {
        if let [_, layers_dir_path, platform_dir_path, buildpack_plan_path] = args {
            Ok(Self {
                layers_dir_path: PathBuf::from(layers_dir_path),
                platform_dir_path: PathBuf::from(platform_dir_path),
                buildpack_plan_path: PathBuf::from(buildpack_plan_path),
            })
        } else {
            Err(BuildArgsParseError::InvalidArguments)
        }
    }
}

#[derive(Debug)]
#[doc(hidden)]
pub enum BuildArgsParseError {
    InvalidArguments,
}

fn read_buildpack_dir<E: Debug>() -> crate::Result<PathBuf, E> {
    env::var("CNB_BUILDPACK_DIR")
        .map_err(Error::CannotDetermineBuildpackDirectory)
        .map(PathBuf::from)
}

fn read_buildpack_descriptor<BD: DeserializeOwned, E: Debug>() -> crate::Result<BD, E> {
    read_buildpack_dir().and_then(|buildpack_dir| {
        read_toml_file(buildpack_dir.join("buildpack.toml"))
            .map_err(Error::CannotReadBuildpackDescriptor)
    })
}

fn context_target<E>() -> crate::Result<Target, E>
where
    E: Debug,
{
    let os = env::var("CNB_TARGET_OS").map_err(Error::CannotDetermineTargetOs)?;
    let arch = env::var("CNB_TARGET_ARCH").map_err(Error::CannotDetermineTargetArch)?;
    let arch_variant = env::var("CNB_TARGET_ARCH_VARIANT").ok();
    // Whilst the Buildpack API spec says these env vars are optional they will always be set in
    // practice, so we treat them as mandatory to improve buildpack author UX. See:
    // https://github.com/heroku/libcnb.rs/issues/820
    let distro_name =
        env::var("CNB_TARGET_DISTRO_NAME").map_err(Error::CannotDetermineTargetDistroName)?;
    let distro_version =
        env::var("CNB_TARGET_DISTRO_VERSION").map_err(Error::CannotDetermineTargetDistroVersion)?;

    Ok(Target {
        os,
        arch,
        arch_variant,
        distro_name,
        distro_version,
    })
}
