use crate::cli::PackageArgs;
use crate::logging::{log, warn};
use crate::package::PackageCommandError::{
    CouldNotCalculateCompiledBuildpackSize, CouldNotCopyBuildpackTomlToTargetDir,
    CouldNotCreateBuildpackTargetDir, CouldNotObtainCargoMetadata,
    CouldNotSerializeBuildpackageData, CouldNotWriteBuildpackageData,
    CouldNotWriteToBuildpackTargetDirectory, CrossCompilationError,
    FailedToCleanBuildpackTargetDirectory, FailedToCompileBuildpackError,
    FailedToCreateLocalBuildPackageDependencies, MetaBuildpackIsMissingBuildpackageData,
    NoBuildpacksToCompile, UnresolvedLocalDependency,
};
use crate::package::PackageableBuildpackDependency::Local;
use crate::package::{PackageCommandError, PackageableBuildpack, PackageableBuildpackDependency};
use cargo_metadata::MetadataCommand;
use indoc::formatdoc;
use libcnb_data::buildpack::{BuildpackDescriptor, BuildpackId};
use libcnb_data::buildpackage::{Buildpackage, BuildpackageDependency};
use libcnb_package::build::build_buildpack_binaries;
use libcnb_package::cross_compile::{cross_compile_assistance, CrossCompileAssistance};
use libcnb_package::{assemble_buildpack_directory, CargoProfile};
use std::collections::{HashMap, HashSet};
use std::ffi::OsString;
use std::path::{Path, PathBuf};
use std::{env, fs, io};

pub(crate) fn package_buildpacks(
    buildpacks: Vec<PackageableBuildpack>,
    args: &PackageArgs,
) -> Result<Vec<PathBuf>, PackageCommandError> {
    let current_dir = env::current_dir().map_err(PackageCommandError::CouldNotGetCurrentDir)?;

    let buildpacks_by_id: HashMap<BuildpackId, PackageableBuildpack> = buildpacks
        .iter()
        .map(|buildpack| {
            let buildpack_id = &buildpack.buildpack_data.buildpack_descriptor.buildpack().id;
            (buildpack_id.clone(), buildpack.clone())
        })
        .collect();

    let requested_buildpacks: Vec<_> = if current_dir.join("buildpack.toml").exists() {
        buildpacks
            .into_iter()
            .filter(|buildpack| buildpack.source_dir == current_dir)
            .collect()
    } else {
        buildpacks
    };

    if requested_buildpacks.is_empty() {
        return Err(NoBuildpacksToCompile);
    }

    // get the requested buildpacks + all their dependencies
    let buildpacks_to_compile =
        get_buildpack_compilation_order(&requested_buildpacks, &buildpacks_by_id)?;

    let mut current_count = 1;
    let total_count = buildpacks_to_compile.len() as u64;
    for buildpack in &buildpacks_to_compile {
        log(format!(
            "📦 [{current_count}/{total_count}] Building {}",
            buildpack.buildpack_data.buildpack_descriptor.buildpack().id
        ));
        match buildpack.buildpack_data.buildpack_descriptor {
            BuildpackDescriptor::Single(_) => {
                compile_single_buildpack_project_for_packaging(buildpack, args)?;
            }
            BuildpackDescriptor::Meta(_) => {
                compile_meta_buildpack_project_for_packaging(buildpack)?;
            }
        }
        current_count += 1;
    }
    log("✨ Packaging successfully finished!");

    // collect the output directory of each requested buildpack
    let packaged_buildpacks: Vec<_> = requested_buildpacks
        .into_iter()
        .map(|buildpack| {
            if contains_buildpack_binaries(&buildpack.source_dir) {
                buildpack.source_dir
            } else {
                buildpack.target_dir
            }
        })
        .collect();

    log(create_example_pack_command(&packaged_buildpacks));

    Ok(packaged_buildpacks)
}

