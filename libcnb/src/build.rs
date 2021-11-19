//! Provides build phase specific types and helpers.

use std::path::PathBuf;

use crate::buildpack::Buildpack;
use crate::data::store::Store;
use crate::layer::{HandleLayerErrorOrBuildpackError, Layer, LayerData};

use crate::data::buildpack::StackId;
use crate::data::{buildpack::BuildpackToml, buildpack_plan::BuildpackPlan, launch::Launch};

/// Context for the build phase execution.
pub struct BuildContext<B: Buildpack + ?Sized> {
    pub layers_dir: PathBuf,
    pub app_dir: PathBuf,
    pub buildpack_dir: PathBuf,
    pub stack_id: StackId,
    pub platform: B::Platform,
    pub buildpack_plan: BuildpackPlan,
    pub buildpack_descriptor: BuildpackToml<B::Metadata>,
}

impl<B: Buildpack + ?Sized> BuildContext<B> {
    /// Handles the given [`Layer`] implementation in this context.
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
    /// # use libcnb::data::layer_content_metadata::LayerTypes;
    /// # use libcnb::detect::{DetectContext, DetectResult};
    /// # use libcnb::generic::{GenericError, GenericMetadata, GenericPlatform};
    /// # use libcnb::layer::{Layer, LayerResult, LayerResultBuilder};
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
    ///         let example_layer = context.handle_layer("example-layer", ExampleLayer)?;
    ///
    ///         println!(
    ///             "Monologue from layer metadata: {}",
    ///             &example_layer.content_metadata.metadata.monologue
    ///         );
    ///
    ///         Ok(BuildResultBuilder::new().build())
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
    ///         &self,
    ///         context: &BuildContext<Self::Buildpack>,
    ///         layer_path: &Path,
    ///     ) -> Result<LayerResult<Self::Metadata>, <Self::Buildpack as Buildpack>::Error> {
    ///         LayerResultBuilder::new(ExampleLayerMetadata {
    ///             monologue: String::from("I've seen things you people wouldn't believe... Attack ships on fire off the shoulder of Orion..." )
    ///         }).build()
    ///     }
    /// }
    /// ```
    pub fn handle_layer<L: Layer<Buildpack = B>>(
        &self,
        name: impl AsRef<str>,
        layer: L,
    ) -> crate::Result<LayerData<L::Metadata>, B::Error> {
        crate::layer::handle_layer(self, name.as_ref(), layer).map_err(|error| match error {
            HandleLayerErrorOrBuildpackError::HandleLayerError(e) => {
                crate::Error::HandleLayerError(e)
            }
            HandleLayerErrorOrBuildpackError::BuildpackError(e) => crate::Error::BuildpackError(e),
        })
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
pub struct BuildResult(pub(crate) InnerBuildResult);

#[derive(Debug)]
pub(crate) enum InnerBuildResult {
    Pass {
        launch: Option<Launch>,
        store: Option<Store>,
    },
}

/// Constructs [`BuildResult`] values.
///
/// # Examples:
/// ```
/// use libcnb::build::BuildResultBuilder;
/// use libcnb_data::launch::{Launch, Process};
///
/// let simple = BuildResultBuilder::new().build();
///
/// let with_launch = BuildResultBuilder::new()
///    .launch(Launch::new().process(Process::new("type", "command", vec!["-v"], false, false).unwrap()))
///    .build();
/// ```
pub struct BuildResultBuilder {
    launch: Option<Launch>,
    store: Option<Store>,
}

impl BuildResultBuilder {
    pub fn new() -> Self {
        Self {
            launch: None,
            store: None,
        }
    }
}

impl BuildResultBuilder {
    pub fn build(self) -> BuildResult {
        BuildResult(InnerBuildResult::Pass {
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

impl Default for BuildResultBuilder {
    fn default() -> Self {
        Self::new()
    }
}
