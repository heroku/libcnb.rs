use crate::cli::PackageArgs;
use crate::package::error::Error;
use cargo_metadata::MetadataCommand;
use libcnb_data::buildpack::{BuildpackDescriptor, BuildpackId};
use libcnb_data::buildpackage::Buildpackage;
use libcnb_package::build::build_buildpack_binaries;
use libcnb_package::buildpack_dependency::{
    rewrite_buildpackage_local_dependencies,
    rewrite_buildpackage_relative_path_dependencies_to_absolute,
};
use libcnb_package::buildpack_package::{read_buildpack_package, BuildpackPackage};
use libcnb_package::cross_compile::{cross_compile_assistance, CrossCompileAssistance};
use libcnb_package::dependency_graph::{create_dependency_graph, get_dependencies};
use libcnb_package::{
    assemble_buildpack_directory, find_buildpack_dirs, find_cargo_workspace_root_dir,
    get_buildpack_package_dir, CargoProfile,
};
use std::collections::HashMap;
use std::path::{Path, PathBuf};

type Result<T> = std::result::Result<T, Error>;

pub(crate) fn execute(args: &PackageArgs) -> Result<()> {
    eprintln!("🔍 Locating buildpacks...");

    let current_dir = std::env::current_dir().map_err(Error::GetCurrentDir)?;

    let workspace_root_path = find_cargo_workspace_root_dir(&current_dir)?;

    let package_dir = args
        .package_dir
        .clone()
        .map_or_else(|| get_default_package_dir(&workspace_root_path), Ok)?;

    std::fs::create_dir_all(&package_dir)
        .map_err(|e| Error::CreatePackageDirectory(package_dir.clone(), e))?;

    let buildpack_dirs = find_buildpack_dirs(&workspace_root_path, &[package_dir.clone()])
        .map_err(|e| Error::FindBuildpackDirs(workspace_root_path, e))?;

    let buildpack_packages = buildpack_dirs
        .into_iter()
        .map(read_buildpack_package)
        .collect::<std::result::Result<Vec<_>, _>>()?;

    let buildpack_packages_graph = create_dependency_graph(buildpack_packages)?;

    let target_directories_index = buildpack_packages_graph
        .node_weights()
        .map(|buildpack_package| {
            let id = buildpack_package.buildpack_id();
            let target_dir = if contains_buildpack_binaries(&buildpack_package.path) {
                buildpack_package.path.clone()
            } else {
                get_buildpack_package_dir(id, &package_dir, args.release, &args.target)
            };
            (id, target_dir)
        })
        .collect::<HashMap<_, _>>();

    let buildpack_packages_requested = buildpack_packages_graph
        .node_weights()
        .filter(|buildpack_package| {
            // If we're in a directory with a buildpack.toml file, we only want to build the
            // buildpack from this directory. Otherwise, all of them.
            if current_dir.join("buildpack.toml").exists() {
                buildpack_package.path == current_dir
            } else {
                true
            }
        })
        .collect::<Vec<_>>();

    if buildpack_packages_requested.is_empty() {
        Err(Error::NoBuildpacksFound)?;
    }

    let build_order = get_dependencies(&buildpack_packages_graph, &buildpack_packages_requested)?;

    let lookup_target_dir = |buildpack_package: &BuildpackPackage| {
        target_directories_index
            .get(&buildpack_package.buildpack_id())
            .ok_or(Error::TargetDirectoryLookup {
                buildpack_id: buildpack_package.buildpack_id().clone(),
            })
            .map(std::clone::Clone::clone)
    };

    let mut current_count = 1;
    let total_count = build_order.len();
    for buildpack_package in &build_order {
        eprintln!(
            "📦 [{current_count}/{total_count}] Building {}",
            buildpack_package.buildpack_id()
        );
        let target_dir = lookup_target_dir(buildpack_package)?;
        match buildpack_package.buildpack_data.buildpack_descriptor {
            BuildpackDescriptor::Single(_) => {
                if contains_buildpack_binaries(&buildpack_package.path) {
                    eprintln!("Not a libcnb.rs buildpack, nothing to compile...");
                } else {
                    package_single_buildpack(buildpack_package, &target_dir, args)?;
                }
            }
            BuildpackDescriptor::Meta(_) => {
                package_meta_buildpack(buildpack_package, &target_dir, &target_directories_index)?;
            }
        }
        current_count += 1;
    }

    eprint_pack_command_hint(
        build_order
            .into_iter()
            .map(lookup_target_dir)
            .collect::<Result<Vec<_>>>()?,
    );

    print_requested_buildpack_output_dirs(
        buildpack_packages_requested
            .into_iter()
            .map(lookup_target_dir)
            .collect::<Result<Vec<_>>>()?,
    );

    Ok(())
}

