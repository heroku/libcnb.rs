#![doc = include_str!("../README.md")]
#![warn(clippy::pedantic)]
#![warn(unused_crate_dependencies)]
// This lint is too noisy and enforces a style that reduces readability in many cases.
#![allow(clippy::module_name_repetitions)]

mod cli;
mod exit_code;

// Suppress warnings due to the `unused_crate_dependencies` lint not handling integration tests well.
#[cfg(test)]
use fs_extra as _;
#[cfg(test)]
use tempfile as _;

use crate::cli::{Cli, LibcnbSubcommand, PackageArgs};
use cargo_metadata::{Metadata, MetadataCommand};
use clap::Parser;
use console::Emoji;
use glob::glob;
use indoc::formatdoc;
use libcnb_data::buildpack::{BuildpackDescriptor, BuildpackId};
use libcnb_data::buildpackage::{Buildpackage, BuildpackageUri};
use libcnb_package::build::{build_buildpack_binaries, BuildBinariesError, BuildError};
use libcnb_package::cross_compile::{cross_compile_assistance, CrossCompileAssistance};
use libcnb_package::{
    assemble_buildpack_directory, default_buildpack_directory_name, read_buildpack_data,
    read_buildpackage_data, BuildpackData, BuildpackageData, CargoProfile,
};
use std::collections::HashSet;
use std::path::{Path, PathBuf};
use std::process::Command;
use std::{env, fs, io};
use toml::Table;
use uriparse::URI;

static LOOKING_GLASS: Emoji<'_, '_> = Emoji("🔍 ", "");
static PACKAGE: Emoji<'_, '_> = Emoji("📦 ", "");
static SPARKLE: Emoji<'_, '_> = Emoji("✨ ", ":-)");
static CROSS_MARK: Emoji<'_, '_> = Emoji("❌ ", "");
static WARNING: Emoji<'_, '_> = Emoji("⚠️ ", "");
static BULB: Emoji<'_, '_> = Emoji("💡️ ", "");

#[derive(Debug, Clone)]
struct BuildpackWorkspace {
    root: PathBuf,
    target_dir: PathBuf,
}

#[derive(Debug, Clone)]
struct BuildpackProject {
    id: BuildpackId,
    source_dir: PathBuf,
    target_dir: PathBuf,
    buildpack_data: BuildpackData<Option<Table>>,
    buildpackage_data: Option<BuildpackageData>,
    local_dependencies: Vec<BuildpackId>,
}

fn main() {
    match Cli::parse() {
        Cli::Libcnb(LibcnbSubcommand::Package(args)) => run_package_command(&args),
    }
}

fn run_package_command(args: &PackageArgs) {
    eprintln!("{LOOKING_GLASS} Locating buildpacks...",);
    let buildpack_workspace = get_buildpack_workspace();
    let buildpack_projects = get_buildpack_projects(&buildpack_workspace, args);
    let buildpack_projects_to_compile = get_buildpack_projects_to_compile(&buildpack_projects);

    let mut current_count = 1;
    let total_count = buildpack_projects_to_compile.len() as u64;
    for buildpack_project in &buildpack_projects_to_compile {
        eprintln!(
            "{PACKAGE} [{current_count}/{total_count}] Building {}",
            buildpack_project.id
        );
        compile_buildpack_project_for_packaging(
            args,
            buildpack_project,
            &buildpack_projects_to_compile,
        );
        current_count += 1;
    }
    eprintln!("{SPARKLE} Packaging successfully finished!");

    eprintln!(
        "{}",
        create_example_pack_command(&buildpack_projects_to_compile)
    );

    println!(
        "{}",
        get_buildpack_projects_in_scope(&buildpack_projects)
            .iter()
            .map(|buildpack_project| buildpack_project.target_dir.to_string_lossy())
            .collect::<Vec<_>>()
            .join("\n")
    );
}

