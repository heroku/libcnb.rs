pub(crate) fn determine_buildpack_cargo_target_name(
    cargo_metadata: &cargo_metadata::Metadata,
) -> Result<String, DetermineBuildpackCargoTargetNameError> {
    let root_package = cargo_metadata
        .root_package()
        .ok_or(DetermineBuildpackCargoTargetNameError::NoRootPackage)?;

    let mut bin_targets: Vec<String> = binary_target_names_from_root_package(root_package);

    match bin_targets.len() {
        0 | 1 => bin_targets
            .pop()
            .ok_or(DetermineBuildpackCargoTargetNameError::NoBinTargets),
        _ => bin_targets
            .contains(&root_package.name)
            .then_some(root_package.name.clone())
            .ok_or(DetermineBuildpackCargoTargetNameError::AmbiguousBinTargets),
    }
}

#[derive(thiserror::Error, Debug)]
pub enum DetermineBuildpackCargoTargetNameError {
    #[error("Cargo metadata is missing the required root package")]
    NoRootPackage,
    #[error("No binary targets could be found in Cargo metadata")]
    NoBinTargets,
    #[error("Ambiguous binary targets found in Cargo metadata")]
    AmbiguousBinTargets,
}

/// Determines the names of all binary targets from the given Cargo metadata.
pub(crate) fn binary_target_names(cargo_metadata: &cargo_metadata::Metadata) -> Vec<String> {
    cargo_metadata
        .root_package()
        .map(binary_target_names_from_root_package)
        .unwrap_or_default()
}

fn binary_target_names_from_root_package(root_package: &cargo_metadata::Package) -> Vec<String> {
    root_package
        .targets
        .iter()
        .filter_map(|target| is_binary_target(target).then_some(target.name.clone()))
        .collect()
}

fn is_binary_target(target: &cargo_metadata::Target) -> bool {
    target.kind.contains(&String::from("bin"))
}
