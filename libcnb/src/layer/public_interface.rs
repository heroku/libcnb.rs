use crate::build::BuildContext;
use crate::data::layer_content_metadata::{LayerContentMetadata, LayerTypes};
use crate::generic::GenericMetadata;
use crate::layer_env::LayerEnv;
use crate::Buildpack;
use serde::de::DeserializeOwned;
use serde::Serialize;
use std::path::{Path, PathBuf};

#[allow(unused_variables)]
pub trait Layer {
    type Buildpack: Buildpack;
    type Metadata: DeserializeOwned + Serialize + Clone;

    fn types(&self) -> LayerTypes;

    fn create(
        &self,
        context: &BuildContext<Self::Buildpack>,
        layer_path: &Path,
    ) -> Result<LayerResult<Self::Metadata>, <Self::Buildpack as Buildpack>::Error>;

    fn should_be_recreated(
        &self,
        context: &BuildContext<Self::Buildpack>,
        layer_data: &LayerData<Self::Metadata>,
    ) -> Result<bool, <Self::Buildpack as Buildpack>::Error> {
        Ok(false)
    }

    fn should_be_updated(
        &self,
        context: &BuildContext<Self::Buildpack>,
        layer_data: &LayerData<Self::Metadata>,
    ) -> Result<bool, <Self::Buildpack as Buildpack>::Error> {
        Ok(false)
    }

    fn update(
        &self,
        context: &BuildContext<Self::Buildpack>,
        layer_data: &LayerData<Self::Metadata>,
    ) -> Result<LayerResult<Self::Metadata>, <Self::Buildpack as Buildpack>::Error> {
        LayerResultBuilder::new(layer_data.content_metadata.metadata.clone()).build()
    }

    fn migrate_incompatible_metadata(
        &self,
        context: &BuildContext<Self::Buildpack>,
        metadata: &GenericMetadata,
    ) -> Result<MetadataMigration<Self::Metadata>, <Self::Buildpack as Buildpack>::Error> {
        Ok(MetadataMigration::RecreateLayer)
    }
}

pub enum MetadataMigration<M> {
    RecreateLayer,
    ReplaceMetadata(M),
}

pub struct LayerData<M> {
    pub name: String,
    pub path: PathBuf,
    pub env: LayerEnv,
    pub content_metadata: LayerContentMetadata<M>,
}

pub struct LayerResult<M> {
    pub metadata: M,
    pub env: Option<LayerEnv>,
}

pub struct LayerResultBuilder<M> {
    metadata: M,
    env: Option<LayerEnv>,
}

impl<M> LayerResultBuilder<M> {
    pub fn new(metadata: M) -> Self {
        Self {
            metadata,
            env: None,
        }
    }

    pub fn env(mut self, layer_env: LayerEnv) -> Self {
        self.env = Some(layer_env);
        self
    }

    pub fn build<E>(self) -> Result<LayerResult<M>, E> {
        Ok(LayerResult {
            metadata: self.metadata,
            env: self.env,
        })
    }
}
