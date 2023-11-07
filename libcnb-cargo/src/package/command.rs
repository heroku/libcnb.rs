use crate::cli::PackageArgs;
use crate::package::error::Error;
use libcnb_data::buildpack::BuildpackId;
use libcnb_package::buildpack_dependency_graph::build_libcnb_buildpacks_dependency_graph;
use libcnb_package::cross_compile::{cross_compile_assistance, CrossCompileAssistance};
use libcnb_package::dependency_graph::get_dependencies;
use libcnb_package::output::create_packaged_buildpack_dir_resolver;
use libcnb_package::util::{absolutize_path, calculate_dir_size};
use libcnb_package::{find_cargo_workspace_root_dir, CargoProfile};
use std::collections::BTreeMap;
use std::fs;
use std::path::{Path, PathBuf};

pub(crate) fn execute(args: &PackageArgs) -> Result<(), Error> {
    let current_dir = std::env::current_dir().map_err(Error::CannotGetCurrentDir)?;

    let cargo_profile = if args.release {
        CargoProfile::Release
    } else {
        CargoProfile::Dev
    };

    let workspace_root_path =
        find_cargo_workspace_root_dir(&current_dir).map_err(Error::CannotFindCargoWorkspaceRoot)?;

    eprintln!("🚚 Preparing package directory...");
    let package_dir = absolutize_path(
        &args
            .package_dir
            .clone()
            .unwrap_or(workspace_root_path.join("packaged")),
        &current_dir,
    );

    fs::create_dir_all(&package_dir)
        .map_err(|error| Error::CannotCreatePackageDirectory(package_dir.clone(), error))?;

    let buildpack_dir_resolver =
        create_packaged_buildpack_dir_resolver(&package_dir, cargo_profile, &args.target);

    eprintln!("🖥️ Gathering Cargo configuration (for {})", args.target);
    let cargo_build_env = if args.no_cross_compile_assistance {
        Vec::new()
    } else {
        match cross_compile_assistance(&args.target) {
            CrossCompileAssistance::Configuration { cargo_env } => cargo_env,
            CrossCompileAssistance::NoAssistance => {
                eprintln!(
                    "Couldn't determine automatic cross-compile settings for target triple {}.",
                    args.target
                );
                eprintln!("This is not an error, but without proper cross-compile settings in your Cargo manifest and locally installed toolchains, compilation might fail.");
                eprintln!("To disable this warning, pass --no-cross-compile-assistance.");
                Vec::new()
            }
            CrossCompileAssistance::HelpText(help_text) => {
                eprintln!("{help_text}");
                return Err(Error::CannotConfigureCrossCompilation);
            }
        }
    };

    eprintln!("🏗️ Building buildpack dependency graph...");
    let buildpack_dependency_graph = build_libcnb_buildpacks_dependency_graph(&workspace_root_path)
        .map_err(Error::CannotBuildBuildpackDependencyGraph)?;

    eprintln!("🔀 Determining build order...");
    let root_nodes = buildpack_dependency_graph
        .node_weights()
        .find(|node| node.path == current_dir)
        .map(|node| vec![node])
        .or_else(|| {
            current_dir.eq(&workspace_root_path).then(|| {
                buildpack_dependency_graph
                    .node_weights()
                    .collect::<Vec<_>>()
            })
        })
        .unwrap_or_default();

    let build_order = get_dependencies(&buildpack_dependency_graph, &root_nodes)
        .map_err(Error::CannotGetDependencies)?;

    if build_order.is_empty() {
        return Err(Error::NoBuildpacksFound);
    }

    eprintln!("🚚 Building {} buildpacks...", build_order.len());
    let mut packaged_buildpack_dirs = BTreeMap::new();
    for (node_index, node) in build_order.iter().enumerate() {
        eprintln!(
            "📦 [{}/{}] Building {} (./{})",
            node_index + 1,
            build_order.len(),
            node.buildpack_id,
            pathdiff::diff_paths(&node.path, &current_dir)
                .unwrap_or_else(|| node.path.clone())
                .to_string_lossy()
        );

        let buildpack_destination_dir = buildpack_dir_resolver(&node.buildpack_id);
        let _ = fs::remove_dir_all(&buildpack_destination_dir);
        fs::create_dir_all(&buildpack_destination_dir).map_err(|error| {
            Error::CannotCreateBuildpackDestinationDir(buildpack_destination_dir.clone(), error)
        })?;

        libcnb_package::package::package_buildpack(
            &node.path,
            cargo_profile,
            &args.target,
            &cargo_build_env,
            &buildpack_destination_dir,
            &packaged_buildpack_dirs,
        )
        .map_err(Error::CannotPackageBuildpack)?;

        eprint_compiled_buildpack_success(&current_dir, &buildpack_destination_dir);

        packaged_buildpack_dirs.insert(node.buildpack_id.clone(), buildpack_destination_dir);
    }

    eprint_pack_command_hint(&packaged_buildpack_dirs, &current_dir);

    packaged_buildpack_dirs
        .iter()
        .filter(|(id, _)| root_nodes.iter().any(|node| node.buildpack_id == **id))
        .for_each(|(_, packaged_buildpack_dir)| {
            println!("{}", packaged_buildpack_dir.to_string_lossy());
        });

    Ok(())
}

fn eprint_pack_command_hint(
    packaged_buildpack_dirs: &BTreeMap<BuildpackId, PathBuf>,
    current_dir: &Path,
) {
    eprintln!("✨ Packaging successfully finished!");
    eprintln!();
    eprintln!("💡 To test your buildpack locally with pack, run:");
    eprintln!("pack build my-image-name \\");
    for dir in packaged_buildpack_dirs.values() {
        eprintln!(
            "  --buildpack {} \\",
            pathdiff::diff_paths(dir, current_dir)
                .unwrap_or_else(|| dir.clone())
                .to_string_lossy()
        );
    }
    eprintln!("  --path /path/to/application");
    eprintln!();
}

fn eprint_compiled_buildpack_success(current_dir: &Path, target_dir: &Path) {
    let size_string = calculate_dir_size(target_dir)
        .map(|size_in_bytes| {
            // Precision will only be lost for sizes bigger than 52 bits (~4 Petabytes), and even
            // then will only result in a less precise figure, so is not an issue.
            #[allow(clippy::cast_precision_loss)]
            let size_in_mib = size_in_bytes as f64 / (1024.0 * 1024.0);
            format!("{size_in_mib:.2}")
        })
        .unwrap_or(String::from("<unknown>"));

    let relative_output_path =
        pathdiff::diff_paths(target_dir, current_dir).unwrap_or_else(|| target_dir.to_path_buf());

    eprintln!(
        "Successfully wrote buildpack directory: {} ({size_string} MiB)",
        relative_output_path.to_string_lossy(),
    );
}
