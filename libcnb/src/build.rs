//! Provides build phase specific types and helpers.

use crate::buildpack::Buildpack;
use crate::data::layer::LayerName;
use crate::data::store::Store;
use crate::data::{
    buildpack::ComponentBuildpackDescriptor, buildpack_plan::BuildpackPlan, launch::Launch,
};
use crate::layer::handling::LayerErrorOrBuildpackError;
use crate::layer::{
    CachedLayerDefinition, InspectExistingAction, IntoAction, InvalidMetadataAction, LayerRef,
    UncachedLayerDefinition,
};
use crate::sbom::Sbom;
use crate::target::ContextTarget;
use libcnb_data::generic::GenericMetadata;
use libcnb_data::layer_content_metadata::LayerTypes;
use serde::de::DeserializeOwned;
use serde::Serialize;
use std::borrow::Borrow;
use std::path::PathBuf;

/// Context for the build phase execution.
pub struct BuildContext<B: Buildpack + ?Sized> {
    pub layers_dir: PathBuf,
    pub app_dir: PathBuf,
    pub buildpack_dir: PathBuf,
    pub target: ContextTarget,
    pub platform: B::Platform,
    pub buildpack_plan: BuildpackPlan,
    pub buildpack_descriptor: ComponentBuildpackDescriptor<B::Metadata>,
    pub store: Option<Store>,
}

impl<B: Buildpack + ?Sized> BuildContext<B> {
    /// Handles the given [`LayerRef`] implementation in this context.
    ///
    /// It will ensure that the layer with the given name is created and/or updated accordingly and
    /// handles all errors that can occur during the process. After this method has executed, the
    /// layer will exist on disk or an error has been returned by this method.
    ///
    /// Use the returned [`LayerData`] to access the layers metadata and environment variables for
    /// subsequent logic or layers.
    ///
    /// # Example:
    /// ```
    /// # use libcnb::build::{BuildContext, BuildResult, BuildResultBuilder};
    /// # use libcnb::data::layer_name;
    /// # use libcnb::data::layer_content_metadata::LayerTypes;
    /// # use libcnb::detect::{DetectContext, DetectResult};
    /// # use libcnb::generic::{GenericError, GenericMetadata, GenericPlatform};
    /// # use libcnb::layer::{Layer, LayerRef, LayerResultBuilder, LayerResult};
    /// # use libcnb::Buildpack;
    /// # use serde::Deserialize;
    /// # use serde::Serialize;
    /// # use std::path::Path;
    /// #
    /// struct ExampleBuildpack;
    ///
    /// impl Buildpack for ExampleBuildpack {
    /// #   type Platform = GenericPlatform;
    /// #   type Metadata = GenericMetadata;
    /// #   type Error = GenericError;
    /// #
    /// #    fn detect(&self, context: DetectContext<Self>) -> libcnb::Result<DetectResult, Self::Error> {
    /// #        unimplemented!()
    /// #    }
    /// #
    ///     fn build(&self, context: BuildContext<Self>) -> libcnb::Result<BuildResult, Self::Error> {
    ///         let example_layer = context.handle_layer(layer_name!("example-layer"), ExampleLayer)?;
    ///
    ///         println!(
    ///             "Monologue from layer metadata: {}",
    ///             &example_layer.content_metadata.metadata.monologue
    ///         );
    ///
    ///         BuildResultBuilder::new().build()
    ///     }
    /// }
    ///
    /// struct ExampleLayer;
    ///
    /// # #[derive(Deserialize, Serialize, Clone)]
    /// # struct ExampleLayerMetadata {
    /// #    monologue: String,
    /// # }
    /// #
    /// impl Layer for ExampleLayer {
    /// # type Buildpack = ExampleBuildpack;
    /// #   type Metadata = ExampleLayerMetadata;
    /// #
    /// #    fn types(&self) -> LayerTypes {
    /// #        unimplemented!()
    /// #    }
    /// #
    ///     fn create(
    ///         &mut self,
    ///         context: &BuildContext<Self::Buildpack>,
    ///         layer_path: &Path,
    ///     ) -> Result<LayerResult<Self::Metadata>, <Self::Buildpack as Buildpack>::Error> {
    ///         LayerResultBuilder::new(ExampleLayerMetadata {
    ///             monologue: String::from("I've seen things you people wouldn't believe... Attack ships on fire off the shoulder of Orion..." )
    ///         }).build()
    ///     }
    /// }
    /// ```
    #[deprecated = "The Layer trait API was replaced by LayerDefinitions. Use `cached_layer` and `uncached_layer`."]
    #[allow(deprecated)]
    pub fn handle_layer<L: crate::layer::Layer<Buildpack = B>>(
        &self,
        layer_name: LayerName,
        layer: L,
    ) -> crate::Result<crate::layer::LayerData<L::Metadata>, B::Error> {
        crate::layer::trait_api::handling::handle_layer(self, layer_name, layer).map_err(|error| {
            match error {
                LayerErrorOrBuildpackError::LayerError(e) => crate::Error::LayerError(e),
                LayerErrorOrBuildpackError::BuildpackError(e) => crate::Error::BuildpackError(e),
            }
        })
    }

