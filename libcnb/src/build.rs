//! Provides build phase specific types and helpers.

use crate::buildpack::Buildpack;
use crate::data::layer::LayerName;
use crate::data::store::Store;
use crate::data::{
    buildpack::ComponentBuildpackDescriptor, buildpack_plan::BuildpackPlan, launch::Launch,
};
use crate::layer::trait_api::handling::LayerErrorOrBuildpackError;
use crate::layer::{
    CachedLayerDefinition, IntoAction, InvalidMetadataAction, LayerRef, RestoredLayerAction,
    UncachedLayerDefinition,
};
use crate::sbom::Sbom;
use crate::Target;
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
    pub target: Target,
    pub platform: B::Platform,
    pub buildpack_plan: BuildpackPlan,
    pub buildpack_descriptor: ComponentBuildpackDescriptor<B::Metadata>,
    pub store: Option<Store>,
}

impl<B: Buildpack + ?Sized> BuildContext<B> {
    /// Handles the given [`crate::layer::Layer`] implementation in this context.
    ///
    /// It will ensure that the layer with the given name is created and/or updated accordingly and
    /// handles all errors that can occur during the process. After this method has executed, the
    /// layer will exist on disk or an error has been returned by this method.
    ///
    /// Use the returned [`crate::layer::LayerData`] to access the layers metadata and environment variables for
    /// subsequent logic or layers.
    ///
    /// # Example:
    /// ```
    /// # use libcnb::build::{BuildContext, BuildResult, BuildResultBuilder};
    /// # use libcnb::data::layer_name;
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
    #[deprecated = "The Layer trait API was replaced by a struct based API. Use `cached_layer` and `uncached_layer`."]
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

