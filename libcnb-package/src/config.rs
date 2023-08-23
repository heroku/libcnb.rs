use cargo_metadata::{Metadata, Target};

#[derive(Debug)]
pub(crate) struct Config {
    pub buildpack_target: String,
}

pub(crate) fn config_from_metadata(cargo_metadata: &Metadata) -> Result<Config, ConfigError> {
    let root_package = cargo_metadata
        .root_package()
        .ok_or(ConfigError::MissingRootPackage)?;

    let buildpack_bin_targets: Vec<&Target> = root_package
        .targets
        .iter()
        .filter(|target| target.kind == vec!["bin"])
        .collect();

    match buildpack_bin_targets.as_slice() {
        [single_target] => Ok(Config {
            buildpack_target: single_target.name.clone(),
        }),
        [] => Err(ConfigError::NoBinTargetsFound),
        bin_target_names => {
            let has_bin_target_with_root_package_name = bin_target_names
                .iter()
                .any(|target_name| target_name.name == root_package.name);

            if has_bin_target_with_root_package_name {
                Ok(Config {
                    buildpack_target: root_package.name.clone(),
                })
            } else {
                Err(ConfigError::MultipleBinTargetsFound)
            }
        }
    }
}

#[derive(thiserror::Error, Debug)]
pub enum ConfigError {
    #[error("Cargo metadata does not contain a root package")]
    MissingRootPackage,
    #[error("No binary targets could be found in Cargo metadata")]
    NoBinTargetsFound,
    #[error("Multiple binary targets found in Cargo metadata")]
    MultipleBinTargetsFound,
}
