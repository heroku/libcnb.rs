#![doc = include_str!("../README.md")]
#![warn(clippy::pedantic)]
#![warn(unused_crate_dependencies)]
// This lint is too noisy and enforces a style that reduces readability in many cases.
#![allow(clippy::module_name_repetitions)]

mod cli;
mod exit_code;

use crate::cli::{Cli, LibcnbSubcommand, PackageArgs};
use cargo_metadata::{Metadata, MetadataCommand};
use clap::Parser;
use glob::glob;
use libcnb_data::buildpackage::{Buildpackage, BuildpackageUri};
use libcnb_package::build::{build_buildpack_binaries, BuildBinariesError, BuildError};
use libcnb_package::cross_compile::{cross_compile_assistance, CrossCompileAssistance};
use libcnb_package::{
    assemble_buildpack_directory, default_buildpack_directory_name, read_buildpack_data,
    read_buildpackage_data, BuildpackData, BuildpackDataError, CargoProfile,
};
use log::{error, info, warn};
use path_absolutize::Absolutize;
use std::collections::HashSet;
use std::path::{Path, PathBuf};
use std::process::Command;
use std::{env, fs, io};
use toml::Table;
use uriparse::RelativeReference;

// Suppress warnings due to the `unused_crate_dependencies` lint not handling integration tests well.
#[cfg(test)]
use assert_cmd as _;
#[cfg(test)]
use fs_extra as _;
use libcnb_data::buildpack::BuildpackDescriptor;
#[cfg(test)]
use tempfile as _;

#[derive(Debug, Clone)]
struct BuildpackWorkspace {
    root: PathBuf,
    target_dir: PathBuf,
}

#[derive(Debug, Clone)]
struct BuildpackProject {
    source_dir: PathBuf,
    target_dir: PathBuf,
    buildpack_data: BuildpackData<Option<Table>>,
}

fn main() {
    setup_logging();
    match Cli::parse() {
        Cli::Libcnb(LibcnbSubcommand::Package(args)) => run_package_command(&args),
    }
}

fn run_package_command(args: &PackageArgs) {
    let buildpack_workspace = get_buildpack_workspace();
    let buildpack_projects = get_buildpack_projects(&buildpack_workspace);
    let buildpack_projects_to_compile = get_buildpack_projects_to_compile(&buildpack_projects);

    for buildpack_project in &buildpack_projects_to_compile {
        compile_buildpack_project_for_packaging(
            args,
            buildpack_project,
            &buildpack_projects_to_compile,
        );
    }
}

fn compile_buildpack_project_for_packaging(
    args: &PackageArgs,
    buildpack_project: &BuildpackProject,
    buildpack_projects_to_compile: &[BuildpackProject],
) {
    if buildpack_project.source_dir.join("Cargo.toml").exists() {
        compile_single_buildpack_project_for_packaging(args, buildpack_project);
    } else {
        compile_meta_buildpack_project_for_packaging(
            buildpack_project,
            buildpack_projects_to_compile,
        );
    }
    println!("{}", buildpack_project.target_dir.to_string_lossy());
}

