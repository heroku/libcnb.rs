// https://rust-lang.github.io/rust-clippy/stable/index.html
#![warn(clippy::pedantic)]
#![warn(unused_crate_dependencies)]
// This lint is too noisy and enforces a style that reduces readability in many cases.
#![allow(clippy::module_name_repetitions)]

mod cli;

use crate::cli::{Cli, LibcnbSubcommand, PackageArgs};
use cargo_metadata::MetadataCommand;
use clap::Parser;
use libcnb_package::build::{build_buildpack_binaries, BuildBinariesError, BuildError};
use libcnb_package::cross_compile::{cross_compile_assistance, CrossCompileAssistance};
use libcnb_package::{
    assemble_buildpack_directory, default_buildpack_directory_name, read_buildpack_data,
    BuildpackDataError, CargoProfile,
};
use log::{error, info, warn};
use size_format::SizeFormatterSI;
use std::fs;

fn main() {
    setup_logging();

    match Cli::parse() {
        Cli::Libcnb(LibcnbSubcommand::Package(args)) => handle_libcnb_package(args),
    }
}

#[allow(clippy::too_many_lines)]
fn handle_libcnb_package(args: PackageArgs) {
    let cargo_profile = if args.release {
        CargoProfile::Release
    } else {
        CargoProfile::Dev
    };

    let target_triple = args.target;

    let current_dir = match std::env::current_dir() {
        Ok(current_dir) => current_dir,
        Err(io_error) => {
            error!("Could not determine current directory: {io_error}");
            std::process::exit(1);
        }
    };

    info!("Reading buildpack metadata...");
    let buildpack_data = match read_buildpack_data(&current_dir) {
        Ok(buildpack_data) => buildpack_data,
        Err(error) => {
            match error {
                BuildpackDataError::IoError(io_error) => {
                    error!("Unable to read buildpack metadata: {io_error}");
                    error!("Hint: Verify that a readable file named \"buildpack.toml\" exists at the root of your project.");
                }
                BuildpackDataError::DeserializationError(deserialization_error) => {
                    error!("Unable to deserialize buildpack metadata: {deserialization_error}");
                    error!("Hint: Verify that your \"buildpack.toml\" is valid.");
                }
            }

            std::process::exit(1);
        }
    };

    info!(
        "Found buildpack {} with version {}.",
        buildpack_data.buildpack_descriptor.buildpack.id,
        buildpack_data.buildpack_descriptor.buildpack.version
    );

    let cargo_metadata = match MetadataCommand::new()
        .manifest_path(&current_dir.join("Cargo.toml"))
        .exec()
    {
        Ok(cargo_metadata) => cargo_metadata,
        Err(error) => {
            error!("Could not obtain metadata from Cargo: {error}");
            std::process::exit(1);
        }
    };

    let output_path = cargo_metadata
        .target_directory
        .join("buildpack")
        .join(match cargo_profile {
            CargoProfile::Dev => "debug",
            CargoProfile::Release => "release",
        })
        .join(default_buildpack_directory_name(
            &buildpack_data.buildpack_descriptor,
        ))
        .into_std_path_buf();

    let relative_output_path =
        pathdiff::diff_paths(&output_path, &current_dir).unwrap_or_else(|| output_path.clone());

    let cargo_build_env = if args.no_cross_compile_assistance {
        vec![]
    } else {
        info!("Determining automatic cross-compile settings...");
        match cross_compile_assistance(&target_triple) {
            CrossCompileAssistance::HelpText(help_text) => {
                error!("{help_text}");
                info!("To disable cross-compile assistance, pass --no-cross-compile-assistance.");
                std::process::exit(1);
            }
            CrossCompileAssistance::NoAssistance => {
                warn!("Could not determine automatic cross-compile settings for target triple {target_triple}.");
                warn!("This is not an error, but without proper cross-compile settings in your Cargo manifest and locally installed toolchains, compilation might fail.");
                warn!("To disable this warning, pass --no-cross-compile-assistance.");
                vec![]
            }
            CrossCompileAssistance::Configuration { cargo_env } => cargo_env,
        }
    };

    info!("Building binaries ({target_triple})...");

    let buildpack_binaries = match build_buildpack_binaries(
        &current_dir,
        &cargo_metadata,
        cargo_profile,
        cargo_build_env,
        &target_triple,
    ) {
        Ok(binaries) => binaries,
        Err(build_error) => {
            error!("Packaging buildpack failed due to a build related error!");

            match build_error {
                BuildBinariesError::ConfigError(_) => {}
                BuildBinariesError::BuildError(target_name, BuildError::IoError(io_error)) => {
                    error!("IO error while executing Cargo for target {target_name}: {io_error}");
                }
                BuildBinariesError::BuildError(
                    target_name,
                    BuildError::UnexpectedCargoExitStatus(exit_status),
                ) => {
                    error!(
                        "Unexpected Cargo exit status for target {target_name}: {}",
                        exit_status
                            .code()
                            .map_or_else(|| String::from("<unknown>"), |code| code.to_string())
                    );
                    error!("Examine Cargo output for details and potential compilation errors.");
                }
                BuildBinariesError::MissingBuildpackTarget(target_name) => {
                    error!("Configured buildpack target name {target_name} could not be found!");
                }
            }

            std::process::exit(1);
        }
    };

    info!("Writing buildpack directory...");
    if output_path.exists() {
        if let Err(error) = fs::remove_dir_all(&output_path) {
            error!("Could not remove buildpack directory: {error}");
            std::process::exit(1);
        };
    }

    if let Err(io_error) = assemble_buildpack_directory(
        &output_path,
        &buildpack_data.buildpack_descriptor_path,
        &buildpack_binaries,
    ) {
        error!("IO error while writing buildpack directory: {io_error}");
        std::process::exit(1);
    };

    info!(
        "Successfully wrote buildpack directory: {} ({})",
        relative_output_path.to_string_lossy(),
        fs_extra::dir::get_size(&output_path).map_or_else(
            |_| String::from("unknown size"),
            |size| SizeFormatterSI::new(size).to_string()
        )
    );

    info!("Packaging successfully finished!");
    info!("Hint: To test your buildpack locally with pack, run: pack build my-image --buildpack {} --path /path/to/application", relative_output_path.to_string_lossy());
}

fn setup_logging() {
    if let Err(error) = stderrlog::new()
        .verbosity(2) // LevelFilter::Info
        .init()
    {
        eprintln!("Unable to initialize logger: {error}");
        std::process::exit(1);
    }
}
