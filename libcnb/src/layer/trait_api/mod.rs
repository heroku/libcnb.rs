// The whole API is deprecated and relies on itself for implementation. To avoid necessary warnings,
// the use of deprecated code is allowed in this module.
#![allow(deprecated)]

use crate::Buildpack;
use crate::build::BuildContext;
use crate::data::layer::LayerName;
use crate::data::layer_content_metadata::{LayerContentMetadata, LayerTypes};
use crate::generic::GenericMetadata;
use crate::layer_env::LayerEnv;
use crate::sbom::Sbom;
use serde::Serialize;
use serde::de::DeserializeOwned;
use std::collections::HashMap;
use std::path::{Path, PathBuf};

pub(crate) mod handling;
#[cfg(test)]
mod tests;

/// Represents a buildpack layer written with the libcnb framework.
///
/// Buildpack authors implement this trait to define how a layer is created/updated/removed
/// depending on its state. To use a `Layer` implementation during build, use
/// [`BuildContext::handle_layer`](crate::build::BuildContext::handle_layer).
#[allow(unused_variables)]
#[deprecated = "The Layer trait API was replaced by a struct based API. Use CachedLayerDefinition or UncachedLayerDefinition."]
pub trait Layer {
    /// The buildpack this layer is used with.
    type Buildpack: Buildpack;

    /// The metadata type for this layer. This is the data within `[metadata]` of the layer content
    /// metadata. If you wish to use raw, untyped, TOML data instead, use [`GenericMetadata`].
    ///
    /// If the layer metadata can't be parsed into this type, libcnb will call [`migrate_incompatible_metadata`](Self::migrate_incompatible_metadata)
    /// with the layer's metadata as raw TOML. This allows migration of older metadata.
    type Metadata: DeserializeOwned + Serialize + Clone;

    /// Returns the types of this layer. Will be called by libcnb whenever it needs to determine
    /// the types for this layer. This includes, but is not limited to: after create, update and
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
        &mut self,
        context: &BuildContext<Self::Buildpack>,
        layer_path: &Path,
    ) -> Result<LayerResult<Self::Metadata>, <Self::Buildpack as Buildpack>::Error>;

    /// This method will be called by libcnb when the layer already exists to determine the strategy
    /// used to deal with it. Implementations of this method can use the current layer contents and
    /// metadata to make that decision.
    ///
    /// This can be used to invalidate a layer based on metadata. For example, the layer metadata
    /// could contain a language runtime version string. If the version requested by the user is
    /// different, this method should can return [`ExistingLayerStrategy::Recreate`], causing a new
    /// language runtime version to be installed from scratch. Conversely, if the metadata matches,
    /// this method can return [`ExistingLayerStrategy::Keep`], causing the layer to stay as-is and
    /// no calls to [`crate`](Layer::create) or [`update`](Layer::update) will be made.
    ///
    /// It can also be used cause a call to [`update`](Layer::update), updating the contents of the
    /// existing layer. Installing additional application dependencies with a package manager is
    /// one common case where this strategy makes sense. Implementations need to return
    /// [`ExistingLayerStrategy::Update`] to trigger that behavior.
    ///
    ///
    /// When not implemented, the default implementation will return
    /// [`ExistingLayerStrategy::Recreate`], causing the layer to be recreated from scratch every
    /// time.
    ///
    /// # Implementation Requirements
    /// Implementations **MUST NOT** modify the file-system.
    fn existing_layer_strategy(
        &mut self,
        context: &BuildContext<Self::Buildpack>,
        layer_data: &LayerData<Self::Metadata>,
    ) -> Result<ExistingLayerStrategy, <Self::Buildpack as Buildpack>::Error> {
        Ok(ExistingLayerStrategy::Recreate)
    }

    /// Updates the layer contents and metadata based on the cached version of a previous run.
    ///
    /// `layer_path` will be a directory with the data from a previous run. This method can modify
    /// the contents freely. Layer metadata, including environment variables, is part of the return
    /// value of this function and will be written to the appropriate locations by libcnb
    /// automatically.
    ///
    /// The return value of this method is the canonical value for metadata and environment variables.
    /// If the returned [`LayerResult`] does not contain metadata or environment
    /// variables, the resulting layer will not have either. To keep the values from the cached
    /// version they must be explicitly added to the result. This can be done by reading the env
    /// data from the given [`LayerData`] value.
    ///
    /// The default implementation will copy both the previous metadata and environment and not
    /// change the layer data itself, making the default implementation a no-op.
    ///
    /// # Implementation Requirements
    /// Implementations **MUST NOT** write to any other location than `layer_path`.
    fn update(
        &mut self,
        context: &BuildContext<Self::Buildpack>,
        layer_data: &LayerData<Self::Metadata>,
    ) -> Result<LayerResult<Self::Metadata>, <Self::Buildpack as Buildpack>::Error> {
        LayerResultBuilder::new(layer_data.content_metadata.metadata.clone())
            .env(layer_data.env.clone())
            .build()
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
        &mut self,
        context: &BuildContext<Self::Buildpack>,
        metadata: &GenericMetadata,
    ) -> Result<MetadataMigration<Self::Metadata>, <Self::Buildpack as Buildpack>::Error> {
        Ok(MetadataMigration::RecreateLayer)
    }
}

