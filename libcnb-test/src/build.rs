use libcnb_common::toml_file::{TomlFileError, read_toml_file};
use libcnb_data::buildpack::{BuildpackDescriptor, BuildpackId};
use libcnb_package::buildpack_dependency_graph::{
    BuildBuildpackDependencyGraphError, build_libcnb_buildpacks_dependency_graph,
};
use libcnb_package::cross_compile::{CrossCompileAssistance, cross_compile_assistance};
use libcnb_package::dependency_graph::{GetDependenciesError, get_dependencies};
use libcnb_package::output::create_packaged_buildpack_dir_resolver;
use libcnb_package::{CargoProfile, FindCargoWorkspaceRootError, find_cargo_workspace_root_dir};
use std::collections::BTreeMap;
use std::ffi::OsString;
use std::path::{Path, PathBuf};
use std::{fs, io};

#[derive(Clone, Debug, PartialEq)]
pub(crate) struct CargoEnvAddition {
    pub(crate) key: OsString,
    pub(crate) value: OsString,
    pub(crate) separator: OsString,
}

fn apply_cargo_env_additions(
    cargo_build_env: &mut Vec<(OsString, OsString)>,
    additions: &[CargoEnvAddition],
) {
    for addition in additions {
        if let Some(existing) = cargo_build_env.iter_mut().find(|(k, _)| k == &addition.key) {
            existing.1.push(&addition.separator);
            existing.1.push(&addition.value);
        } else {
            cargo_build_env.push((addition.key.clone(), addition.value.clone()));
        }
    }
}

/// Packages the current crate as a buildpack into the provided directory.
pub(crate) fn package_crate_buildpack(
    cargo_profile: CargoProfile,
    target_triple: impl AsRef<str>,
    cargo_manifest_dir: &Path,
    target_buildpack_dir: &Path,
    cargo_env_additions: &[CargoEnvAddition],
) -> Result<PathBuf, PackageBuildpackError> {
    let buildpack_toml = cargo_manifest_dir.join("buildpack.toml");

    if !buildpack_toml.exists() {
        return Err(PackageBuildpackError::BuildpackDescriptorNotFound(
            buildpack_toml,
        ));
    }

    let buildpack_descriptor: BuildpackDescriptor = read_toml_file(buildpack_toml)
        .map_err(PackageBuildpackError::CannotReadBuildpackDescriptor)?;

    package_buildpack(
        &buildpack_descriptor.buildpack().id,
        cargo_profile,
        target_triple,
        cargo_manifest_dir,
        target_buildpack_dir,
        cargo_env_additions,
    )
}

pub(crate) fn package_buildpack(
    buildpack_id: &BuildpackId,
    cargo_profile: CargoProfile,
    target_triple: impl AsRef<str>,
    cargo_manifest_dir: &Path,
    target_buildpack_dir: &Path,
    cargo_env_additions: &[CargoEnvAddition],
) -> Result<PathBuf, PackageBuildpackError> {
    let mut cargo_build_env = match cross_compile_assistance(target_triple.as_ref()) {
        CrossCompileAssistance::HelpText(help_text) => {
            return Err(PackageBuildpackError::CrossCompileToolchainNotFound(
                help_text,
            ));
        }
        CrossCompileAssistance::NoAssistance => Vec::new(),
        CrossCompileAssistance::Configuration { cargo_env } => cargo_env,
    };

    apply_cargo_env_additions(&mut cargo_build_env, cargo_env_additions);

    let workspace_root_path = find_cargo_workspace_root_dir(cargo_manifest_dir)
        .map_err(PackageBuildpackError::FindCargoWorkspaceRoot)?;

    let buildpack_dir_resolver = create_packaged_buildpack_dir_resolver(
        target_buildpack_dir,
        cargo_profile,
        target_triple.as_ref(),
    );

    let buildpack_dependency_graph = build_libcnb_buildpacks_dependency_graph(&workspace_root_path)
        .map_err(PackageBuildpackError::BuildBuildpackDependencyGraph)?;

    let root_node = buildpack_dependency_graph
        .node_weights()
        .find(|node| &node.buildpack_id == buildpack_id)
        .ok_or_else(|| {
            PackageBuildpackError::BuildpackIdNotFound(buildpack_id.clone(), workspace_root_path)
        })?;

    let build_order = get_dependencies(&buildpack_dependency_graph, &[root_node])
        .map_err(PackageBuildpackError::GetDependencies)?;

    let mut packaged_buildpack_dirs = BTreeMap::new();
    for node in &build_order {
        let buildpack_destination_dir = buildpack_dir_resolver(&node.buildpack_id);

        fs::create_dir_all(&buildpack_destination_dir).map_err(|error| {
            PackageBuildpackError::CannotCreateDirectory(buildpack_destination_dir.clone(), error)
        })?;

        libcnb_package::package::package_buildpack(
            &node.path,
            cargo_profile,
            target_triple.as_ref(),
            &cargo_build_env,
            &buildpack_destination_dir,
            &packaged_buildpack_dirs,
        )
        .map_err(PackageBuildpackError::PackageBuildpack)?;

        packaged_buildpack_dirs.insert(node.buildpack_id.clone(), buildpack_destination_dir);
    }

    Ok(buildpack_dir_resolver(buildpack_id))
}

