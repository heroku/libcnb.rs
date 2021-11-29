use cargo_metadata::MetadataCommand;
use clap::{App, AppSettings, Arg, ArgMatches};
use libcnb_cargo::cross_compile::cross_compile_help;
use libcnb_cargo::{
    assemble_buildpack_tarball, build_buildpack_binary, default_buildpack_tarball_filename,
    read_buildpack_data, BuildError, BuildpackDataError, CargoProfile,
};
use log::error;
use log::info;
use size_format::SizeFormatterSI;
use std::path::PathBuf;

fn main() {
    setup_logging();

    match setup_cli_parsing().get_matches().subcommand() {
        ("libcnb", Some(matches)) => match matches.subcommand() {
            ("package", Some(matches)) => handle_libcnb_package(matches),
            // This should never be reached since clap will catch unknown subcommands for us
            _ => unimplemented!("Only the \"package\" subcommand is currently implemented!"),
        },
        // This should never be reached since clap will catch unknown subcommands for us
        _ => unimplemented!("Only the \"libcnb\" subcommand is currently implemented!"),
    }
}

fn handle_libcnb_package(matches: &ArgMatches) {
    let cargo_profile = if matches.is_present("release") {
        CargoProfile::Release
    } else {
        CargoProfile::Dev
    };

    let target_triple = match matches.value_of("target") {
        None => {
            error!("Could not determine target triple!");
            std::process::exit(1);
        }
        Some(target_triple) => target_triple,
    };

    let current_dir = match std::env::current_dir() {
        Ok(current_dir) => current_dir,
        Err(io_error) => {
            error!("Could not determine current directory: {}", io_error);
            std::process::exit(1);
        }
    };

    info!("Reading buildpack metadata...");
    let buildpack_data = match read_buildpack_data(&current_dir) {
        Ok(buildpack_data) => buildpack_data,
        Err(error) => {
            error!("Packaging buildpack failed due to an error while reading buildpack metadata!");

            match error {
                BuildpackDataError::IoError(io_error) => {
                    error!("IO error while reading buildpack metadata: {}", io_error);
                    error!("Hint: Verify that a readable file named \"buildpack.toml\" exists at the root of your project.")
                }
                BuildpackDataError::DeserializationError(deserialization_error) => {
                    error!(
                        "Could not deserialize buildpack metadata: {}",
                        deserialization_error
                    );
                    error!("Hint: Verify that your \"buildpack.toml\" is valid.")
                }
            }

            std::process::exit(1);
        }
    };

    info!(
        "Found valid buildpack with id \"{}\" @ {}!",
        buildpack_data.buildpack_toml.buildpack.id, buildpack_data.buildpack_toml.buildpack.version
    );

    let output_path = matches
        .value_of("output-path")
        .map(PathBuf::from)
        .or_else(|| {
            MetadataCommand::new()
                .manifest_path(&current_dir.join("Cargo.toml"))
                .exec()
                .map(|metadata| {
                    metadata
                        .target_directory
                        .join(default_buildpack_tarball_filename(
                            &buildpack_data.buildpack_toml,
                            cargo_profile,
                        ))
                        .into_std_path_buf()
                })
                .ok()
        });

    let output_path = match output_path {
        Some(output_path) => output_path,
        None => {
            error!("Could not determine output path for tarball!");
            std::process::exit(1);
        }
    };

    let relative_output_path =
        pathdiff::diff_paths(&output_path, &current_dir).unwrap_or_else(|| output_path.clone());

    info!("Building buildpack binary ({})...", &target_triple);
    let binary_path = match build_buildpack_binary(&current_dir, cargo_profile, &target_triple) {
        Ok(binary_path) => binary_path,
        Err(error) => {
            error!("Packaging buildpack failed due to a build related error!");

            match error {
                BuildError::IoError(io_error) => {
                    error!("IO error while executing Cargo: {}", io_error);
                }
                BuildError::UnexpectedExitStatus(exit_status) => {
                    error!(
                        "Unexpected Cargo exit status: {}",
                        exit_status
                            .code()
                            .map(|code| code.to_string())
                            .unwrap_or_else(|| String::from("<unknown>"))
                    );
                    error!("Examine Cargo output for details and potential compilation errors.")
                }
                BuildError::CrossCompileError(_) => {
                    error!(
                        "Could not find required linker and C compiler for the target platform!"
                    );
                    if let Some(help_text) = cross_compile_help(&target_triple) {
                        error!("Hint:\n{}", help_text);
                    }
                }
                BuildError::NoTargetsFound => {
                    error!("No targets were found in the Cargo manifest. Ensure that there is exactly one binary target and try again.")
                }
                BuildError::MultipleTargetsFound => {
                    error!("Multiple targets were found in the Cargo manifest. Ensure that there is exactly one binary target and try again.")
                }
                BuildError::MetadataError(metadata_error) => {
                    error!("Unable to obtain metadata from Cargo: {}", metadata_error)
                }
                BuildError::CouldNotFindRootPackage => {
                    error!("Root package could not be determined from the Cargo manifest.")
                }
            }

            std::process::exit(1);
        }
    };

    info!("Writing buildpack tarball...");
    if let Err(io_error) = assemble_buildpack_tarball(
        &output_path,
        &buildpack_data.buildpack_toml_path,
        &binary_path,
    ) {
        error!("IO error while writing buildpack tarball: {}", io_error);
        std::process::exit(1);
    };

    info!(
        "Successfully wrote buildpack tarball: {} ({})",
        relative_output_path.to_string_lossy(),
        output_path
            .metadata()
            .map(|metadata| SizeFormatterSI::new(metadata.len()).to_string())
            .unwrap_or_else(|_| String::from("unknown size"))
    );

    info!("Packaging successfully finished!");
    info!("Hint: To test your buildpack locally with pack, run: pack build my-image -b {} --path /path/to/application", relative_output_path.to_string_lossy());
}

fn setup_cli_parsing<'a, 'b>() -> clap::App<'a, 'b> {
    App::new(env!("CARGO_PKG_NAME"))
        .bin_name("cargo")
        .setting(AppSettings::SubcommandRequiredElseHelp)
        .subcommand(
            App::new("libcnb")
                .setting(AppSettings::SubcommandRequiredElseHelp)
                .subcommand(
                    App::new("package")
                        .arg(
                            Arg::with_name("release")
                                .long("release")
                                .help("Build in release mode, with optimizations"),
                        )
                        .arg(
                            Arg::with_name("target")
                                .long("target")
                                .default_value("x86_64-unknown-linux-musl")
                                .help("Build for the target triple"),
                        )
                        .arg(
                            Arg::with_name("output-path")
                                .long("output")
                                .short("o")
                                .help("Write buildpack tarball to this path instead of Cargo's target directory")
                                .takes_value(true),
                        ),
                ),
        )
}

fn setup_logging() {
    if let Err(error) = stderrlog::new().quiet(false).verbosity(2).init() {
        eprintln!("Unable to initialize logger: {}", error);
        std::process::exit(1);
    }
}