fn get_buildpack_compilation_order(
    requested_buildpacks: &[PackageableBuildpack],
    buildpacks_by_id: &HashMap<BuildpackId, PackageableBuildpack>,
) -> Result<Vec<PackageableBuildpack>, PackageCommandError> {
    let mut buildpacks_seen: HashSet<&BuildpackId> = HashSet::new();
    let mut buildpacks_with_dependencies: Vec<PackageableBuildpack> = vec![];

    for buildpack in requested_buildpacks {
        for dependency in &buildpack.dependencies {
            if let Local { buildpack_id, .. } = dependency {
                if !buildpacks_seen.contains(&buildpack_id) {
                    let dependent_buildpack =
                        buildpacks_by_id
                            .get(buildpack_id)
                            .ok_or(UnresolvedLocalDependency(
                                buildpack.clone(),
                                buildpack_id.clone(),
                            ))?;

                    buildpacks_seen.insert(buildpack_id);
                    buildpacks_with_dependencies.push(dependent_buildpack.clone());
                }
            }
        }

        let buildpack_id = &buildpack.buildpack_data.buildpack_descriptor.buildpack().id;
        if !buildpacks_seen.contains(&buildpack_id) {
            buildpacks_seen.insert(buildpack_id);
            buildpacks_with_dependencies.push(buildpack.clone());
        }
    }

    Ok(buildpacks_with_dependencies)
}

fn compile_single_buildpack_project_for_packaging(
    buildpack: &PackageableBuildpack,
    args: &PackageArgs,
) -> Result<(), PackageCommandError> {
    if contains_buildpack_binaries(&buildpack.source_dir) {
        log("Non-rust buildpack directory detected, no compilation will be performed...");
        return Ok(());
    }

    let cargo_profile = if args.release {
        CargoProfile::Release
    } else {
        CargoProfile::Dev
    };

    let cargo_metadata = MetadataCommand::new()
        .manifest_path(&buildpack.source_dir.join("Cargo.toml"))
        .exec()
        .map_err(CouldNotObtainCargoMetadata)?;

    let cargo_build_env: Vec<(OsString, OsString)> = get_cargo_cargo_build_env(args)?;

    log(format!("Building binaries ({})...", args.target));
    let buildpack_binaries = build_buildpack_binaries(
        &buildpack.source_dir,
        &cargo_metadata,
        cargo_profile,
        &cargo_build_env,
        &args.target,
    )
    .map_err(FailedToCompileBuildpackError)?;

    log("Writing buildpack directory...");
    clean_buildpack_target_dir(buildpack)?;
    assemble_buildpack_directory(
        &buildpack.target_dir,
        &buildpack.buildpack_data.buildpack_descriptor_path,
        &buildpack_binaries,
    )
    .map_err(CouldNotWriteToBuildpackTargetDirectory)?;

    report_compiled_buildpack(buildpack)
}

