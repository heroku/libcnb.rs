#![doc = include_str!("../README.md")]
#![warn(clippy::pedantic)]
#![warn(unused_crate_dependencies)]
// This lint is too noisy and enforces a style that reduces readability in many cases.
#![allow(clippy::module_name_repetitions)]

mod cli;
mod exit_code;

use crate::cli::{Cli, LibcnbSubcommand, PackageArgs};
use cargo_metadata::MetadataCommand;
use chrono::Datelike;
use clap::{Parser, ValueEnum};
use cli::InitArgs;
use heck::ToUpperCamelCase;
use libcnb_package::build::{build_buildpack_binaries, BuildBinariesError, BuildError};
use libcnb_package::cross_compile::{cross_compile_assistance, CrossCompileAssistance};
use libcnb_package::{
    assemble_buildpack_directory, default_buildpack_directory_name, read_buildpack_data,
    BuildpackDataError, CargoProfile,
};
use log::{error, info, warn};
use std::path::{Path, PathBuf};
use std::process::{Command, Output};
use std::{fs, io};
use tera::Context;
use tera::Tera;

fn main() {
    setup_logging();

    match Cli::parse() {
        Cli::Libcnb(LibcnbSubcommand::Package(args)) => handle_libcnb_package(args),
        Cli::Libcnb(LibcnbSubcommand::Init(args)) => handle_libcnb_init(&args),
    }
}

#[derive(Debug, Clone, Eq, PartialEq, PartialOrd, Hash)]
struct BuildpackTemplate {
    target_path: PathBuf,
    contents: String,
}

fn templates_for_init(args: InitArgs) -> Vec<BuildpackTemplate> {
    let mut context = Context::new();
    context.insert("namespace", &args.namespace());
    context.insert("name", &args.name());
    context.insert("copyright", &args.copyright);
    context.insert("year", &chrono::Utc::now().year().to_string());
    context.insert("detect_file", &args.detect_file);
    context.insert(
        "buildpack_struct_name",
        &format!("{}Buildpack", &args.name().to_upper_camel_case()),
    );

    let mut tera = match Tera::new("templates/buildpack_init/**/*.jinja") {
        Ok(t) => t,
        Err(e) => {
            error!("Parsing template error(s): {}", e);
            std::process::exit(exit_code::UNSPECIFIED_ERROR);
        }
    };

    // Code of Conduct chooser
    tera.add_template_file(
        PathBuf::new()
            .join("templates")
            .join("code_of_conduct")
            .join(format!(
                "{}.md.jinja",
                args.conduct.to_possible_value().unwrap().get_name()
            )),
        Some("CODE_OF_CONDUCT.md"),
    )
    .expect("Internal error adding code of conduct template file to the templating engine");

    // License chooser
    tera.add_template_file(
        PathBuf::new()
            .join("templates")
            .join("license")
            .join(format!(
                "{}.md.jinja",
                args.license.to_possible_value().unwrap().get_name()
            )),
        Some("LICENSE.txt"),
    )
    .expect("Internal error adding license template file to the templating engine");

    let mut templates = tera
        .get_template_names()
        .into_iter()
        .map(|name| BuildpackTemplate {
            target_path: args
                .destination
                .join(name.strip_suffix(".jinja").unwrap_or(name)),
            contents: tera
                .render(name, &context)
                .expect("Could not compile template"),
        })
        .collect::<Vec<_>>();

    templates.push(BuildpackTemplate {
        target_path: args
            .destination
            .join("tests")
            .join("fixtures")
            .join("hello_world")
            .join(args.detect_file),
        contents: String::new(),
    });

    templates
}

use std::io::Write; // <--- bring flush() into scope

fn ask(question: &str) -> Result<String, io::Error> {
    println!("{}", question);
    print!("> ");
    io::stdout().flush()?;

    let mut user_input = String::new();

    io::stdin().read_line(&mut user_input)?;

    Ok(user_input.trim().to_string())
}

#[derive(Clone, Eq, PartialEq, PartialOrd, Hash, Debug)]
enum PromptState {
    Identical,
    Write,
    Skip,
    AllWrite,
    Quit,
}