#[derive(thiserror::Error, Debug)]
pub(crate) enum PackageBuildpackError {
    #[error("Couldn't find a buildpack.toml file at {0}")]
    BuildpackDescriptorNotFound(PathBuf),
    #[error("Couldn't find a buildpack with ID '{0}' in the workspace at {1}")]
    BuildpackIdNotFound(BuildpackId, PathBuf),
    #[error("Couldn't create directory {0}: {1}")]
    CannotCreateDirectory(PathBuf, io::Error),
    #[error("Couldn't read buildpack.toml: {0}")]
    CannotReadBuildpackDescriptor(TomlFileError),
    #[error("Couldn't calculate buildpack dependency graph: {0}")]
    BuildBuildpackDependencyGraph(BuildBuildpackDependencyGraphError),
    #[error("Couldn't find cross-compilation toolchain.\n\n{0}")]
    CrossCompileToolchainNotFound(String),
    #[error("Couldn't find Cargo workspace root: {0}")]
    FindCargoWorkspaceRoot(FindCargoWorkspaceRootError),
    #[error("Couldn't get buildpack dependencies: {0}")]
    GetDependencies(GetDependenciesError<BuildpackId>),
    #[error(transparent)]
    PackageBuildpack(libcnb_package::package::PackageBuildpackError),
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn apply_cargo_env_addition_inserts_into_empty() {
        let mut env = Vec::new();
        apply_cargo_env_additions(
            &mut env,
            &[CargoEnvAddition {
                key: OsString::from("RUSTFLAGS"),
                value: OsString::from("-C opt-level=3"),
                separator: OsString::from(" "),
            }],
        );
        assert_eq!(env.len(), 1);
        assert_eq!(env[0].0, "RUSTFLAGS");
        assert_eq!(env[0].1, "-C opt-level=3");
    }

    #[test]
    fn apply_cargo_env_addition_inserts_new_key_alongside_existing() {
        let mut env = vec![(OsString::from("CC"), OsString::from("clang"))];
        apply_cargo_env_additions(
            &mut env,
            &[CargoEnvAddition {
                key: OsString::from("RUSTFLAGS"),
                value: OsString::from("-C lto"),
                separator: OsString::from(" "),
            }],
        );
        assert_eq!(env.len(), 2);
        assert_eq!(env[0], (OsString::from("CC"), OsString::from("clang")));
        assert_eq!(
            env[1],
            (OsString::from("RUSTFLAGS"), OsString::from("-C lto"))
        );
    }

    #[test]
    fn apply_cargo_env_addition_joins_existing_key() {
        let mut env = vec![(OsString::from("RUSTFLAGS"), OsString::from("-C linker=lld"))];
        apply_cargo_env_additions(
            &mut env,
            &[CargoEnvAddition {
                key: OsString::from("RUSTFLAGS"),
                value: OsString::from("-C instrument-coverage"),
                separator: OsString::from(" "),
            }],
        );
        assert_eq!(env.len(), 1);
        assert_eq!(env[0].0, "RUSTFLAGS");
        assert_eq!(env[0].1, "-C linker=lld -C instrument-coverage");
    }
}