    /// Creates a cached layer, potentially re-using a previously cached version.
    ///
    /// Buildpack code uses this function to create a cached layer and will get back a reference to
    /// the layer directory on disk. Intricacies of the CNB spec are automatically handled such as
    /// the maintenance of TOML files. Buildpack code can also specify a callback for cached layer
    /// invalidation.
    ///
    /// Users of this function pass in a [`CachedLayerDefinition`] that describes the desired layer
    /// and the returned [`LayerRef`] can then be used to modify the layer like any other path. This
    /// allows users to be flexible in how and when the layer is modified and to abstract layer
    /// creation away if necessary.
    ///
    /// See [`IntoAction`] for details which values can be returned from the
    /// `invalid_metadata_action` and `restored_layer_action` functions.
    ///
    /// # Basic Example
    /// ```rust
    /// # use libcnb::build::{BuildContext, BuildResult, BuildResultBuilder};
    /// # use libcnb::detect::{DetectContext, DetectResult};
    /// # use libcnb::generic::GenericPlatform;
    /// # use libcnb::layer::{
    /// #     CachedLayerDefinition, RestoredLayerAction, InvalidMetadataAction, LayerState,
    /// # };
    /// # use libcnb::layer_env::{LayerEnv, ModificationBehavior, Scope};
    /// # use libcnb::Buildpack;
    /// # use libcnb_data::generic::GenericMetadata;
    /// # use libcnb_data::layer_name;
    /// # use std::fs;
    /// #
    /// # struct ExampleBuildpack;
    /// #
    /// # #[derive(Debug)]
    /// # enum ExampleBuildpackError {
    /// #     WriteDataError(std::io::Error),
    /// # }
    /// #
    /// # impl Buildpack for ExampleBuildpack {
    /// #    type Platform = GenericPlatform;
    /// #    type Metadata = GenericMetadata;
    /// #    type Error = ExampleBuildpackError;
    /// #
    /// #    fn detect(&self, context: DetectContext<Self>) -> libcnb::Result<DetectResult, Self::Error> {
    /// #        unimplemented!()
    /// #    }
    /// #
    /// #    fn build(&self, context: BuildContext<Self>) -> libcnb::Result<BuildResult, Self::Error> {
    /// let layer_ref = context.cached_layer(
    ///     layer_name!("example_layer"),
    ///     CachedLayerDefinition {
    ///         build: false,
    ///         launch: false,
    ///         // Will be called if a cached version of the layer was found, but the metadata
    ///         // could not be parsed. In this example, we instruct libcnb to always delete the
    ///         // existing layer in such a case. But we can implement any logic here if we want.
    ///         invalid_metadata_action: &|_| InvalidMetadataAction::DeleteLayer,
    ///         // Will be called if a cached version of the layer was found. This allows us to
    ///         // inspect the contents and metadata to decide if we want to keep the existing
    ///         // layer or let libcnb delete the existing layer and create a new one for us.
    ///         // This is libcnb's method to implement cache invalidations for layers.
    ///         restored_layer_action: &|_: &GenericMetadata, _| RestoredLayerAction::KeepLayer,
    ///     },
    /// )?;
    ///
    /// // At this point, a layer exists on disk. It might contain cached data or might be empty.
    /// // Since we need to conditionally work with the layer contents based on its state, we can
    /// // inspect the `state` field of the layer reference to get detailed information about
    /// // the current layer contents and the cause(s) for the state.
    /// //
    /// // In the majority of cases, we don't need more details beyond if it's empty or not and can
    /// // ignore the details. This is what we do in this example. See the later example for a more
    /// // complex situation.
    /// match layer_ref.state {
    ///     LayerState::Empty { .. } => {
    ///         println!("Creating new example layer!");
    ///
    ///         // Modify the layer contents with regular Rust functions:
    ///         fs::write(
    ///             layer_ref.path().join("data.txt"),
    ///             "Here is some example data",
    ///         )
    ///         .map_err(ExampleBuildpackError::WriteDataError)?;
    ///
    ///         // Use functions on LayerRef for common CNB specific layer modifications:
    ///         layer_ref.write_env(LayerEnv::new().chainable_insert(
    ///             Scope::All,
    ///             ModificationBehavior::Append,
    ///             "PLANET",
    ///             "LV-246",
    ///         ))?;
    ///     }
    ///     LayerState::Restored { .. } => {
    ///         println!("Reusing example layer from previous run!");
    ///     }
    /// }
    /// #
    /// #        BuildResultBuilder::new().build()
    /// #    }
    /// # }
    /// #
    /// # impl From<ExampleBuildpackError> for libcnb::Error<ExampleBuildpackError> {
    /// #    fn from(value: ExampleBuildpackError) -> Self {
    /// #        Self::BuildpackError(value)
    /// #    }
    /// # }
    /// ```
    ///
    /// # More complex example
    /// ```rust
    /// # use libcnb::build::{BuildContext, BuildResult, BuildResultBuilder};
    /// # use libcnb::detect::{DetectContext, DetectResult};
    /// # use libcnb::generic::GenericPlatform;
    /// # use libcnb::layer::{
    /// #     CachedLayerDefinition, EmptyLayerCause, RestoredLayerAction, InvalidMetadataAction,
    /// #     LayerState,
    /// # };
    /// # use libcnb::Buildpack;
    /// # use libcnb_data::generic::GenericMetadata;
    /// # use libcnb_data::layer_name;
    /// # use serde::{Deserialize, Serialize};
    /// # use std::fs;
    /// #
    /// # struct ExampleBuildpack;
    /// #
    /// # #[derive(Debug)]
    /// # enum ExampleBuildpackError {
    /// #     UnexpectedIoError(std::io::Error),
    /// # }
    /// #
    /// #[derive(Deserialize, Serialize)]
    /// struct ExampleLayerMetadata {
    ///     lang_runtime_version: String,
    /// }
    ///
    /// enum CustomCause {
    ///     Ok,
    ///     LegacyVersion,
    ///     HasBrokenModule,
    ///     MissingModulesFile,
    /// }
    ///
    /// # impl Buildpack for ExampleBuildpack {
    /// #     type Platform = GenericPlatform;
    /// #     type Metadata = GenericMetadata;
    /// #     type Error = ExampleBuildpackError;
    /// #
    /// #     fn detect(&self, _: DetectContext<Self>) -> libcnb::Result<DetectResult, Self::Error> {
    /// #         unimplemented!()
    /// #     }
    /// #
    /// fn build(&self, context: BuildContext<Self>) -> libcnb::Result<BuildResult, Self::Error> {
    ///     let layer_ref = context.cached_layer(
    ///         layer_name!("example_layer"),
    ///         CachedLayerDefinition {
    ///             build: false,
    ///             launch: false,
    ///             invalid_metadata_action: &|_| InvalidMetadataAction::DeleteLayer,
    ///             restored_layer_action: &|metadata: &ExampleLayerMetadata, layer_dir| {
    ///                 if metadata.lang_runtime_version.starts_with("0.") {
    ///                     // The return value for restored_layer_action can be anything with an
    ///                     // IntoAction implementation. libcnb provides built-in implementations
    ///                     // for raw RestoredLayerAction/InvalidMetadataAction values, tuples of
    ///                     // actions with a cause value (of any type) plus variants that are wrapped
    ///                     // in a Result. See IntoAction for details.
    ///                     Ok((
    ///                         RestoredLayerAction::DeleteLayer,
    ///                         CustomCause::LegacyVersion,
    ///                     ))
    ///                 } else {
    ///                     let file_path = layer_dir.join("modules.txt");
    ///
    ///                     if file_path.is_file() {
    ///                         // This is a fallible operation where an unexpected IO error occurs
    ///                         // during operation. In this example, we chose not to map it to
    ///                         // a layer action but let it automatically "bubble up". This error will
    ///                         // end up in the regular libcnb buildpack on_error.
    ///                         let file_contents = fs::read_to_string(&file_path)
    ///                             .map_err(ExampleBuildpackError::UnexpectedIoError)?;
    ///
    ///                         if file_contents == "known-broken-0.1c" {
    ///                             Ok((
    ///                                 RestoredLayerAction::DeleteLayer,
    ///                                 CustomCause::HasBrokenModule,
    ///                             ))
    ///                         } else {
    ///                             Ok((RestoredLayerAction::KeepLayer, CustomCause::Ok))
    ///                         }
    ///                     } else {
    ///                         Ok((
    ///                             RestoredLayerAction::DeleteLayer,
    ///                             CustomCause::MissingModulesFile,
    ///                         ))
    ///                     }
    ///                 }
    ///             },
    ///         },
    ///     )?;
    ///
    ///     match layer_ref.state {
    ///         LayerState::Empty { ref cause } => {
    ///             // Since the cause is just a regular Rust value, we can match it with regular
    ///             // Rust syntax and be as complex or simple as we need.
    ///             let message = match cause {
    ///                 EmptyLayerCause::RestoredLayerAction {
    ///                     cause: CustomCause::LegacyVersion,
    ///                 } => "Re-installing language runtime (legacy cached version)",
    ///                 EmptyLayerCause::RestoredLayerAction {
    ///                     cause: CustomCause::HasBrokenModule | CustomCause::MissingModulesFile,
    ///                 } => "Re-installing language runtime (broken modules detected)",
    ///                 _ => "Installing language runtime",
    ///             };
    ///
    ///             println!("{message}");
    ///
    ///             // Code to install the language runtime would go here
    ///
    ///             layer_ref.write_metadata(ExampleLayerMetadata {
    ///                 lang_runtime_version: String::from("1.0.0"),
    ///             })?;
    ///         }
    ///         LayerState::Restored { .. } => {
    ///             println!("Re-using cached language runtime");
    ///         }
    ///     }
    ///
    ///     BuildResultBuilder::new().build()
    /// }
    /// # }
    /// #
    /// # impl From<ExampleBuildpackError> for libcnb::Error<ExampleBuildpackError> {
    /// #     fn from(value: ExampleBuildpackError) -> Self {
    /// #         Self::BuildpackError(value)
    /// #     }
    /// # }
    /// ```
    pub fn cached_layer<'a, M, MA, RA, MAC, RAC>(
        &self,
        layer_name: impl Borrow<LayerName>,
        layer_definition: impl Borrow<CachedLayerDefinition<'a, M, MA, RA>>,
    ) -> crate::Result<LayerRef<B, MAC, RAC>, B::Error>
    where
        M: 'a + Serialize + DeserializeOwned,
        MA: 'a + IntoAction<InvalidMetadataAction<M>, MAC, B::Error>,
        RA: 'a + IntoAction<RestoredLayerAction, RAC, B::Error>,
    {
        let layer_definition = layer_definition.borrow();

        crate::layer::struct_api::handling::handle_layer(
            LayerTypes {
                launch: layer_definition.launch,
                build: layer_definition.build,
                cache: true,
            },
            layer_definition.invalid_metadata_action,
            layer_definition.restored_layer_action,
            layer_name.borrow(),
            &self.layers_dir,
        )
    }

    /// Creates an uncached layer.
    ///
    /// If the layer already exists because it was cached in a previous buildpack run, the existing
    /// data will be deleted.
    ///
    /// This function is essentially the same as [`BuildContext::uncached_layer`] but simpler.
    pub fn uncached_layer(
        &self,
        layer_name: impl Borrow<LayerName>,
        layer_definition: impl Borrow<UncachedLayerDefinition>,
    ) -> crate::Result<LayerRef<B, (), ()>, B::Error> {
        let layer_definition = layer_definition.borrow();

        crate::layer::struct_api::handling::handle_layer(
            LayerTypes {
                launch: layer_definition.launch,
                build: layer_definition.build,
                cache: false,
            },
            &|_| InvalidMetadataAction::DeleteLayer,
            &|_: &GenericMetadata, _| RestoredLayerAction::DeleteLayer,
            layer_name.borrow(),
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