#[allow(clippy::too_many_lines)]
fn compile_single_buildpack_project_for_packaging(
    args: &PackageArgs,
    buildpack_project: &BuildpackProject,
) {
    let cargo_profile = if args.release {
        CargoProfile::Release
    } else {
        CargoProfile::Dev
    };
    let target_triple = &args.target;
    let cargo_metadata = get_cargo_metadata(&buildpack_project.source_dir);
    let package_dir = buildpack_project.source_dir.clone();
    let output_path = buildpack_project.target_dir.clone();
    let relative_output_path =
        pathdiff::diff_paths(&output_path, &package_dir).unwrap_or_else(|| output_path.clone());

    let cargo_build_env = if args.no_cross_compile_assistance {
        vec![]
    } else {
        info!("Determining automatic cross-compile settings...");
        match cross_compile_assistance(target_triple) {
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
        &package_dir,
        &cargo_metadata,
        cargo_profile,
        &cargo_build_env,
        target_triple,
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
        &buildpack_project.buildpack_data.buildpack_descriptor_path,
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

fn compile_meta_buildpack_project_for_packaging(
    buildpack_project: &BuildpackProject,
    buildpack_projects_to_compile: &[BuildpackProject],
) {
    let relative_output_path =
        pathdiff::diff_paths(&buildpack_project.target_dir, &buildpack_project.source_dir)
            .unwrap_or_else(|| buildpack_project.target_dir.clone());

    info!("Writing buildpack directory...");
    if buildpack_project.target_dir.exists() {
        if let Err(error) = fs::remove_dir_all(&buildpack_project.target_dir) {
            error!("Could not remove buildpack directory: {error}");
            std::process::exit(exit_code::UNSPECIFIED_ERROR);
        };
    }

    if let Err(error) = fs::create_dir_all(&buildpack_project.target_dir) {
        error!("Could not create packaged buildpack directory: {error}");
        std::process::exit(exit_code::UNSPECIFIED_ERROR);
    }

    if let Err(error) = fs::copy(
        &buildpack_project.buildpack_data.buildpack_descriptor_path,
        buildpack_project.target_dir.join("buildpack.toml"),
    ) {
        error!("Could not copy buildpack.toml to target directory: {error}");
        std::process::exit(exit_code::UNSPECIFIED_ERROR);
    }

    let buildpackage_data =
        if let Ok(buildpackage_data) = read_buildpackage_data(&buildpack_project.source_dir) {
            buildpackage_data
        } else {
            error!(
                "Could not read package.toml in {}",
                buildpack_project.source_dir.to_string_lossy()
            );
            std::process::exit(exit_code::UNSPECIFIED_ERROR);
        };

    let dependencies = buildpackage_data
        .buildpackage_descriptor
        .dependencies
        .into_iter()
        .map(|dependency| {
            if RelativeReference::try_from(dependency.uri.as_str()).is_ok() {
                let absolute_path = if dependency.uri.starts_with('.') {
                    absolutize(&buildpack_project.source_dir.join(&dependency.uri))
                } else {
                    absolutize(&PathBuf::from(&dependency.uri))
                };

                let local_buildpack_project = buildpack_projects_to_compile
                    .iter()
                    .find(|local_buildpack_project| local_buildpack_project.target_dir == absolute_path);

                if let Some(local_buildpack_project) = local_buildpack_project {
                    BuildpackageUri {
                        uri: String::from(local_buildpack_project.target_dir.to_string_lossy())
                    }
                } else {
                    error!(
                        "The local buildpack dependency '{}' could not be found. Verify the path and correct it in {}",
                        dependency.uri,
                        buildpack_project.source_dir.join("package.toml").to_string_lossy()
                    );
                    std::process::exit(exit_code::UNSPECIFIED_ERROR);
                }
            } else {
                dependency
            }
        })
        .collect::<Vec<_>>();

    let buildpackage = Buildpackage {
        buildpack: buildpackage_data.buildpackage_descriptor.buildpack,
        platform: buildpackage_data.buildpackage_descriptor.platform,
        dependencies,
    };

    let buildpackage_content = match toml::to_string(&buildpackage) {
        Ok(buildpackage_content) => buildpackage_content,
        Err(error) => {
            error!("Could not serialize package.toml content: {error}");
            std::process::exit(exit_code::UNSPECIFIED_ERROR);
        }
    };

    if let Err(error) = fs::write(
        buildpack_project.target_dir.join("package.toml"),
        buildpackage_content,
    ) {
        error!("Could not write package.toml to target directory: {error}");
        std::process::exit(exit_code::UNSPECIFIED_ERROR);
    }

    let size_in_bytes =
        calculate_dir_size(&buildpack_project.target_dir).unwrap_or_else(|io_error| {
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

fn get_buildpack_workspace() -> BuildpackWorkspace {
    let workspace_root = get_workspace_root();
    let workspace_cargo_metadata = get_cargo_metadata(&workspace_root);
    let workspace_target_directory = workspace_cargo_metadata
        .target_directory
        .into_std_path_buf();
    BuildpackWorkspace {
        root: workspace_root,
        target_dir: workspace_target_directory,
    }
}

fn get_buildpack_projects(buildpack_workspace: &BuildpackWorkspace) -> Vec<BuildpackProject> {
    let buildpack_dirs = find_buildpack_directories(buildpack_workspace);
    let buildpack_metadatas: Vec<_> = buildpack_dirs.iter().map(get_buildpack_metadata).collect();
    let buildpack_target_dirs: Vec<_> = buildpack_metadatas
        .iter()
        .map(|buildpack_data| get_buildpack_target_directory(buildpack_workspace, buildpack_data))
        .collect();
    buildpack_dirs
        .iter()
        .zip(buildpack_metadatas.iter())
        .zip(buildpack_target_dirs.iter())
        .map(
            |((buildpack_dir, buildpack_data), buildpack_target_dir)| BuildpackProject {
                source_dir: buildpack_dir.clone(),
                target_dir: buildpack_target_dir.clone(),
                buildpack_data: buildpack_data.clone(),
            },
        )
        .collect()
}

fn get_buildpack_projects_to_compile(
    buildpack_projects: &[BuildpackProject],
) -> Vec<BuildpackProject> {
    let current_dir = get_current_dir();

    let buildpack_projects_in_scope: Vec<_> = if current_dir.join("buildpack.toml").exists() {
        buildpack_projects
            .iter()
            .filter(|buildpack_project| buildpack_project.source_dir.clone() == current_dir)
            .collect()
    } else {
        buildpack_projects.iter().collect()
    };

    let mut buildpack_projects_seen: HashSet<PathBuf> = HashSet::new();
    let mut buildpack_projects_to_compile: Vec<BuildpackProject> = vec![];

    for buildpack_project in buildpack_projects_in_scope {
        for dependent_buildpack_project in
            get_local_dependencies_for_buildpack(buildpack_project, buildpack_projects)
        {
            if !buildpack_projects_seen.contains(&dependent_buildpack_project.source_dir) {
                buildpack_projects_seen.insert(dependent_buildpack_project.source_dir.clone());
                buildpack_projects_to_compile.push(dependent_buildpack_project.clone());
            }
        }
        if !buildpack_projects_seen.contains(&buildpack_project.source_dir) {
            buildpack_projects_seen.insert(buildpack_project.source_dir.clone());
            buildpack_projects_to_compile.push(buildpack_project.clone());
        }
    }

    buildpack_projects_to_compile
}

fn get_buildpack_metadata(buildpack_dir: &PathBuf) -> BuildpackData<Option<Table>> {
    info!("Reading buildpack metadata...");

    let buildpack_data = match read_buildpack_data(buildpack_dir) {
        Ok(buildpack_data) => buildpack_data,
        Err(error) => {
            match error {
                BuildpackDataError::IoError(io_error) => {
                    error!("Unable to read buildpack metadata: {io_error}");
                    error!(
                        "Hint: Verify that a readable file named \"buildpack.toml\" exists in {}.",
                        buildpack_dir.to_string_lossy()
                    );
                }
                BuildpackDataError::DeserializationError(deserialization_error) => {
                    error!("Unable to deserialize buildpack metadata: {deserialization_error}");
                    error!(
                        "Hint: Verify that your \"buildpack.toml\" in {} is valid.",
                        buildpack_dir.to_string_lossy()
                    );
                }
            }
            std::process::exit(exit_code::UNSPECIFIED_ERROR);
        }
    };

    let (id, version) = match &buildpack_data.buildpack_descriptor {
        BuildpackDescriptor::Single(descriptor) => {
            (&descriptor.buildpack.id, &descriptor.buildpack.version)
        }
        BuildpackDescriptor::Meta(descriptor) => {
            (&descriptor.buildpack.id, &descriptor.buildpack.version)
        }
    };

    info!("Found buildpack {} with version {}.", id, version);

    buildpack_data
}

fn get_cargo_metadata(buildpack_dir: &Path) -> Metadata {
    match MetadataCommand::new()
        .manifest_path(&buildpack_dir.join("Cargo.toml"))
        .exec()
    {
        Ok(cargo_metadata) => cargo_metadata,
        Err(error) => {
            error!("Could not obtain metadata from Cargo: {error}");
            std::process::exit(exit_code::UNSPECIFIED_ERROR);
        }
    }
}

fn get_current_dir() -> PathBuf {
    match env::current_dir() {
        Ok(current_dir) => current_dir,
        Err(io_error) => {
            error!("Could not determine current directory: {io_error}");
            std::process::exit(exit_code::UNSPECIFIED_ERROR);
        }
    }
}

fn get_workspace_root() -> PathBuf {
    let cargo = env::var("CARGO")
        .map(PathBuf::from)
        .ok()
        .unwrap_or_else(|| PathBuf::from("cargo"));

    let mut locate_project = Command::new(cargo);
    locate_project.args(["locate-project", "--workspace", "--message-format", "plain"]);

    match locate_project.output() {
        Ok(output) => {
            let output = String::from_utf8_lossy(&output.stdout);
            parent_dir(&PathBuf::from(output.trim()))
        }
        Err(error) => {
            error!("Could not locate project root: {error}");
            std::process::exit(exit_code::UNSPECIFIED_ERROR);
        }
    }
}

fn find_buildpack_directories(buildpack_workspace: &BuildpackWorkspace) -> Vec<PathBuf> {
    let paths = match glob(
        &buildpack_workspace
            .root
            .join("**/buildpack.toml")
            .to_string_lossy(),
    ) {
        Ok(paths) => paths,
        Err(error) => {
            error!(
                "Failed to glob buildpack.toml files in {}: {error}",
                buildpack_workspace.root.to_string_lossy()
            );
            std::process::exit(exit_code::UNSPECIFIED_ERROR);
        }
    };

    let exclude_target_pattern = match glob::Pattern::new(
        &buildpack_workspace
            .target_dir
            .join("**/buildpack.toml")
            .to_string_lossy(),
    ) {
        Ok(pattern) => pattern,
        Err(error) => {
            error!(
                "Invalid glob pattern for {}: {error}",
                buildpack_workspace.target_dir.to_string_lossy()
            );
            std::process::exit(exit_code::UNSPECIFIED_ERROR);
        }
    };

    paths
        .filter_map(Result::ok)
        .filter(|path| !exclude_target_pattern.matches(&path.to_string_lossy()))
        .map(|path| parent_dir(&path))
        .collect()
}

fn get_buildpack_target_directory(
    buildpack_workspace: &BuildpackWorkspace,
    buildpack_data: &BuildpackData<Option<Table>>,
) -> PathBuf {
    buildpack_workspace
        .target_dir
        .join("buildpack")
        .join(default_buildpack_directory_name(
            &buildpack_data.buildpack_descriptor,
        ))
}

fn get_local_dependencies_for_buildpack(
    buildpack_project: &BuildpackProject,
    buildpack_projects: &[BuildpackProject],
) -> Vec<BuildpackProject> {
    let local_buildpack_uris = match read_buildpackage_data(&buildpack_project.source_dir) {
        Ok(buildpackage_data) => buildpackage_data
            .buildpackage_descriptor
            .dependencies
            .into_iter()
            .filter(|dependency| RelativeReference::try_from(dependency.uri.as_str()).is_ok())
            .collect::<Vec<_>>(),
        Err(_) => vec![],
    };

    local_buildpack_uris
        .iter()
        .map(|local_buildpack_uri| {
            let absolute_path = if local_buildpack_uri.uri.starts_with('.') {
                absolutize(&buildpack_project.source_dir.join(&local_buildpack_uri.uri))
            } else {
                absolutize(&PathBuf::from(&local_buildpack_uri.uri))
            };

            if let Some(local_buildpack_project) = buildpack_projects
                .iter()
                .find(|buildpack_project| buildpack_project.target_dir == absolute_path) { local_buildpack_project.clone()
            } else {
                error!(
                    "The local buildpack dependency '{}' could not be found. Verify the path and correct it in {}",
                    local_buildpack_uri.uri,
                    buildpack_project.source_dir.join("package.toml").to_string_lossy()
                );
                std::process::exit(exit_code::UNSPECIFIED_ERROR);
            }
        })
        .collect()
}

fn parent_dir(path: &Path) -> PathBuf {
    if let Some(parent) = path.parent() {
        parent.to_path_buf()
    } else {
        error!(
            "Could not get parent directory from {}",
            path.to_string_lossy()
        );
        std::process::exit(exit_code::UNSPECIFIED_ERROR);
    }
}

fn absolutize(path: &PathBuf) -> PathBuf {
    match path.absolutize() {
        Ok(abs_path) => abs_path.to_path_buf(),
        Err(error) => {
            error!(
                "Could not get absolute path for {}: {error}",
                path.to_string_lossy()
            );
            std::process::exit(exit_code::UNSPECIFIED_ERROR);
        }
    }
}
