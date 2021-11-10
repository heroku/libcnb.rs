//! Provides build phase specific types and helpers.
use serde::de::DeserializeOwned;
use serde::Serialize;
use std::{fs, path::PathBuf};

use crate::buildpack::Buildpack;
use crate::data::store::Store;
use crate::layer::{Layer, LayerData};

use crate::data::{buildpack::BuildpackToml, buildpack_plan::BuildpackPlan, launch::Launch};
use crate::HandleLayerErrorOrBuildpackError;

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

impl<B: Buildpack + ?Sized> BuildContext<B> {
    pub fn handle_layer<L: Layer<Buildpack = B>>(
        &self,
        name: impl AsRef<str>,
        layer: L,
    ) -> crate::Result<LayerData<L::Metadata>, B::Error> {
        crate::layer::handle_layer(&self, name.as_ref(), layer).map_err(|error| match error {
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