fn package_single_buildpack(
    buildpack_package: &BuildpackPackage,
    target_dir: &Path,
    args: &PackageArgs,
) -> Result<()> {
    let cargo_profile = if args.release {
        CargoProfile::Release
    } else {
        CargoProfile::Dev
    };

    let target_triple = &args.target;

    let cargo_metadata = MetadataCommand::new()
        .manifest_path(&buildpack_package.path.join("Cargo.toml"))
        .exec()
        .map_err(|e| Error::ReadCargoMetadata {
            path: buildpack_package.path.clone(),
            source: e,
        })?;

    let cargo_build_env = if args.no_cross_compile_assistance {
        vec![]
    } else {
        eprintln!("Determining automatic cross-compile settings...");
        match cross_compile_assistance(target_triple) {
            CrossCompileAssistance::Configuration { cargo_env } => cargo_env,

            CrossCompileAssistance::NoAssistance => {
                eprintln!("Could not determine automatic cross-compile settings for target triple {target_triple}.");
                eprintln!("This is not an error, but without proper cross-compile settings in your Cargo manifest and locally installed toolchains, compilation might fail.");
                eprintln!("To disable this warning, pass --no-cross-compile-assistance.");
                vec![]
            }

            CrossCompileAssistance::HelpText(help_text) => {
                Err(Error::CrossCompilationHelp { message: help_text })?
            }
        }
    };

    eprintln!("Building binaries ({target_triple})...");

    let buildpack_binaries = build_buildpack_binaries(
        &buildpack_package.path,
        &cargo_metadata,
        cargo_profile,
        &cargo_build_env,
        target_triple,
    )?;

    eprintln!("Writing buildpack directory...");

    clean_target_directory(target_dir)?;

    assemble_buildpack_directory(
        target_dir,
        &buildpack_package.buildpack_data.buildpack_descriptor_path,
        &buildpack_binaries,
    )
    .map_err(|e| Error::AssembleBuildpackDirectory(target_dir.to_path_buf(), e))?;

    let buildpackage_content =
        toml::to_string(&Buildpackage::default()).map_err(Error::SerializeBuildpackage)?;

    std::fs::write(target_dir.join("package.toml"), buildpackage_content)
        .map_err(|e| Error::WriteBuildpackage(target_dir.to_path_buf(), e))?;

    eprint_compiled_buildpack_success(&buildpack_package.path, target_dir)
}

