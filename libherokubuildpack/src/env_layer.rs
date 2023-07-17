use libcnb::build::BuildContext;
use libcnb::data::layer::LayerName;
use libcnb::data::layer_content_metadata::LayerTypes;
use libcnb::generic::GenericMetadata;
use libcnb::layer::{Layer, LayerData, LayerResult, LayerResultBuilder};
use libcnb::layer_env::{LayerEnv, ModificationBehavior, Scope};
use libcnb::Buildpack;
use std::ffi::OsString;
use std::marker::PhantomData;
use std::path::Path;

/// Set default environment variables
///
/// If all you need to do is set default environment values, you can use
/// the `env_layer::set_default` function to set those values without having
/// to create a struct from scratch. Example:
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
/// use libcnb::layer_env::Scope;
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
///         // Don't forget to apply context.platform.env() in addition to current envs;
///         let env = Env::from_current();
///
///         let layer = env_layer::set_default(&context, layer_name!("default_env"),
///             [
///                 ("JRUBY_OPTS", "-Xcompile.invokedynamic=false"),
///                 ("RACK_ENV", "production"),
///                 ("RAILS_ENV", "production"),
///                 ("RAILS_SERVE_STATIC_FILES", "enabled"),
///                 ("RAILS_LOG_TO_STDOUT", "enabled"),
///                 ("MALLOC_ARENA_MAX", "2"),
///                 ("DISABLE_SPRING", "1"),
///             ]
///             .into_iter(),
///         )?;
///         let env = layer.env.apply(Scope::Build, &env);
///
///#        todo!()
///#     }
///# }
/// ```
pub fn set_default<B, E, K, V>(
    context: &BuildContext<B>,
    layer_name: LayerName,
    envs: E,
) -> libcnb::Result<LayerData<GenericMetadata>, <B as Buildpack>::Error>
where
    B: Buildpack,
    E: IntoIterator<Item = (K, V)> + Clone,
    K: Into<OsString>,
    V: Into<OsString>,
{
    context.handle_layer(layer_name, DefaultEnvLayer::new(envs))
}

/// Set default environment variables in a layer
///
/// This struct is used by the helper function `set_default`. You can also use it directly with
/// with [`BuildContext::handle_layer`] to set default environment variables.
pub struct DefaultEnvLayer;

impl DefaultEnvLayer {
    #[allow(clippy::new_ret_no_self)]
    pub fn new<E, K, V, B>(env: E) -> ConfigureEnvLayer<B>
    where
        E: IntoIterator<Item = (K, V)> + Clone,
        K: Into<OsString>,
        V: Into<OsString>,
        B: libcnb::Buildpack,
    {
        let mut layer_env = LayerEnv::new();
        for (key, value) in env {
            layer_env =
                layer_env.chainable_insert(Scope::All, ModificationBehavior::Default, key, value);
        }

        ConfigureEnvLayer {
            data: layer_env,
            _buildpack: PhantomData,
        }
    }
}

/// Set environment variables
///
/// If you want to set many default environment variables you can use the
/// `env_layer::set_default` function. If you need to set different types of environment
/// variables you can use the `env_layer::set_envs` function. Example:
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
///         // Don't forget to apply context.platform.env() too;
///         let env = Env::from_current();
///
///         let env = {
///             let layer = env_layer::set_envs(
///                 &context,
///                 layer_name!("configure_env"),
///                 LayerEnv::new()
///                     .chainable_insert(
///                         Scope::All,
///                         ModificationBehavior::Override,
///                         "BUNDLE_GEMFILE", // Tells bundler where to find the `Gemfile`
///                         context.app_dir.join("Gemfile"),
///                     )
///                     .chainable_insert(
///                         Scope::All,
///                         ModificationBehavior::Override,
///                         "BUNDLE_CLEAN", // After successful `bundle install` bundler will automatically run `bundle clean`
///                         "1",
///                     )
///                     .chainable_insert(
///                         Scope::All,
///                         ModificationBehavior::Override,
///                         "BUNDLE_DEPLOYMENT", // Requires the `Gemfile.lock` to be in sync with the current `Gemfile`.
///                         "1",
///                     )
///                     .chainable_insert(
///                         Scope::All,
///                         ModificationBehavior::Default,
///                         "MY_ENV_VAR",
///                         "Whatever I want"
///                     )
///             )?;
///             layer.env.apply(Scope::Build, &env)
///         };
///
///#        todo!()
///#     }
///# }
/// ```
pub fn set_envs<B>(
    context: &BuildContext<B>,
    layer_name: LayerName,
    envs: LayerEnv,
) -> libcnb::Result<LayerData<GenericMetadata>, <B as Buildpack>::Error>
where
    B: Buildpack,
{
    context.handle_layer(layer_name, ConfigureEnvLayer::new(envs))
}

/// Set custom environment variables in a layer
///
/// This struct is used by the helper function `set_envs`. You can also use it directly with
/// use directly with [`BuildContext::handle_layer`] to set specific environment variables.
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
