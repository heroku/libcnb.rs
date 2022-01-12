use std::path::Path;

use crate::util;

use tempfile::NamedTempFile;

use crate::{RubyBuildpack, RubyBuildpackError};
use libcnb::build::BuildContext;
use libcnb::data::layer_content_metadata::LayerTypes;
use libcnb::generic::GenericMetadata;
use libcnb::layer::{Layer, LayerResult, LayerResultBuilder};
use libcnb::layer_env::{LayerEnv, ModificationBehavior, Scope};

pub struct RubyLayer;

impl Layer for RubyLayer {
    type Buildpack = RubyBuildpack;
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
        context: &BuildContext<Self::Buildpack>,
        layer_path: &Path,
    ) -> Result<LayerResult<Self::Metadata>, RubyBuildpackError> {
        println!("---> Download and extracting Ruby");

        let ruby_tgz =
            NamedTempFile::new().map_err(RubyBuildpackError::CouldNotCreateTemporaryFile)?;

        util::download(
            &context.buildpack_descriptor.metadata.ruby_url,
            ruby_tgz.path(),
        )
        .map_err(RubyBuildpackError::RubyDownloadError)?;

        util::untar(ruby_tgz.path(), &layer_path).map_err(RubyBuildpackError::RubyUntarError)?;

        LayerResultBuilder::new(GenericMetadata::default())
            .env(
                LayerEnv::new()
                    .chainable_insert(
                        Scope::All,
                        ModificationBehavior::Prepend,
                        "PATH",
                        context.app_dir.join(".gem/ruby/2.6.6/bin"),
                    )
                    .chainable_insert(
                        Scope::All,
                        ModificationBehavior::Prepend,
                        "LD_LIBRARY_PATH",
                        layer_path,
                    ),
            )
            .build()
    }
}