fn file_collision_prompt(file: &Path, contents: &str, force: bool) -> PromptState {
    if !file.exists() || force {
        PromptState::Write
    } else {
        let original =
            std::fs::read_to_string(file).expect("Internal error could not read file from disk");
        if contents.trim() == original.trim() {
            PromptState::Identical
        } else {
            match ask(&format!(
                "Overwrite {}? (enter 'h' for help) [Ynaqdh]",
                file.display()
            )) {
                Err(io_error) => {
                    error!("Could not read input: {io_error}");
                    std::process::exit(exit_code::UNSPECIFIED_ERROR);
                }
                Ok(out) => match out.to_lowercase().as_str() {
                    "y" | "yes" | "" => PromptState::Write,
                    "n" | "no" => PromptState::Skip,
                    "a" | "all" => PromptState::AllWrite,
                    "q" | "quit" => PromptState::Quit,
                    "d" | "diff" => {
                        let tempdir = tempfile::tempdir()
                            .expect("Internal error, could not create temp file");
                        let dir = tempdir.path();
                        let tempfile = dir.join("new_contents.txt");
                        std::fs::write(&tempfile, contents)
                            .expect("Internal error, could not write to temp file");

                        let out = Command::new("diff")
                            .arg("-u")
                            .args([file, &tempfile])
                            .output();

                        match out {
                            Ok(out) => {
                                println!("{}", std::str::from_utf8(&out.stdout).expect("Internal error, could not convert output to UTF8. Check file contents are valid UTF-8"));

                                println!("Retrying...");
                                file_collision_prompt(file, contents, force)
                            }
                            Err(io_error) => {
                                error!(
                                    "Could not diff contents with file: {} error:\n{io_error}",
                                    file.display()
                                );
                                std::process::exit(exit_code::UNSPECIFIED_ERROR);
                            }
                        }
                    }
                    _ => {
                        println!(
                            r#"
    Y - yes, overwrite
    n - no, do not overwrite
    a - all, overwrite this and all others
    q - quit, abort
    d - diff, show the differences between the old and the new
    h - help, show this help
                    "#
                        );
                        file_collision_prompt(file, contents, force)
                    }
                },
            }
        }
    }
}

fn handle_libcnb_init(args: &InitArgs) {
    let mut force_all = args.force;
    for template in &templates_for_init(args.clone()) {
        let BuildpackTemplate {
            target_path,
            contents,
        } = template;

        if let Some(parent) = target_path.parent().filter(|p| !p.exists()) {
            std::fs::create_dir_all(parent).expect("Internal error, could not create directory");
        };

        let state = file_collision_prompt(target_path, contents, force_all);
        match state {
            PromptState::Identical => {
                info!("Identical {}", template.target_path.display());
            }
            PromptState::Skip => {}
            PromptState::Write | PromptState::AllWrite => {
                info!("Writing {}", template.target_path.display());
                std::fs::write(target_path, contents)
                    .expect("Internal error, could not write contents to path");

                if state == PromptState::AllWrite {
                    force_all = true;
                }
            }
            PromptState::Quit => {
                info!("Aborting...");
                std::process::exit(0);
            }
        }
    }

    let cmd = "cargo fmt --all";
    info!("Running {}", cmd);
    run_cmd_in_dir_checked(&args.destination, cmd);

    let cmd = "git init --initial-branch=main --quiet .";
    info!("Running {}", cmd);
    run_cmd_in_dir_checked(&args.destination, cmd);
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
            std::process::exit(exit_code::UNSPECIFIED_ERROR);
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

            std::process::exit(exit_code::UNSPECIFIED_ERROR);
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
            std::process::exit(exit_code::UNSPECIFIED_ERROR);
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
                std::process::exit(exit_code::UNSPECIFIED_ERROR);
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
        &cargo_build_env,
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

            std::process::exit(exit_code::UNSPECIFIED_ERROR);
        }
    };

    info!("Writing buildpack directory...");
    if output_path.exists() {
        if let Err(error) = fs::remove_dir_all(&output_path) {
            error!("Could not remove buildpack directory: {error}");
            std::process::exit(exit_code::UNSPECIFIED_ERROR);
        };
    }

    if let Err(io_error) = assemble_buildpack_directory(
        &output_path,
        &buildpack_data.buildpack_descriptor_path,
        &buildpack_binaries,
    ) {
        error!("IO error while writing buildpack directory: {io_error}");
        std::process::exit(exit_code::UNSPECIFIED_ERROR);
    };

    let size_in_bytes = calculate_dir_size(&output_path).unwrap_or_else(|io_error| {
        error!("IO error while calculating buildpack directory size: {io_error}");
        std::process::exit(exit_code::UNSPECIFIED_ERROR);
    });

    // Precision will only be lost for sizes bigger than 52 bits (~4 Petabytes), and even
    // then will only result in a less precise figure, so is not an issue.
    #[allow(clippy::cast_precision_loss)]
    let size_in_mb = size_in_bytes as f64 / (1024.0 * 1024.0);

    info!(
        "Successfully wrote buildpack directory: {} ({size_in_mb:.2} MiB)",
        relative_output_path.to_string_lossy(),
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
        std::process::exit(exit_code::UNSPECIFIED_ERROR);
    }
}

