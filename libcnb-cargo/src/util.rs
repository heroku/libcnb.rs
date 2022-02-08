use cargo_metadata::Metadata;

pub fn binary_target_names(cargo_metadata: &Metadata) -> Vec<String> {
    cargo_metadata
        .root_package()
        .map(|root_package| {
            root_package
                .targets
                .iter()
                .filter_map(|target| {
                    if target.kind.contains(&String::from("bin")) {
                        Some(target.name.clone())
                    } else {
                        None
                    }
                })
                .collect()
        })
        .unwrap_or_default()
}
