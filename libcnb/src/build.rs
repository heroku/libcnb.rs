use std::{fs, path::PathBuf};

use serde::de::DeserializeOwned;
use serde::Serialize;

use crate::buildpack::Buildpack;
use crate::data::store::Store;
use crate::{
    data::{
        buildpack::BuildpackToml, buildpack_plan::BuildpackPlan, launch::Launch,
        layer_content_metadata::LayerContentMetadata,
    },
    toml_file::{read_toml_file, write_toml_file, TomlFileError},
};

/// Context for the build phase execution.
pub struct BuildContext<B: Buildpack + ?Sized> {
    pub layers_dir: PathBuf,
    pub app_dir: PathBuf,
    pub buildpack_dir: PathBuf,
    pub stack_id: String,
    pub platform: B::Platform,
    pub buildpack_plan: BuildpackPlan,
    pub buildpack_descriptor: BuildpackToml<B::Metadata>,
}

/// Describes the outcome of the build phase. Besides indicating success or failure, it also
/// contains build phase output such as launch and/or store metadata.
///
/// To construct values of this type, use a [`BuildOutcomeBuilder`].
#[derive(Debug)]
pub struct BuildOutcome(pub(crate) InnerBuildOutcome);

#[derive(Debug)]
pub(crate) enum InnerBuildOutcome {
    Fail,
    Pass {
        launch: Option<Launch>,
        store: Option<Store>,
    },
}

/// Constructs [`BuildOutcome`] values.
///
/// # Examples:
/// ```
/// use libcnb::build::BuildOutcomeBuilder;
/// use libcnb_data::launch::{Launch, Process};
///
/// let simple_success = BuildOutcomeBuilder::success().build();
/// let simple_failure = BuildOutcomeBuilder::fail().build();
///
/// let with_launch = BuildOutcomeBuilder::success()
///    .launch(Launch::new().process(Process::new("type", "command", vec!["-v"], false, false).unwrap()))
///    .build();
/// ```
pub struct BuildOutcomeBuilder;

impl BuildOutcomeBuilder {
    pub fn success() -> SuccessBuildOutcomeBuilder {
        SuccessBuildOutcomeBuilder {
            launch: None,
            store: None,
        }
    }

    pub fn fail() -> FailBuildOutcomeBuilder {
        FailBuildOutcomeBuilder {}
    }
}

pub struct SuccessBuildOutcomeBuilder {
    launch: Option<Launch>,
    store: Option<Store>,
}

impl SuccessBuildOutcomeBuilder {
    pub fn build(self) -> BuildOutcome {
        BuildOutcome(InnerBuildOutcome::Pass {
            launch: self.launch,
            store: self.store,
        })
    }

    pub fn launch(mut self, launch: Launch) -> Self {
        self.launch = Some(launch);
        self
    }

    pub fn store(mut self, store: Store) -> Self {
        self.store = Some(store);
        self
    }
}

pub struct FailBuildOutcomeBuilder;

impl FailBuildOutcomeBuilder {
    #[allow(clippy::unused_self)]
    pub fn build(self) -> BuildOutcome {
        BuildOutcome(InnerBuildOutcome::Fail)
    }
}

impl<B: Buildpack> BuildContext<B> {
    pub fn layer_path(&self, layer_name: impl AsRef<str>) -> PathBuf {
        self.layers_dir.join(layer_name.as_ref())
    }

    pub fn layer_content_metadata_path(&self, layer_name: impl AsRef<str>) -> PathBuf {
        self.layers_dir
            .join(format!("{}.toml", layer_name.as_ref()))
    }

    pub fn read_layer_content_metadata<M: DeserializeOwned>(
        &self,
        layer_name: impl AsRef<str>,
    ) -> Result<Option<LayerContentMetadata<M>>, TomlFileError> {
        let path = self.layer_content_metadata_path(layer_name);

        if path.exists() {
            read_toml_file(path).map(Some)
        } else {
            Ok(None)
        }
    }

    pub fn write_layer_content_metadata<M: Serialize>(
        &self,
        layer_name: impl AsRef<str>,
        layer_content_metadata: &LayerContentMetadata<M>,
    ) -> Result<(), TomlFileError> {
        write_toml_file(
            layer_content_metadata,
            self.layer_content_metadata_path(layer_name),
        )
    }

    pub fn delete_layer(&self, layer_name: impl AsRef<str>) -> Result<(), std::io::Error> {
        // Do not fail if the metadata file does not exist
        match fs::remove_file(self.layer_content_metadata_path(&layer_name)) {
            Err(io_error) => match io_error.kind() {
                std::io::ErrorKind::NotFound => Ok(()),
                _ => Err(io_error),
            },
            Ok(_) => Ok(()),
        }?;

        match fs::remove_dir_all(self.layer_path(&layer_name)) {
            Err(io_error) => match io_error.kind() {
                std::io::ErrorKind::NotFound => Ok(()),
                _ => Err(io_error),
            },
            Ok(_) => Ok(()),
        }?;

        Ok(())
    }

    pub fn read_layer<M: DeserializeOwned>(
        &self,
        layer_name: impl AsRef<str>,
    ) -> Result<Option<(PathBuf, LayerContentMetadata<M>)>, TomlFileError> {
        let layer_path = self.layer_path(&layer_name);

        self.read_layer_content_metadata(&layer_name)
            .map(|maybe_content_layer_metadata| {
                maybe_content_layer_metadata.and_then(
                    |layer_content_metadata: LayerContentMetadata<M>| {
                        if layer_path.exists() {
                            Some((layer_path, layer_content_metadata))
                        } else {
                            None
                        }
                    },
                )
            })
    }

    pub fn layer_exists(&self, layer_name: impl AsRef<str>) -> bool {
        let layer_path = self.layer_path(&layer_name);
        let content_metadata_path = self.layer_content_metadata_path(&layer_name);
        layer_path.exists() && content_metadata_path.exists()
    }
}
