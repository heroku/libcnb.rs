use serde::{Deserialize, Serialize};

/// Describes Layer Content Metadata
///
/// See [Cloud Native Buildpack specification](https://github.com/buildpacks/spec/blob/main/buildpack.md#layer-content-metadata-toml)
#[derive(Debug, Deserialize, Serialize)]
#[serde(deny_unknown_fields)]
pub struct LayerContentMetadata<M> {
    pub types: Option<LayerTypes>,

    /// Metadata that describes the layer contents.
    pub metadata: M,
}

impl<M: PartialEq> PartialEq for LayerContentMetadata<M> {
    fn eq(&self, other: &Self) -> bool {
        self.types == other.types && self.metadata == other.metadata
    }
}

/// Used to specify layer availability based
/// on buildpack phase.
#[derive(Debug, Default, Deserialize, Serialize, Eq, PartialEq)]
#[serde(deny_unknown_fields)]
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

#[cfg(test)]
mod tests {
    use super::*;

    type GenericLayerContentMetadata = LayerContentMetadata<Option<toml::value::Table>>;

    #[test]
    fn deserialize_everything() {
        let toml_str = r#"
        [types]
        launch = true
        build = true
        cache = false

        [metadata]
        version = "1.2.3"
        "#;
        let layer = toml::from_str::<GenericLayerContentMetadata>(toml_str).unwrap();
        assert_eq!(
            layer.types,
            Some(LayerTypes {
                launch: true,
                build: true,
                cache: false
            })
        );
        assert_eq!(
            layer.metadata.unwrap().get("version"),
            Some(&toml::value::Value::try_from("1.2.3").unwrap())
        );
    }

    #[test]
    fn deserialize_empty() {
        let layer = toml::from_str::<GenericLayerContentMetadata>("").unwrap();
        assert_eq!(layer.types, None);
        assert_eq!(layer.metadata, None);
    }

    #[test]
    fn types_table_with_no_entries_has_defaults() {
        let toml_str = r#"
        [types]
        "#;
        let layer = toml::from_str::<GenericLayerContentMetadata>(toml_str).unwrap();
        assert_eq!(
            layer.types,
            Some(LayerTypes {
                launch: false,
                build: false,
                cache: false
            })
        );
        assert_eq!(layer.metadata, None);
    }
}