fn package_meta_buildpack(
    buildpack_package: &BuildpackPackage,
    target_dir: &Path,
    target_dirs_by_buildpack_id: &HashMap<&BuildpackId, PathBuf>,
) -> Result<()> {
    eprintln!("Writing buildpack directory...");

    clean_target_directory(target_dir)?;

    std::fs::create_dir_all(target_dir)
        .map_err(|e| Error::CreateBuildpackTargetDirectory(target_dir.to_path_buf(), e))?;

    std::fs::copy(
        &buildpack_package.buildpack_data.buildpack_descriptor_path,
        target_dir.join("buildpack.toml"),
    )
    .map_err(|e| Error::WriteBuildpack(target_dir.to_path_buf(), e))?;

    let buildpackage_content = &buildpack_package
        .buildpackage_data
        .as_ref()
        .map(|buildpackage_data| &buildpackage_data.buildpackage_descriptor)
        .ok_or(Error::MissingBuildpackageData)
        .and_then(|buildpackage| {
            rewrite_buildpackage_local_dependencies(buildpackage, target_dirs_by_buildpack_id)
                .map_err(std::convert::Into::into)
        })
        .and_then(|buildpackage| {
            rewrite_buildpackage_relative_path_dependencies_to_absolute(
                &buildpackage,
                &buildpack_package.path,
            )
            .map_err(std::convert::Into::into)
        })
        .and_then(|buildpackage| {
            toml::to_string(&buildpackage).map_err(Error::SerializeBuildpackage)
        })?;

    std::fs::write(target_dir.join("package.toml"), buildpackage_content)
        .map_err(|e| Error::WriteBuildpackage(target_dir.to_path_buf(), e))?;

    eprint_compiled_buildpack_success(&buildpack_package.path, target_dir)
}

fn eprint_pack_command_hint(pack_directories: Vec<PathBuf>) {
    eprintln!("✨ Packaging successfully finished!");
    eprintln!();
    eprintln!("💡 To test your buildpack locally with pack, run:");
    eprintln!("pack build my-image-name \\");
    for dir in pack_directories {
        eprintln!("  --buildpack {} \\", dir.to_string_lossy());
    }
    eprintln!("  --path /path/to/application");
    eprintln!();
}

fn print_requested_buildpack_output_dirs(output_directories: Vec<PathBuf>) {
    let mut output_directories = output_directories
        .into_iter()
        .map(|dir| dir.to_string_lossy().to_string())
        .collect::<Vec<_>>();
    output_directories.sort();
    for dir in output_directories {
        println!("{dir}");
    }
}

fn clean_target_directory(dir: &Path) -> Result<()> {
    if dir.exists() {
        std::fs::remove_dir_all(dir)
            .map_err(|e| Error::CleanBuildpackTargetDirectory(dir.to_path_buf(), e))?;
    }
    Ok(())
}

fn eprint_compiled_buildpack_success(source_dir: &Path, target_dir: &Path) -> Result<()> {
    let size_in_bytes = calculate_dir_size(target_dir)
        .map_err(|e| Error::CalculateDirectorySize(target_dir.to_path_buf(), e))?;

    // Precision will only be lost for sizes bigger than 52 bits (~4 Petabytes), and even
    // then will only result in a less precise figure, so is not an issue.
    #[allow(clippy::cast_precision_loss)]
    let size_in_mb = size_in_bytes as f64 / (1024.0 * 1024.0);
    let relative_output_path =
        pathdiff::diff_paths(target_dir, source_dir).unwrap_or_else(|| source_dir.to_path_buf());

    eprintln!(
        "Successfully wrote buildpack directory: {} ({size_in_mb:.2} MiB)",
        relative_output_path.to_string_lossy(),
    );

    Ok(())
}

/// Recursively calculate the size of a directory and its contents in bytes.
// Not using `fs_extra::dir::get_size` since it doesn't handle symlinks correctly:
// https://github.com/webdesus/fs_extra/issues/59
fn calculate_dir_size(path: impl AsRef<Path>) -> std::io::Result<u64> {
    let mut size_in_bytes = 0;

    // The size of the directory entry (ie: its metadata only, not the directory contents).
    size_in_bytes += path.as_ref().metadata()?.len();

    for entry in std::fs::read_dir(&path)? {
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

fn contains_buildpack_binaries(dir: &Path) -> bool {
    ["bin/detect", "bin/build"]
        .into_iter()
        .map(|path| dir.join(path))
        .all(|path| path.is_file())
}

fn get_default_package_dir(workspace_root_path: &Path) -> Result<PathBuf> {
    MetadataCommand::new()
        .manifest_path(&workspace_root_path.join("Cargo.toml"))
        .exec()
        .map(|metadata| metadata.workspace_root.into_std_path_buf().join("packaged"))
        .map_err(|e| Error::GetBuildpackOutputDir(workspace_root_path.to_path_buf(), e))
}
