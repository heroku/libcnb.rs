use libcnb::build::BuildContext;
use libcnb::data::layer_content_metadata::LayerTypes;
use libcnb::generic::GenericMetadata;
use libcnb::layer::{Layer, LayerResult, LayerResultBuilder};
use libcnb::layer_env::LayerEnv;
use std::marker::PhantomData;
use std::path::Path;

/// Convenience layer for setting environment variables
///
/// If you do not need to modify files on disk or cache metadata, you can use this layer along with
/// [`BuildContext::handle_layer`] to apply results of [`LayerEnv::chainable_insert`] to build and
/// launch (runtime) environments.
///
/// Example:
///
/// ```no_run
///# use libcnb::build::{BuildContext, BuildResult, BuildResultBuilder};
///# use libcnb::data::launch::{LaunchBuilder, ProcessBuilder};
///# use libcnb::data::process_type;
///# use libcnb::detect::{DetectContext, DetectResult, DetectResultBuilder};
///# use libcnb::generic::{GenericError, GenericMetadata, GenericPlatform};
///# use libcnb::{buildpack_main, Buildpack};
///# use libcnb::data::layer::LayerName;
///
///# pub(crate) struct HelloWorldBuildpack;
///
/// use libcnb::Env;
/// use libcnb::data::layer_name;
/// use libcnb::layer_env::{LayerEnv, ModificationBehavior, Scope};
/// use libherokubuildpack::env_layer;
///
///# impl Buildpack for HelloWorldBuildpack {
///#     type Platform = GenericPlatform;
///#     type Metadata = GenericMetadata;
///#     type Error = GenericError;
///
///#     fn detect(&self, _context: DetectContext<Self>) -> libcnb::Result<DetectResult, Self::Error> {
///#         todo!()
///#     }
///
///#     fn build(&self, context: BuildContext<Self>) -> libcnb::Result<BuildResult, Self::Error> {
///         let env = Env::from_current();
///
///         let env = {
///             let layer = context.handle_layer(
///                 layer_name!("configure_env"),
///                 env_layer::ConfigureEnvLayer::new(
///                     LayerEnv::new()
///                         .chainable_insert(
///                             Scope::All,
///                             ModificationBehavior::Override,
///                             "BUNDLE_GEMFILE", // Tells bundler where to find the `Gemfile`
///                             context.app_dir.join("Gemfile"),
///                         )
///                         .chainable_insert(
///                             Scope::All,
///                             ModificationBehavior::Override,
///                             "BUNDLE_CLEAN", // After successful `bundle install` bundler will automatically run `bundle clean`
///                             "1",
///                         )
///                         .chainable_insert(
///                             Scope::All,
///                             ModificationBehavior::Override,
///                             "BUNDLE_DEPLOYMENT", // Requires the `Gemfile.lock` to be in sync with the current `Gemfile`.
///                             "1",
///                         )
///                         .chainable_insert(
///                             Scope::All,
///                             ModificationBehavior::Default,
///                             "MY_ENV_VAR",
///                             "Whatever I want",
///                         ),
///                 ),
///             )?;
///             layer.env.apply(Scope::Build, &env)
///         };
///
///#        todo!()
///#     }
///# }
/// ```
pub struct ConfigureEnvLayer<B: libcnb::Buildpack> {
    pub(crate) data: LayerEnv,
    pub(crate) _buildpack: std::marker::PhantomData<B>,
}

impl<B> ConfigureEnvLayer<B>
where
    B: libcnb::Buildpack,
{
    #[must_use]
    pub fn new(env: LayerEnv) -> Self {
        ConfigureEnvLayer {
            data: env,
            _buildpack: PhantomData,
        }
    }
}

impl<B> Layer for ConfigureEnvLayer<B>
where
    B: libcnb::Buildpack,
{
    type Buildpack = B;
    type Metadata = GenericMetadata;

    fn types(&self) -> LayerTypes {
        LayerTypes {
            build: true,
            launch: true,
            cache: false,
        }
    }

    fn create(
        &self,
        _context: &BuildContext<Self::Buildpack>,
        _layer_path: &Path,
    ) -> Result<LayerResult<Self::Metadata>, B::Error> {
        LayerResultBuilder::new(GenericMetadata::default())
            .env(self.data.clone())
            .build()
    }
}
