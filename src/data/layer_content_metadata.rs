use serde::{Deserialize, Serialize};

use crate::data::defaults;
use crate::generic::GenericMetadata;

/// Describes Layer Content Metadata
///
/// See [Cloud Native Buildpack specification](https://github.com/buildpacks/spec/blob/main/buildpack.md#layer-content-metadata-toml)
#[derive(Debug, Deserialize, Serialize)]
pub struct LayerContentMetadata<M> {
    /// Whether the layer is intended for launch.
    #[serde(default = "defaults::r#false")]
    pub launch: bool,

    /// Whether the layer is intended for build.
    #[serde(default = "defaults::r#false")]
    pub build: bool,

    /// Whether the layer is cached.
    #[serde(default = "defaults::r#false")]
    pub cache: bool,

    /// Metadata that describes the layer contents.
    pub metadata: M,
}

impl Default for LayerContentMetadata<GenericMetadata> {
    fn default() -> Self {
        LayerContentMetadata {
            launch: false,
            build: false,
            cache: false,
            metadata: GenericMetadata::default(),
        }
    }
}

impl<M> LayerContentMetadata<M> {
    pub fn launch(mut self, launch: bool) -> Self {
        self.launch = launch;
        self
    }

    pub fn build(mut self, build: bool) -> Self {
        self.build = build;
        self
    }

    pub fn cache(mut self, cache: bool) -> Self {
        self.cache = cache;
        self
    }

    pub fn metadata<NM>(&mut self, metadata: NM) -> LayerContentMetadata<NM> {
        LayerContentMetadata {
            cache: self.cache,
            build: self.build,
            launch: self.launch,
            metadata,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn metadata_is_optional() {
        let layer: Result<LayerContentMetadata<Option<toml::value::Table>>, toml::de::Error> =
            toml::from_str(
                r#"
            launch = true
            build = true
            cache = false
            "#,
            );

        assert!(!layer.is_err());
    }
}