fn compile_meta_buildpack_project_for_packaging(
    buildpack: &PackageableBuildpack,
) -> Result<(), PackageCommandError> {
    log("Writing buildpack directory...");
    clean_buildpack_target_dir(buildpack)?;
    fs::create_dir_all(&buildpack.target_dir).map_err(CouldNotCreateBuildpackTargetDir)?;
    fs::copy(
        &buildpack.buildpack_data.buildpack_descriptor_path,
        buildpack.target_dir.join("buildpack.toml"),
    )
    .map_err(CouldNotCopyBuildpackTomlToTargetDir)?;
    update_and_write_package_toml(buildpack)?;
    report_compiled_buildpack(buildpack)
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

fn contains_buildpack_binaries(dir: &Path) -> bool {
    ["bin/detect", "bin/build"]
        .into_iter()
        .map(|path| dir.join(path))
        .all(|path| path.is_file())
}

fn get_cargo_cargo_build_env(
    args: &PackageArgs,
) -> Result<Vec<(OsString, OsString)>, PackageCommandError> {
    if args.no_cross_compile_assistance {
        return Ok(vec![]);
    }

    log("Determining automatic cross-compile settings...");
    match cross_compile_assistance(&args.target) {
        CrossCompileAssistance::HelpText(help_text) => Err(CrossCompilationError(help_text)),
        CrossCompileAssistance::NoAssistance => {
            warn(formatdoc! { "
                Could not determine automatic cross-compile settings for target triple {}.
                This is not an error, but without proper cross-compile settings in your Cargo manifest and locally installed toolchains, compilation might fail.
                To disable this warning, pass --no-cross-compile-assistance.
            ", args.target });
            Ok(vec![])
        }
        CrossCompileAssistance::Configuration { cargo_env } => Ok(cargo_env),
    }
}

fn clean_buildpack_target_dir(buildpack: &PackageableBuildpack) -> Result<(), PackageCommandError> {
    if buildpack.target_dir.exists() {
        fs::remove_dir_all(&buildpack.target_dir).map_err(FailedToCleanBuildpackTargetDirectory)?;
    }
    Ok(())
}

fn report_compiled_buildpack(buildpack: &PackageableBuildpack) -> Result<(), PackageCommandError> {
    let size_in_bytes = calculate_dir_size(&buildpack.target_dir)
        .map_err(CouldNotCalculateCompiledBuildpackSize)?;

    // Precision will only be lost for sizes bigger than 52 bits (~4 Petabytes), and even
    // then will only result in a less precise figure, so is not an issue.
    #[allow(clippy::cast_precision_loss)]
    let size_in_mb = size_in_bytes as f64 / (1024.0 * 1024.0);
    let relative_output_path = pathdiff::diff_paths(&buildpack.target_dir, &buildpack.source_dir)
        .unwrap_or_else(|| buildpack.source_dir.clone());

    eprintln!(
        "Successfully wrote buildpack directory: {} ({size_in_mb:.2} MiB)",
        relative_output_path.to_string_lossy(),
    );

    Ok(())
}

fn update_and_write_package_toml(
    buildpack: &PackageableBuildpack,
) -> Result<(), PackageCommandError> {
    let buildpackage_data = &buildpack
        .buildpackage_data
        .as_ref()
        .ok_or(MetaBuildpackIsMissingBuildpackageData)?;

    let (buildpackage_dependencies, errors): (Vec<_>, Vec<_>) = buildpack
        .dependencies
        .iter()
        .map(|dependency| match dependency {
            Local { target_dir, .. } => BuildpackageDependency::try_from(target_dir.clone()),
            PackageableBuildpackDependency::External(buildpackage_dependency) => {
                Ok(buildpackage_dependency.clone())
            }
        })
        .partition(Result::is_ok);

    if !errors
        .into_iter()
        .filter_map(Result::err)
        .collect::<Vec<_>>()
        .is_empty()
    {
        return Err(FailedToCreateLocalBuildPackageDependencies);
    }

    let buildpackage = Buildpackage {
        dependencies: buildpackage_dependencies
            .into_iter()
            .filter_map(Result::ok)
            .collect(),
        ..buildpackage_data.buildpackage_descriptor.clone()
    };

    let buildpackage_content =
        toml::to_string(&buildpackage).map_err(CouldNotSerializeBuildpackageData)?;

    fs::write(
        buildpack.target_dir.join("package.toml"),
        buildpackage_content,
    )
    .map_err(CouldNotWriteBuildpackageData)
}

fn create_example_pack_command(packaged_buildpacks: &[PathBuf]) -> String {
    let pack_buildpack_flags = packaged_buildpacks
        .iter()
        .map(|packaged_buildpack| format!("--buildpack {}", packaged_buildpack.to_string_lossy()))
        .collect::<Vec<_>>()
        .join(" \\\n  ");

    let pack_example_command = formatdoc! { "
        💡 To test your buildpack locally with pack, run:
        pack build my-image-name \\\n  {pack_buildpack_flags} \\\n  --path /path/to/application
    " };

    format!("\n{pack_example_command}")
}
