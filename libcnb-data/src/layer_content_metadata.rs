use serde::{Deserialize, Serialize};

use crate::defaults;

/// Used to specify layer availability based
/// on buildpack phase.
#[derive(Debug, Deserialize, Serialize)]
pub struct LayerContentTypeTable {
    /// Whether the layer is intended for launch.
    #[serde(default = "defaults::r#false")]
    pub launch: bool,

    /// Whether the layer is intended for build.
    #[serde(default = "defaults::r#false")]
    pub build: bool,

    /// Whether the layer is cached.
    #[serde(default = "defaults::r#false")]
    pub cache: bool,
}

impl LayerContentTypeTable {
    fn default() -> Self {
        Self {
            launch: false,
            build: false,
            cache: false,
        }
    }
}

/// Describes Layer Content Metadata
///
/// See [Cloud Native Buildpack specification](https://github.com/buildpacks/spec/blob/main/buildpack.md#layer-content-metadata-toml)
///
/// ```
/// use libcnb_data::layer_content_metadata::LayerContentMetadata;
/// use toml::toml;
///
/// let layer = LayerContentMetadata::default()
///   .build(true)
///   .cache(true)
///   .launch(true)
///   .metadata(
///     toml! {
///       version = "2.5"
///       name = "ACME Corp."
///     });
///
/// assert!(layer.types.build);
///
/// let version = layer.metadata.get("version").unwrap().as_str().unwrap();
/// assert_eq!(version, "2.5");
/// ```
#[derive(Debug, Deserialize, Serialize)]
pub struct LayerContentMetadata<M> {
    #[serde(default = "LayerContentTypeTable::default")]
    pub types: LayerContentTypeTable,

    /// Metadata that describes the layer contents.
    pub metadata: M,
}

impl Default for LayerContentMetadata<Option<toml::Value>> {
    fn default() -> Self {
        Self {
            types: LayerContentTypeTable::default(),
            metadata: Option::default(),
        }
    }
}

impl<M> LayerContentMetadata<M> {
    pub fn launch(mut self, launch: bool) -> Self {
        self.types.launch = launch;
        self
    }

    pub fn build(mut self, build: bool) -> Self {
        self.types.build = build;
        self
    }

    pub fn cache(mut self, cache: bool) -> Self {
        self.types.cache = cache;
        self
    }

    pub fn metadata<NM>(&mut self, metadata: NM) -> LayerContentMetadata<NM> {
        LayerContentMetadata {
            types: LayerContentTypeTable {
                cache: self.types.cache,
                build: self.types.build,
                launch: self.types.launch,
            },

            metadata,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn layer_types_have_defaults() {
        let layer: Result<LayerContentMetadata<Option<toml::value::Table>>, toml::de::Error> =
            toml::from_str(
                r#"
            [types]
            "#,
            );

        let layer = layer.unwrap();
        assert_eq!(layer.metadata, None);
        assert!(!layer.types.launch);
        assert!(!layer.types.build);
        assert!(!layer.types.cache);

        let layer: Result<LayerContentMetadata<Option<toml::value::Table>>, toml::de::Error> =
            toml::from_str(r#""#);

        let layer = layer.unwrap();
        assert_eq!(layer.metadata, None);
        assert!(!layer.types.launch);
        assert!(!layer.types.build);
        assert!(!layer.types.cache);
    }

    #[test]
    fn metadata_is_optional() {
        let layer: Result<LayerContentMetadata<Option<toml::value::Table>>, toml::de::Error> =
            toml::from_str(
                r#"
            [types]
            launch = true
            build = true
            cache = false
            "#,
            );

        let layer = layer.unwrap();
        assert_eq!(layer.metadata, None);
        assert!(layer.types.launch);
        assert!(layer.types.build);
        assert!(!layer.types.cache);
    }
}
