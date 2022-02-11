use cargo_metadata::{Metadata, Target};
use serde;
use serde::Deserialize;

#[derive(Debug, Deserialize)]
pub(crate) struct Config {
    #[serde(rename = "buildpack-target")]
    pub buildpack_target: String,
}

pub(crate) fn config_from_metadata(cargo_metadata: &Metadata) -> Result<Config, ConfigError> {
    let root_package = cargo_metadata
        .root_package()
        .ok_or(ConfigError::MissingRootPackage)?;

    let explicit_config = root_package
        .metadata
        .as_object()
        .unwrap_or(&serde_json::Map::default())
        .get("libcnb")
        .map(|libcnb_object| {
            serde_json::from_value::<Config>(libcnb_object.clone())
                .map_err(ConfigError::ConfigJsonError)
        })
        .transpose();

    explicit_config.and_then(|maybe_config| {
        maybe_config.map_or_else(
            || {
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
            },
            Ok,
        )
    })
}

#[derive(Debug)]
pub enum ConfigError {
    MissingRootPackage,
    ConfigJsonError(serde_json::Error),
    NoBinTargetsFound,
    MultipleBinTargetsFound,
}
