use serde::{Deserialize, Serialize};

/// Used to specify layer availability based
/// on buildpack phase.
#[derive(Debug, Default, Deserialize, Serialize, Eq, PartialEq)]
pub struct LayerTypes {
    /// Whether the layer is intended for launch.
    #[serde(default)]
    pub launch: bool,

    /// Whether the layer is intended for build.
    #[serde(default)]
    pub build: bool,

    /// Whether the layer is cached.
    #[serde(default)]
    pub cache: bool,
}

/// Describes Layer Content Metadata
///
/// See [Cloud Native Buildpack specification](https://github.com/buildpacks/spec/blob/main/buildpack.md#layer-content-metadata-toml)
#[derive(Debug, Deserialize, Serialize)]
pub struct LayerContentMetadata<M> {
    #[serde(default)]
    pub types: LayerTypes,

    /// Metadata that describes the layer contents.
    pub metadata: M,
}

impl<M: PartialEq> PartialEq for LayerContentMetadata<M> {
    fn eq(&self, other: &Self) -> bool {
        self.types == other.types && self.metadata == other.metadata
    }
}

impl<M: Default> Default for LayerContentMetadata<M> {
    fn default() -> Self {
        Self {
            types: LayerTypes::default(),
            metadata: M::default(),
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
