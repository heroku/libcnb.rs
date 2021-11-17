use crate::build::BuildContext;
use crate::data::layer_content_metadata::{LayerContentMetadata, LayerTypes};
use crate::generic::GenericMetadata;
use crate::layer_env::LayerEnv;
use crate::Buildpack;
use serde::de::DeserializeOwned;
use serde::Serialize;
use std::path::{Path, PathBuf};

/// Represents a buildpack layer written with the libcnb framework.
///
/// Buildpack authors implement this trait to define how a layer is created/updated/removed
/// depending on its state. To use a `Layer` implementation during build, use
/// [`BuildContext::handle_layer`](crate::build::BuildContext::handle_layer).
#[allow(unused_variables)]
pub trait Layer {
    /// The buildpack this layer is used with.
    type Buildpack: Buildpack;

    /// The metadata type for this layer. This is the data within `[metadata]` of the layer content
    /// metadata. If you wish to use raw, untyped, TOML data instead, use [`GenericMetadata`](crate::generic::GenericMetadata).
    ///
    /// If the layer metadata cannot be parsed into this type, libcnb will call [`migrate_incompatible_metadata`](Self::migrate_incompatible_metadata)
    /// with the layer's metadata as raw TOML. This allows migration of older metadata.
    type Metadata: DeserializeOwned + Serialize + Clone;

    /// Returns the types of this layer. Will be called by libcnb whenever it needs to determine
    /// the types for this layer. This includes, but is not limited to, after create, update and
    /// when the layer is not modified at all.
    ///
    /// # Implementation Requirements
    /// Implementations **MUST** be pure. This includes that they **MUST NOT** side-effect,
    /// including writing to stdout/stderr or the file system.
    fn types(&self) -> LayerTypes;

    /// Creates the layer from scratch.
    ///
    /// `layer_path` will be an empty directory where this method can write files to. Layer
    /// metadata, including environment variables, is part of the return value of this function and
    /// will be written to the appropriate locations by libcnb automatically.
    ///
    /// # Implementation Requirements
    /// Implementations **MUST NOT** write to any other location than `layer_path`.
    fn create(
        &self,
        context: &BuildContext<Self::Buildpack>,
        layer_path: &Path,
    ) -> Result<LayerResult<Self::Metadata>, <Self::Buildpack as Buildpack>::Error>;

    /// This method will be called by libcnb to determine if, based on the current state, the layer
    /// needs to be recreated. If this method returns true, libcnb will delete the layer and call
    /// create as if the layer didn't exist.
    ///
    /// This can be used to invalidate a layer based on metadata. For example, the layer metadata
    /// could contain a language runtime version string. If the version requested by the user is
    /// different, this method should return true, causing the new language runtime version to be
    /// installed.
    ///
    /// When not implemented, the layer will never be recreated.
    ///
    /// # Implementation Requirements
    /// Implementations **MUST** be read-only. They **MUST NOT** modify the file-system or write
    /// anything to stdout/stdout or any other stream.
    fn should_be_recreated(
        &self,
        context: &BuildContext<Self::Buildpack>,
        layer_data: &LayerData<Self::Metadata>,
    ) -> Result<bool, <Self::Buildpack as Buildpack>::Error> {
        Ok(false)
    }

    /// This method will be called by libcnb to determine if, based on the current state, the layer
    /// needs to be updated. A layer will only be updated if it was restored from the cache and this
    /// method returns `true`.
    ///
    /// If the layer was restored from cache and this method returns `false`, neither
    /// [`create`](Self::create) nor [`update`](Self::update) will be called by libcnb. In this
    /// case, the layer will stay unmodified, but libcnb will ensure its types match the ones
    /// returned from the [`types`](Self::types) method.
    ///
    /// When not implemented, the layer will never be updated.
    ///
    /// # Implementation Requirements
    /// Implementations **MUST** be read-only. They **MUST NOT** modify the file-system or write
    /// anything to stdout/stdout or any other stream.
    fn should_be_updated(
        &self,
        context: &BuildContext<Self::Buildpack>,
        layer_data: &LayerData<Self::Metadata>,
    ) -> Result<bool, <Self::Buildpack as Buildpack>::Error> {
        Ok(false)
    }

    /// Updates the layer contents and metadata based on the cached version of a previous run.
    ///
    /// `layer_path` will be a directory with the data from a previous run. This method can modify
    /// the contents freely. Layer metadata, including environment variables, is part of the return
    /// value of this function and will be written to the appropriate locations by libcnb
    /// automatically.
    ///
    /// The return value of this method is the canonical value for metadata and environment variables.
    /// If the returned [`LayerResult`](LayerResult) does not contain metadata or environment
    /// variables, the resulting layer will not have either. To keep the values from the cached
    /// version you need to explicitly add them to the result. This can be done by reading that
    /// data from the given [`LayerData`](LayerData) value.
    ///
    /// # Implementation Requirements
    /// Implementations **MUST NOT** write to any other location than `layer_path`.
    fn update(
        &self,
        context: &BuildContext<Self::Buildpack>,
        layer_data: &LayerData<Self::Metadata>,
    ) -> Result<LayerResult<Self::Metadata>, <Self::Buildpack as Buildpack>::Error> {
        LayerResultBuilder::new(layer_data.content_metadata.metadata.clone()).build()
    }

    /// This method will be called by libcnb when parsing of the layer's metadata into
    /// [`Self::Metadata`] failed. The result of this method call determines which strategy libcnb
    /// should use to continue.
    ///
    /// The simplest strategy, [`MetadataMigration::RecreateLayer`] will delete the layer and
    /// recreate it from scratch. This is also the default implementation.
    ///
    /// In some cases, a layer might be able to migrate metadata from an older version to a new
    /// structure. For this, [`MetadataMigration::ReplaceMetadata`] can be used. Implementations can
    /// use the raw TOML metadata passed as `metadata` to the method and the contents of the layer
    /// to construct a new value for [`Self::Metadata`].
    ///
    /// # Implementation Requirements
    /// Implementations **MUST** be read-only. They **MUST NOT** modify the file-system or write
    /// anything to stdout/stdout or any other stream.
    fn migrate_incompatible_metadata(
        &self,
        context: &BuildContext<Self::Buildpack>,
        metadata: &GenericMetadata,
    ) -> Result<MetadataMigration<Self::Metadata>, <Self::Buildpack as Buildpack>::Error> {
        Ok(MetadataMigration::RecreateLayer)
    }
}

/// The result of a [`Layer::migrate_incompatible_metadata`] call.
pub enum MetadataMigration<M> {
    /// The layer should be recreated entirely.
    RecreateLayer,
    /// The layer's metadata should be replaced by this new value.
    ReplaceMetadata(M),
}

/// Information about an existing CNB layer.
pub struct LayerData<M> {
    pub name: String,
    /// The layer's path, should not be modified outside of a [`Layer`] implementation.
    pub path: PathBuf,
    pub env: LayerEnv,
    pub content_metadata: LayerContentMetadata<M>,
}

/// The result of a function that processes layer data.
///
/// Essentially, this carries additional metadata about a layer this later persisted according
/// to the CNB spec by libcnb.
pub struct LayerResult<M> {
    pub metadata: M,
    pub env: Option<LayerEnv>,
}

/// A builder that simplifies the creation of [`LayerResult`] values.
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