    pub fn uncached_layer(
        &self,
        layer_name: LayerName,
        layer_definition: impl Borrow<UncachedLayerDefinition>,
    ) -> crate::Result<LayerRef<B, (), ()>, B::Error> {
        let layer_definition = layer_definition.borrow();

        crate::layer::execute(
            LayerTypes {
                launch: layer_definition.launch,
                build: layer_definition.build,
                cache: false,
            },
            &|_| InvalidMetadataAction::DeleteLayer,
            &|_: &GenericMetadata, _| InspectExistingAction::Delete,
            layer_name,
            &self.layers_dir,
        )
    }

    pub fn cached_layer<'a, M, X, Y, O, I>(
        &self,
        layer_name: LayerName,
        layer_definition: impl Borrow<CachedLayerDefinition<'a, M, O, I>>,
    ) -> crate::Result<LayerRef<B, X, Y>, B::Error>
    where
        M: 'a + Serialize + DeserializeOwned,
        O: 'a + IntoAction<InvalidMetadataAction<M>, X, B::Error>,
        I: 'a + IntoAction<InspectExistingAction, Y, B::Error>,
    {
        let layer_definition = layer_definition.borrow();

        crate::layer::execute(
            LayerTypes {
                launch: layer_definition.launch,
                build: layer_definition.build,
                cache: true,
            },
            layer_definition.invalid_metadata,
            layer_definition.inspect_existing,
            layer_name,
            &self.layers_dir,
        )
    }
}

/// Describes the result of the build phase.
///
/// In contrast to `DetectResult`, it always signals a successful build. To fail the build phase,
/// return a failed [`crate::Result`] from the build function.
///
/// It contains build phase output such as launch and/or store metadata which will be subsequently
/// handled by libcnb.
///
/// To construct values of this type, use a [`BuildResultBuilder`].
#[derive(Debug)]
#[must_use]
pub struct BuildResult(pub(crate) InnerBuildResult);

#[derive(Debug)]
pub(crate) enum InnerBuildResult {
    Pass {
        launch: Option<Launch>,
        store: Option<Store>,
        build_sboms: Vec<Sbom>,
        launch_sboms: Vec<Sbom>,
    },
}

/// Constructs [`BuildResult`] values.
///
/// # Examples:
/// ```
/// use libcnb::build::{BuildResult, BuildResultBuilder};
/// use libcnb::data::launch::LaunchBuilder;
/// use libcnb::data::launch::ProcessBuilder;
/// use libcnb::data::process_type;
///
/// let simple: Result<BuildResult, ()> = BuildResultBuilder::new().build();
///
/// let with_launch: Result<BuildResult, ()> = BuildResultBuilder::new()
///     .launch(
///         LaunchBuilder::new()
///             .process(
///                 ProcessBuilder::new(process_type!("type"), ["command"])
///                     .arg("-v")
///                     .build(),
///             )
///             .build(),
///     )
///     .build();
/// ```
#[derive(Default)]
#[must_use]
pub struct BuildResultBuilder {
    launch: Option<Launch>,
    store: Option<Store>,
    build_sboms: Vec<Sbom>,
    launch_sboms: Vec<Sbom>,
}

impl BuildResultBuilder {
    pub fn new() -> Self {
        Self::default()
    }

    /// Builds the final [`BuildResult`].
    ///
    /// This method returns the [`BuildResult`] wrapped in a [`Result`] even though its technically
    /// not fallible. This is done to simplify using this method in the context it's most often used
    /// in: a buildpack's [build method](crate::Buildpack::build).
    ///
    /// See [`build_unwrapped`](Self::build_unwrapped) for an unwrapped version of this method.
    pub fn build<E>(self) -> Result<BuildResult, E> {
        Ok(self.build_unwrapped())
    }

    pub fn build_unwrapped(self) -> BuildResult {
        BuildResult(InnerBuildResult::Pass {
            launch: self.launch,
            store: self.store,
            build_sboms: self.build_sboms,
            launch_sboms: self.launch_sboms,
        })
    }

    pub fn launch(mut self, launch: Launch) -> Self {
        self.launch = Some(launch);
        self
    }

    pub fn store<S: Into<Store>>(mut self, store: S) -> Self {
        self.store = Some(store.into());
        self
    }

    /// Adds a build SBOM to the build result.
    ///
    /// Entries in this SBOM represent materials in the build container for auditing purposes.
    /// This function can be called multiple times to add SBOMs in different formats.
    ///
    /// Please note that these SBOMs are not added to the resulting image, they are purely for
    /// auditing the build container.
    pub fn build_sbom(mut self, sbom: Sbom) -> Self {
        self.build_sboms.push(sbom);
        self
    }

    /// Adds a launch SBOM to the build result.
    ///
    /// Entries in this SBOM represent materials in the launch image for auditing purposes.
    /// This function can be called multiple times to add SBOMs in different formats.
    pub fn launch_sbom(mut self, sbom: Sbom) -> Self {
        self.launch_sboms.push(sbom);
        self
    }
}