fn compile_buildpack_project_for_packaging(
    args: &PackageArgs,
    buildpack_project: &BuildpackProject,
    buildpack_projects_to_compile: &[BuildpackProject],
) {
    match buildpack_project.buildpack_data.buildpack_descriptor {
        BuildpackDescriptor::Single(_) => {
            if is_legacy_buildpack_project(&buildpack_project.source_dir) {
                compile_legacy_buildpack_project_for_packaging(buildpack_project);
            } else {
                compile_single_buildpack_project_for_packaging(args, buildpack_project);
            }
        }
        BuildpackDescriptor::Meta(_) => compile_meta_buildpack_project_for_packaging(
            buildpack_project,
            buildpack_projects_to_compile,
        ),
    }
}

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

    let cargo_build_env = if args.no_cross_compile_assistance {
        vec![]
    } else {
        eprintln!("Determining automatic cross-compile settings...");
        match cross_compile_assistance(target_triple) {
            CrossCompileAssistance::HelpText(help_text) => {
                fail_with_error(formatdoc! { "
                    {help_text}

                    To disable cross-compile assistance, pass --no-cross-compile-assistance.
                " });
            }
            CrossCompileAssistance::NoAssistance => {
                warn(formatdoc! { "
                    Could not determine automatic cross-compile settings for target triple {target_triple}.
                    This is not an error, but without proper cross-compile settings in your Cargo manifest and locally installed toolchains, compilation might fail.
                    To disable this warning, pass --no-cross-compile-assistance.
                " });
                vec![]
            }
            CrossCompileAssistance::Configuration { cargo_env } => cargo_env,
        }
    };

    eprintln!("Building binaries ({target_triple})...");

    let buildpack_binaries = match build_buildpack_binaries(
        &package_dir,
        &cargo_metadata,
        cargo_profile,
        &cargo_build_env,
        target_triple,
    ) {
        Ok(binaries) => binaries,
        Err(build_error) => {
            let error_header = "Packaging buildpack failed due to a build related error!";

            match build_error {
                BuildBinariesError::ConfigError(_) => fail_with_error(error_header),
                BuildBinariesError::BuildError(target_name, BuildError::IoError(io_error)) => {
                    fail_with_error(formatdoc! { "
                        {error_header}

                        IO error while executing Cargo for target {target_name}: {io_error}
                    " })
                }
                BuildBinariesError::BuildError(
                    target_name,
                    BuildError::UnexpectedCargoExitStatus(exit_status),
                ) => fail_with_error(formatdoc! { "
                        {error_header}

                        Unexpected Cargo exit status for target {target_name}: {}

                        Examine Cargo output for details and potential compilation errors.
                    ", exit_status
                .code()
                .map_or_else(|| String::from("<unknown>"), |code| code.to_string()) }),

                BuildBinariesError::MissingBuildpackTarget(target_name) => {
                    fail_with_error(formatdoc! { "
                        {error_header}

                        Configured buildpack target name {target_name} could not be found!
                    " })
                }
            }
        }
    };

    eprintln!("Writing buildpack directory...");
    if output_path.exists() {
        if let Err(error) = fs::remove_dir_all(&output_path) {
            fail_with_error(format!("Could not remove buildpack directory: {error}"));
        };
    }

    if let Err(io_error) = assemble_buildpack_directory(
        &output_path,
        &buildpack_project.buildpack_data.buildpack_descriptor_path,
        &buildpack_binaries,
    ) {
        fail_with_error(format!(
            "IO error while writing buildpack directory: {io_error}"
        ));
    };

    let size_in_bytes = calculate_dir_size(&output_path).unwrap_or_else(|io_error| {
        fail_with_error(format!(
            "IO error while calculating buildpack directory size: {io_error}"
        ));
    });

    // Precision will only be lost for sizes bigger than 52 bits (~4 Petabytes), and even
    // then will only result in a less precise figure, so is not an issue.
    #[allow(clippy::cast_precision_loss)]
    let size_in_mb = size_in_bytes as f64 / (1024.0 * 1024.0);
    let relative_output_path =
        pathdiff::diff_paths(&output_path, &package_dir).unwrap_or_else(|| output_path.clone());

    eprintln!(
        "Successfully wrote buildpack directory: {} ({size_in_mb:.2} MiB)",
        relative_output_path.to_string_lossy(),
    );
}

fn compile_legacy_buildpack_project_for_packaging(buildpack_project: &BuildpackProject) {
    let is_legacy_buildpack = ["detect", "build"].iter().all(|bin_name| {
        let bin_path = buildpack_project.source_dir.join("bin").join(bin_name);
        bin_path.exists() && bin_path.is_file()
    });

    if is_legacy_buildpack {
        eprintln!("Non-rust buildpack directory detected, no compilation will be performed...");
    } else {
        fail_with_error(format!("The directory {} contains a buildpack but it is neither a Rust-based project nor a legacy buildpack", buildpack_project.source_dir.to_string_lossy()));
    }
}

fn compile_meta_buildpack_project_for_packaging(
    buildpack_project: &BuildpackProject,
    buildpack_projects_to_compile: &[BuildpackProject],
) {
    eprintln!("Writing buildpack directory...");
    if buildpack_project.target_dir.exists() {
        if let Err(error) = fs::remove_dir_all(&buildpack_project.target_dir) {
            fail_with_error(format!("Could not remove buildpack directory: {error}"));
        };
    }

    if let Err(error) = fs::create_dir_all(&buildpack_project.target_dir) {
        fail_with_error(format!(
            "Could not create packaged buildpack directory: {error}"
        ));
    }

    if let Err(error) = fs::copy(
        &buildpack_project.buildpack_data.buildpack_descriptor_path,
        buildpack_project.target_dir.join("buildpack.toml"),
    ) {
        fail_with_error(format!(
            "Could not copy buildpack.toml to target directory: {error}"
        ));
    }

    let buildpackage_data = match &buildpack_project.buildpackage_data {
        Some(buildpackage_data) => buildpackage_data,
        None => {
            fail_with_error(format!(
                "Attempting to compile meta-buildpack at `{}` but no package.toml data was found",
                &buildpack_project.source_dir.to_string_lossy()
            ));
        }
    };

    let dependencies: Vec<_> = buildpackage_data
        .buildpackage_descriptor
        .dependencies
        .iter()
        .map(|buildpackage_uri| {
            let uri = match URI::try_from(buildpackage_uri.uri.as_str()) {
                Ok(uri) => uri,
                Err(_) => fail_with_error(format!("Could not compile meta-buildpack at {} due to invalid URI `{}` in package.toml",
                buildpack_project.source_dir.to_string_lossy(), buildpackage_uri.uri.as_str())),
            };
            if is_local_buildpack_uri(&uri) {
                let local_dependency_target_dir =
                    match find_local_buildpack_project(buildpack_projects_to_compile, &uri) {
                        Some(project_dependency) => project_dependency.target_dir.to_string_lossy(),
                        None => fail_with_error(format!("Attempting to compile meta-buildpack at `{}` because it depends on a local buildpack `{}` but no valid buildpack matching that id could be located in the project",
                                                        buildpack_project.source_dir.to_string_lossy(), local_buildpack_uri_to_id(&uri)))
                    };
                BuildpackageUri {
                    uri: local_dependency_target_dir.to_string(),
                }
            } else {
                buildpackage_uri.clone()
            }
        })
        .collect();

    let buildpackage = Buildpackage {
        dependencies,
        ..buildpackage_data.buildpackage_descriptor.clone()
    };

    let buildpackage_content = match toml::to_string(&buildpackage) {
        Ok(buildpackage_content) => buildpackage_content,
        Err(error) => {
            fail_with_error(format!("Could not serialize package.toml content: {error}"));
        }
    };

    if let Err(error) = fs::write(
        buildpack_project.target_dir.join("package.toml"),
        buildpackage_content,
    ) {
        fail_with_error(format!(
            "Could not write package.toml to target directory: {error}"
        ));
    }

    let size_in_bytes =
        calculate_dir_size(&buildpack_project.target_dir).unwrap_or_else(|io_error| {
            fail_with_error(format!(
                "IO error while calculating buildpack directory size: {io_error}"
            ));
        });

    // Precision will only be lost for sizes bigger than 52 bits (~4 Petabytes), and even
    // then will only result in a less precise figure, so is not an issue.
    #[allow(clippy::cast_precision_loss)]
    let size_in_mb = size_in_bytes as f64 / (1024.0 * 1024.0);
    let relative_output_path =
        pathdiff::diff_paths(&buildpack_project.target_dir, &buildpack_project.source_dir)
            .unwrap_or_else(|| buildpack_project.target_dir.clone());

    eprintln!(
        "Successfully wrote buildpack directory: {} ({size_in_mb:.2} MiB)",
        relative_output_path.to_string_lossy(),
    );
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

#[allow(clippy::too_many_lines)]
fn get_buildpack_projects(
    buildpack_workspace: &BuildpackWorkspace,
    args: &PackageArgs,
) -> Vec<BuildpackProject> {
    let mut buildpack_projects: Vec<BuildpackProject> = vec![];

    for buildpack_dir in find_buildpack_directories(buildpack_workspace) {
        let buildpack_data = match read_buildpack_data(&buildpack_dir) {
            Ok(buildpack_data) => buildpack_data,
            Err(error) => {
                warn(formatdoc! { "
                    Ignoring buildpack project from {} 

                    To include this project, please verify that the `buildpack.toml` file:
                    • is readable
                    • contains valid buildpack metadata

                    Error: {:#?}
                ", &buildpack_dir.to_string_lossy(), error });
                continue;
            }
        };

        let buildpackage_data = if buildpack_dir.join("package.toml").exists() {
            match read_buildpackage_data(&buildpack_dir) {
                Ok(buildpackage_data) => Some(buildpackage_data),
                Err(error) => {
                    warn(formatdoc! { "
                        Ignoring buildpack project from {} 
    
                        To include this project, please verify that the `package.toml` file:
                        • is readable
                        • contains valid buildpackagage metadata
    
                        Error: {:#?}
                    ", &buildpack_dir.to_string_lossy(), error });
                    continue;
                }
            }
        } else {
            None
        };

        let buildpack_target_dir = if is_legacy_buildpack_project(&buildpack_dir) {
            buildpack_dir.clone()
        } else {
            buildpack_workspace
                .target_dir
                .join("buildpack")
                .join(if args.release { "release" } else { "debug" })
                .join(default_buildpack_directory_name(
                    &buildpack_data.buildpack_descriptor,
                ))
        };

        let id = match &buildpack_data.buildpack_descriptor {
            BuildpackDescriptor::Single(d) => d.buildpack.id.clone(),
            BuildpackDescriptor::Meta(d) => d.buildpack.id.clone(),
        };

        let mut local_dependencies: Vec<BuildpackId> = vec![];

        if let Some(buildpackage_data) = &buildpackage_data {
            let mut parsed_uris: Vec<URI> = vec![];
            let mut invalid_uris: Vec<String> = vec![];
            let mut invalid_local_buildpack_uris: Vec<String> = vec![];

            for buildpackage_uri in &buildpackage_data.buildpackage_descriptor.dependencies {
                if let Ok(uri) = URI::try_from(buildpackage_uri.uri.as_str()) {
                    parsed_uris.push(uri);
                } else {
                    invalid_uris.push(buildpackage_uri.uri.clone());
                }
            }

            if !invalid_uris.is_empty() {
                let invalid_uris: Vec<_> =
                    invalid_uris.iter().map(|uri| format!("• {uri}")).collect();
                warn(formatdoc! { "
                    Ignoring buildpack project from {} 

                    To include this project, please fix the following invalid URIs in the `package.toml` file:
                    {}
                ", &buildpack_dir.to_string_lossy(), invalid_uris.join("\n") });
                continue;
            }

            for uri in parsed_uris {
                if is_local_buildpack_uri(&uri) {
                    match local_buildpack_uri_to_id(&uri).parse::<BuildpackId>() {
                        Ok(local_buildpack_id) => local_dependencies.push(local_buildpack_id),
                        Err(_) => invalid_local_buildpack_uris.push(String::from(&uri.to_string())),
                    }
                }
            }

            if !invalid_local_buildpack_uris.is_empty() {
                let invalid_buildpack_ids: Vec<_> = invalid_local_buildpack_uris
                    .iter()
                    .map(|id| format!("• {id}"))
                    .collect();
                warn(formatdoc! { "
                    Ignoring buildpack project from {} 

                    To include this project, please fix the following URIs with invalid Buildpack Ids in the `package.toml` file:
                    {}
                ", &buildpack_dir.to_string_lossy(), invalid_buildpack_ids.join("\n") });
                continue;
            }
        };

        let buildpack_project = BuildpackProject {
            id,
            local_dependencies,
            buildpack_data,
            buildpackage_data,
            source_dir: buildpack_dir,
            target_dir: buildpack_target_dir,
        };

        buildpack_projects.push(buildpack_project);
    }

    buildpack_projects
}

fn get_buildpack_projects_to_compile(
    buildpack_projects: &[BuildpackProject],
) -> Vec<BuildpackProject> {
    let mut buildpack_projects_seen: HashSet<&BuildpackId> = HashSet::new();
    let mut buildpack_projects_to_compile: Vec<BuildpackProject> = vec![];

    for buildpack_project in get_buildpack_projects_in_scope(buildpack_projects) {
        for local_dependency_id in &buildpack_project.local_dependencies {
            let local_dependency = match buildpack_projects
                .iter()
                .find(|buildpack_project| &buildpack_project.id == local_dependency_id)
            {
                Some(local_dependency) => local_dependency,
                None => {
                    fail_with_error(format!("Buildpack `{}` depends on local buildpack `{}` but no valid buildpack matching that id could be located in the project",
                                    buildpack_project.id, local_dependency_id));
                }
            };

            if !buildpack_projects_seen.contains(&local_dependency.id) {
                buildpack_projects_seen.insert(&local_dependency.id);
                buildpack_projects_to_compile.push(local_dependency.clone());
            }
        }
        if !buildpack_projects_seen.contains(&buildpack_project.id) {
            buildpack_projects_seen.insert(&buildpack_project.id);
            buildpack_projects_to_compile.push(buildpack_project.clone());
        }
    }

    buildpack_projects_to_compile
}

fn get_cargo_metadata(buildpack_dir: &Path) -> Metadata {
    match MetadataCommand::new()
        .manifest_path(&buildpack_dir.join("Cargo.toml"))
        .exec()
    {
        Ok(cargo_metadata) => cargo_metadata,
        Err(error) => {
            fail_with_error(format!("Could not obtain metadata from Cargo: {error}"));
        }
    }
}

fn get_current_dir() -> PathBuf {
    match env::current_dir() {
        Ok(current_dir) => current_dir,
        Err(io_error) => {
            fail_with_error(format!("Could not determine current directory: {io_error}"));
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
            fail_with_error(format!("Could not locate project root: {error}"));
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
        Err(error) => fail_with_error(format!(
            "Failed to glob buildpack.toml files in {}: {error}",
            buildpack_workspace.root.to_string_lossy()
        )),
    };

    let exclude_target_pattern = match glob::Pattern::new(
        &buildpack_workspace
            .target_dir
            .join("**/buildpack.toml")
            .to_string_lossy(),
    ) {
        Ok(pattern) => pattern,
        Err(error) => fail_with_error(format!(
            "Invalid glob pattern for {}: {}",
            buildpack_workspace.target_dir.to_string_lossy(),
            error
        )),
    };

    paths
        .filter_map(Result::ok)
        .filter(|path| !exclude_target_pattern.matches(&path.to_string_lossy()))
        .map(|path| parent_dir(&path))
        .collect()
}

fn parent_dir(path: &Path) -> PathBuf {
    if let Some(parent) = path.parent() {
        parent.to_path_buf()
    } else {
        fail_with_error(format!(
            "Could not get parent directory from {}",
            path.to_string_lossy()
        ));
    }
}

fn is_legacy_buildpack_project(buildpack_project_dir: &Path) -> bool {
    let contains_buildpack_binaries = ["detect", "build"].iter().all(|bin_name| {
        let bin_path = buildpack_project_dir.join("bin").join(bin_name);
        bin_path.exists() && bin_path.is_file()
    });
    let not_a_cargo_project = !buildpack_project_dir.join("Cargo.toml").exists();
    contains_buildpack_binaries && not_a_cargo_project
}

fn is_local_buildpack_uri(uri: &URI) -> bool {
    &uri.scheme().to_string() == "libcnb"
}

fn local_buildpack_uri_to_id(uri: &URI) -> String {
    uri.to_string().replace("libcnb://", "")
}

fn find_local_buildpack_project<'a>(
    buildpack_projects: &'a [BuildpackProject],
    uri: &URI,
) -> Option<&'a BuildpackProject> {
    buildpack_projects.iter().find(|buildpack_project| {
        is_local_buildpack_uri(uri)
            && buildpack_project.id.to_string() == local_buildpack_uri_to_id(uri)
    })
}

fn get_buildpack_projects_in_scope(
    buildpack_projects: &[BuildpackProject],
) -> Vec<&BuildpackProject> {
    let current_dir = get_current_dir();
    if current_dir.join("buildpack.toml").exists() {
        buildpack_projects
            .iter()
            .filter(|buildpack_project| buildpack_project.source_dir.clone() == current_dir)
            .collect()
    } else {
        buildpack_projects.iter().collect()
    }
}

fn create_example_pack_command(buildpack_projects_to_compile: &[BuildpackProject]) -> String {
    let packaged_buildpack_dirs = get_buildpack_projects_in_scope(buildpack_projects_to_compile)
        .iter()
        .map(|buildpack_project| buildpack_project.target_dir.to_string_lossy())
        .collect::<Vec<_>>();

    let pack_buildpack_flags = packaged_buildpack_dirs
        .iter()
        .map(|packaged_buildpack_dir| format!("--buildpack {packaged_buildpack_dir}"))
        .collect::<Vec<_>>()
        .join(" \\\n  ");

    let pack_example_command = formatdoc! { "
        {BULB} To test your buildpack locally with pack, run:
        pack build my-image-name \\\n  {pack_buildpack_flags} \\\n  --path /path/to/application
    " };

    format!("\n{pack_example_command}")
}

fn fail_with_error<IntoString: Into<String>>(error: IntoString) -> ! {
    eprintln!("{CROSS_MARK} {}", error.into());
    std::process::exit(exit_code::UNSPECIFIED_ERROR);
}

fn warn<IntoString: Into<String>>(warning: IntoString) {
    eprintln!("{WARNING} {}", warning.into());
}