/// The result of a [`Layer::existing_layer_strategy`] call.
#[derive(Eq, PartialEq, Clone, Copy, Debug)]
#[deprecated = "Part of the Layer trait API that was replaced by a struct based API."]
pub enum ExistingLayerStrategy {
    /// The existing layer should not be modified.
    Keep,
    /// The existing layer should be deleted and then recreated from scratch.
    Recreate,
    /// The existing layer contents should be updated with the [`Layer::update`] method.
    Update,
}

/// The result of a [`Layer::migrate_incompatible_metadata`] call.
#[deprecated = "Part of the Layer trait API that was replaced by a struct based API."]
pub enum MetadataMigration<M> {
    /// The layer should be recreated entirely.
    RecreateLayer,
    /// The layer's metadata should be replaced by this new value.
    ReplaceMetadata(M),
}

/// Information about an existing CNB layer.
#[deprecated = "Part of the Layer trait API that was replaced by a struct based API."]
pub struct LayerData<M> {
    pub name: LayerName,
    /// The layer's path, should not be modified outside of a [`Layer`] implementation.
    pub path: PathBuf,
    pub env: LayerEnv,
    pub content_metadata: LayerContentMetadata<M>,
}

/// The result of a function that processes layer data.
///
/// Essentially, this carries additional metadata about a layer this later persisted according
/// to the CNB spec by libcnb.
#[deprecated = "Part of the Layer trait API that was replaced by a struct based API."]
pub struct LayerResult<M> {
    pub metadata: M,
    pub env: Option<LayerEnv>,
    pub exec_d_programs: HashMap<String, PathBuf>,
    pub sboms: Vec<Sbom>,
}

/// A builder that simplifies the creation of [`LayerResult`] values.
#[deprecated = "Part of the Layer trait API that was replaced by a struct based API."]
pub struct LayerResultBuilder<M> {
    metadata: M,
    env: Option<LayerEnv>,
    exec_d_programs: HashMap<String, PathBuf>,
    sboms: Vec<Sbom>,
}

impl<M> LayerResultBuilder<M> {
    #[must_use]
    pub fn new(metadata: M) -> Self {
        Self {
            metadata,
            env: None,
            exec_d_programs: HashMap::new(),
            sboms: Vec::new(),
        }
    }

    #[must_use]
    pub fn env(mut self, layer_env: LayerEnv) -> Self {
        self.env = Some(layer_env);
        self
    }

    /// Adds an exec.d program to the layer.
    ///
    /// # Example
    ///
    /// ```compile_fail
    /// use libcnb::generic::GenericMetadata;
    /// use libcnb::layer::LayerResultBuilder;
    /// use libcnb::additional_buildpack_binary_path;
    ///
    /// LayerResultBuilder::new(GenericMetadata::default())
    ///        .exec_d_program(
    ///            "program_name_in_layer",
    ///            // This does not compile in this doctest since there is no binary target with this name.
    ///            additional_buildpack_binary_path!("binary_target_name"),
    ///        )
    ///        .build();
    /// ```
    #[must_use]
    pub fn exec_d_program(
        mut self,
        name: impl Into<String>,
        exec_d_program: impl Into<PathBuf>,
    ) -> Self {
        self.exec_d_programs
            .insert(name.into(), exec_d_program.into());
        self
    }

    /// Adds an SBOM to the layer.
    ///
    /// # Example
    ///
    /// ```no_run
    /// use libcnb::data::sbom::SbomFormat;
    /// use libcnb::generic::GenericMetadata;
    /// use libcnb::layer::LayerResultBuilder;
    /// use libcnb::sbom::Sbom;
    /// use std::path::PathBuf;
    ///
    /// # fn wrapper() -> std::io::Result<libcnb::layer::LayerResult<GenericMetadata>> {
    /// LayerResultBuilder::new(GenericMetadata::default())
    ///     .sbom(Sbom::from_path(
    ///         SbomFormat::CycloneDxJson,
    ///         PathBuf::from("/path/to/generated_sbom"),
    ///     )?)
    ///     .build()
    /// # }
    /// ```
    #[must_use]
    pub fn sbom<S: Into<Sbom>>(mut self, s: S) -> Self {
        self.sboms.push(s.into());
        self
    }

    /// Builds the final [`LayerResult`].
    ///
    /// This method returns the [`LayerResult`] wrapped in a [`Result`] even though its technically
    /// not fallible. This is done to simplify using this method in the contexts it's most often
    /// used in: a layer's [create](Layer::create) and/or
    /// [update](Layer::update) methods.
    ///
    /// See [`build_unwrapped`](Self::build_unwrapped) for an unwrapped version of this method.
    pub fn build<E>(self) -> Result<LayerResult<M>, E> {
        Ok(self.build_unwrapped())
    }

    #[must_use]
    pub fn build_unwrapped(self) -> LayerResult<M> {
        LayerResult {
            metadata: self.metadata,
            env: self.env,
            exec_d_programs: self.exec_d_programs,
            sboms: self.sboms,
        }
    }
}