/// Recursively calculate the size of a directory and its contents in bytes.
// Not using `fs_extra::dir::get_size` since it doesn't handle symlinks correctly:
// https://github.com/webdesus/fs_extra/issues/59
fn calculate_dir_size(path: impl AsRef<Path>) -> io::Result<u64> {
    let mut size_in_bytes = 0;

    // The size of the directory entry (ie: its metadata only, not the directory contents).
    size_in_bytes += path.as_ref().metadata()?.len();

    for entry in fs::read_dir(&path)? {
        let entry = entry?;
        let metadata = entry.metadata()?;

        if metadata.is_dir() {
            size_in_bytes += calculate_dir_size(entry.path())?;
        } else {
            size_in_bytes += metadata.len();
        }
    }

    Ok(size_in_bytes)
}

fn run_cmd_in_dir(dir: &Path, command: &str) -> Output {
    let command = format!("cd {} && {}", dir.display(), command);
    let out = Command::new("bash")
        .args(["-c", &format!("cd {} && {}", dir.display(), command)])
        .output();

    match out {
        Ok(out) => out,
        Err(io_error) => {
            error!(
                "Error running command: {} in dir: {} error:\n{}",
                command,
                dir.display(),
                io_error
            );
            std::process::exit(exit_code::UNSPECIFIED_ERROR);
        }
    }
}

fn run_cmd_in_dir_checked(dir: &Path, command: &str) -> Output {
    let out = run_cmd_in_dir(dir, command);

    if !out.status.success() {
        let stdout = std::str::from_utf8(&out.stdout)
            .expect("Internal error, could not convert stdout to UTF8");
        let stderr = std::str::from_utf8(&out.stderr)
            .expect("Internal error, could not convert stderr to UTF8");

        error!(
            "Command: {} in dir: {} failed:\n{}\n{}",
            command,
            dir.display(),
            stdout,
            stderr
        );
        std::process::exit(exit_code::UNSPECIFIED_ERROR);
    }
    out
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::cli::{CodeOfConduct, License};

    fn default_init(destination: PathBuf) -> InitArgs {
        let detect_file = String::from("README.md");
        let conduct = CodeOfConduct::Salesforce;
        let name_namespace: crate::cli::NameWithNamespace = "heroku/ruby".parse().unwrap();
        let copyright = String::from("David S. Pumpkins");
        let license = License::Bsd3;
        let force = true;

        InitArgs {
            destination,
            name_namespace,
            detect_file,
            license,
            copyright,
            conduct,
            force,
        }
    }

    #[test]
    fn it_exercises_templates_for_init() {
        let tempdir = tempfile::tempdir().unwrap();
        let dir = tempdir.into_path();
        let templates = templates_for_init(default_init(dir.clone()));

        let expected = dir.join("src").join("main.rs");
        templates
            .iter()
            .find(|template| template.target_path == expected)
            .unwrap();

        let expected = dir.join("cargo.toml");
        templates
            .iter()
            .find(|template| template.target_path == expected)
            .unwrap();

        let expected = dir.join("buildpack.toml");
        templates
            .iter()
            .find(|template| template.target_path == expected)
            .unwrap();
    }

    #[test]
    #[ignore = "integration test"]
    fn test_handle_libcnb_init() {
        let tempdir = tempfile::tempdir().unwrap();
        let dir = tempdir.into_path();
        handle_libcnb_init(&default_init(dir.clone()));

        assert!(dir.join("CODE_OF_CONDUCT.md").exists());
        assert!(dir.join("cargo.toml").exists());

        let out = run_cmd_in_dir(
            &dir,
            "RUST_BACKTRACE=1 cargo test --all-features -- --include-ignored",
        );
        let stdout = std::str::from_utf8(&out.stdout).unwrap();
        let stderr = std::str::from_utf8(&out.stderr).unwrap();
        println!("{}\n{}", stdout, stderr);

        assert!(out.status.success());

        let out = run_cmd_in_dir(
            &dir,
            "cargo clippy --all-targets --all-features --locked -- --deny warnings",
        );
        let stdout = std::str::from_utf8(&out.stdout).unwrap();
        let stderr = std::str::from_utf8(&out.stderr).unwrap();
        println!("{}\n{}", stdout, stderr);

        assert!(out.status.success());
    }
}
