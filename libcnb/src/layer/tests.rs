//! Tests for [`Layer`] and [`layer::handle_layer`].
//!
//! Even though they are implemented as unit tests, they are really integration tests since they
//! need files on disk, an implementation of [`Layer`] and [`Buildpack`], and test logic across all
//! of these parts. There is no other way of testing this if we want to keep the public API as-is.
//! Please note that individual building blocks that are used in [`layer::handle_layer`] are unit
//! tested separately.
//!
//! All tests in this module assume a specific [`Layer`] implementation that is also in this file.
//! Look for the `TestLayer` type and it's [`Layer`] implementation.

use crate::build::{BuildContext, BuildResult, BuildResultBuilder};
use crate::data::buildpack_id;
use crate::data::layer_content_metadata::LayerTypes;
use crate::data::stack_id;
use crate::detect::{DetectContext, DetectResult, DetectResultBuilder};
use crate::generic::{GenericMetadata, GenericPlatform};
use crate::layer::{
    handle_layer, ExistingLayerStrategy, Layer, LayerData, LayerResult, LayerResultBuilder,
    MetadataMigration,
};
use crate::layer_env::{LayerEnv, ModificationBehavior, Scope};
use crate::{read_toml_file, Buildpack, Env, LIBCNB_SUPPORTED_BUILDPACK_API};
use libcnb_data::buildpack::{BuildpackVersion, SingleBuildpackDescriptor, Stack};
use libcnb_data::buildpack_plan::BuildpackPlan;
use libcnb_data::layer::LayerName;
use libcnb_data::layer_content_metadata::LayerContentMetadata;
use serde::Deserialize;
use serde::Serialize;
use std::collections::HashSet;
use std::fs;
use std::iter::repeat_with;
use std::path::{Path, PathBuf};
use tempfile::{tempdir, TempDir};

const TEST_LAYER_LAUNCH: bool = true;
const TEST_LAYER_BUILD: bool = true;
const TEST_LAYER_CACHE: bool = true;
const TEST_LAYER_CREATE_FILE_CONTENTS: &str = "ran";
const TEST_LAYER_UPDATE_FILE_CONTENTS: &str = "ran";
const TEST_LAYER_CREATE_FILE_NAME: &str = "create";
const TEST_LAYER_UPDATE_FILE_NAME: &str = "update";

#[derive(Clone)]
struct TestLayer {
    existing_layer_strategy: ExistingLayerStrategy,
    write_layer_env: Option<LayerEnv>,
    write_version: String,
}

impl Layer for TestLayer {
    type Buildpack = TestBuildpack;
    type Metadata = TestLayerMetadata;

    fn types(&self) -> LayerTypes {
        LayerTypes {
            launch: TEST_LAYER_LAUNCH,
            build: TEST_LAYER_BUILD,
            cache: TEST_LAYER_CACHE,
        }
    }

    fn create(
        &self,
        _context: &BuildContext<Self::Buildpack>,
        layer_path: &Path,
    ) -> Result<LayerResult<Self::Metadata>, <Self::Buildpack as Buildpack>::Error> {
        fs::write(
            layer_path.join(TEST_LAYER_CREATE_FILE_NAME),
            TEST_LAYER_CREATE_FILE_CONTENTS,
        )
        .map_err(TestBuildpackError::IoError)?;

        LayerResultBuilder::new(TestLayerMetadata {
            version: self.write_version.clone(),
        })
        .env(self.write_layer_env.clone().unwrap_or_default())
        .build()
    }

    fn existing_layer_strategy(
        &self,
        _context: &BuildContext<Self::Buildpack>,
        _layer_data: &LayerData<Self::Metadata>,
    ) -> Result<ExistingLayerStrategy, <Self::Buildpack as Buildpack>::Error> {
        Ok(self.existing_layer_strategy)
    }

    fn update(
        &self,
        _context: &BuildContext<Self::Buildpack>,
        layer_data: &LayerData<Self::Metadata>,
    ) -> Result<LayerResult<Self::Metadata>, <Self::Buildpack as Buildpack>::Error> {
        fs::write(
            layer_data.path.join(TEST_LAYER_UPDATE_FILE_NAME),
            TEST_LAYER_UPDATE_FILE_CONTENTS,
        )
        .map_err(TestBuildpackError::IoError)?;

        LayerResultBuilder::new(layer_data.content_metadata.metadata.clone()).build()
    }

    fn migrate_incompatible_metadata(
        &self,
        _context: &BuildContext<Self::Buildpack>,
        metadata: &GenericMetadata,
    ) -> Result<MetadataMigration<Self::Metadata>, <Self::Buildpack as Buildpack>::Error> {
        match metadata.clone().and_then(|toml| toml.get("v").cloned()) {
            Some(toml::Value::String(version)) => {
                Ok(MetadataMigration::ReplaceMetadata(TestLayerMetadata {
                    version,
                }))
            }
            _ => Ok(MetadataMigration::RecreateLayer),
        }
    }
}

#[derive(Deserialize, Serialize, Debug, Clone, Eq, PartialEq)]
struct TestLayerMetadata {
    version: String,
}

impl Default for TestLayer {
    fn default() -> Self {
        Self {
            existing_layer_strategy: ExistingLayerStrategy::Recreate,
            write_version: String::from("1.0.0"),
            write_layer_env: None,
        }
    }
}

#[test]
fn create() {
    let temp_dir = tempdir().unwrap();
    let context = build_context(&temp_dir);
    let layer_name = random_layer_name();
    let metadata_version_string = String::from("1.0.0");
    let test_layer = TestLayer {
        existing_layer_strategy: ExistingLayerStrategy::Keep,
        write_version: metadata_version_string.clone(),
        write_layer_env: None,
    };

    let handle_layer_result = handle_layer(&context, layer_name.clone(), test_layer).unwrap();

    // Assert layer content metadata, returned and on disk
    let expected_content_layer_metadata = LayerContentMetadata {
        types: Some(LayerTypes {
            launch: TEST_LAYER_LAUNCH,
            build: TEST_LAYER_BUILD,
            cache: TEST_LAYER_CACHE,
        }),
        metadata: TestLayerMetadata {
            version: metadata_version_string,
        },
    };

    assert_eq!(
        &handle_layer_result.content_metadata,
        &expected_content_layer_metadata
    );

    let layer_content_metadata_from_disk: LayerContentMetadata<TestLayerMetadata> = read_toml_file(
        temp_dir
            .path()
            .join("layers")
            .join(format!("{layer_name}.toml")),
    )
    .unwrap();

    // Assert basic LayerData fields
    assert_eq!(
        &layer_content_metadata_from_disk,
        &expected_content_layer_metadata
    );

    assert_eq!(handle_layer_result.name, layer_name);
    assert_eq!(
        handle_layer_result.path,
        temp_dir.path().join("layers").join(layer_name.as_str())
    );

    assert_eq!(handle_layer_result.env, LayerEnv::new());

    // Assert layer directory contents
    let create_file_contents =
        fs::read_to_string(handle_layer_result.path.join(TEST_LAYER_CREATE_FILE_NAME)).ok();

    let update_file_contents =
        fs::read_to_string(handle_layer_result.path.join(TEST_LAYER_UPDATE_FILE_NAME)).ok();

    assert_eq!(
        create_file_contents,
        Some(String::from(TEST_LAYER_CREATE_FILE_CONTENTS))
    );

    assert_eq!(update_file_contents, None);
}

#[test]
fn create_then_update() {
    let temp_dir = tempdir().unwrap();
    let context = build_context(&temp_dir);
    let layer_name = random_layer_name();
    let metadata_version_string = String::from("2.3.4");
    let test_layer = TestLayer {
        existing_layer_strategy: ExistingLayerStrategy::Update,
        write_version: metadata_version_string.clone(),
        write_layer_env: None,
    };

    handle_layer(&context, layer_name.clone(), test_layer.clone()).unwrap();
    let handle_layer_result = handle_layer(&context, layer_name.clone(), test_layer).unwrap();

    // Assert layer content metadata, returned and on disk
    let expected_content_layer_metadata = LayerContentMetadata {
        types: Some(LayerTypes {
            launch: TEST_LAYER_LAUNCH,
            build: TEST_LAYER_BUILD,
            cache: TEST_LAYER_CACHE,
        }),
        metadata: TestLayerMetadata {
            version: metadata_version_string,
        },
    };

    assert_eq!(
        &handle_layer_result.content_metadata,
        &expected_content_layer_metadata
    );

    let layer_content_metadata_from_disk: LayerContentMetadata<TestLayerMetadata> = read_toml_file(
        temp_dir
            .path()
            .join("layers")
            .join(format!("{layer_name}.toml")),
    )
    .unwrap();

    // Assert basic LayerData fields
    assert_eq!(
        &layer_content_metadata_from_disk,
        &expected_content_layer_metadata
    );

    assert_eq!(handle_layer_result.name, layer_name);
    assert_eq!(
        handle_layer_result.path,
        temp_dir.path().join("layers").join(layer_name.as_str())
    );

    assert_eq!(handle_layer_result.env, LayerEnv::new());

    // Assert layer directory contents
    let create_file_contents =
        fs::read_to_string(handle_layer_result.path.join(TEST_LAYER_CREATE_FILE_NAME)).ok();

    let update_file_contents =
        fs::read_to_string(handle_layer_result.path.join(TEST_LAYER_UPDATE_FILE_NAME)).ok();

    assert_eq!(
        create_file_contents,
        Some(String::from(TEST_LAYER_CREATE_FILE_CONTENTS))
    );

    assert_eq!(
        update_file_contents,
        Some(String::from(TEST_LAYER_UPDATE_FILE_CONTENTS))
    );
}

#[test]
fn create_then_recreate() {
    let temp_dir = tempdir().unwrap();
    let context = build_context(&temp_dir);
    let layer_name = random_layer_name();
    let residue_file_name = "RESIDUE.txt";
    let metadata_version_string = String::from("2.3.4");
    let recreated_metadata_version_string = String::from("1.0.0");
    let test_layer = TestLayer {
        existing_layer_strategy: ExistingLayerStrategy::Recreate,
        write_version: metadata_version_string,
        write_layer_env: None,
    };

    let handle_layer_result =
        handle_layer(&context, layer_name.clone(), test_layer.clone()).unwrap();

    // Add a random file to the layer directory between handle_layer calls to assess if the layer is
    // actually recreated without any residue left in the directory
    fs::write(
        handle_layer_result.path.join(residue_file_name),
        "RESIDUE DATA",
    )
    .unwrap();

    let handle_layer_result = handle_layer(
        &context,
        layer_name.clone(),
        TestLayer {
            write_version: recreated_metadata_version_string.clone(),
            ..test_layer
        },
    )
    .unwrap();

    // Assert layer content metadata, returned and on disk
    let expected_content_layer_metadata = LayerContentMetadata {
        types: Some(LayerTypes {
            launch: TEST_LAYER_LAUNCH,
            build: TEST_LAYER_BUILD,
            cache: TEST_LAYER_CACHE,
        }),
        metadata: TestLayerMetadata {
            version: recreated_metadata_version_string,
        },
    };

    assert_eq!(
        &handle_layer_result.content_metadata,
        &expected_content_layer_metadata
    );

    let layer_content_metadata_from_disk: LayerContentMetadata<TestLayerMetadata> = read_toml_file(
        temp_dir
            .path()
            .join("layers")
            .join(format!("{layer_name}.toml")),
    )
    .unwrap();

    // Assert basic LayerData fields
    assert_eq!(
        &layer_content_metadata_from_disk,
        &expected_content_layer_metadata
    );

    assert_eq!(handle_layer_result.name, layer_name);
    assert_eq!(
        handle_layer_result.path,
        temp_dir.path().join("layers").join(layer_name.as_str())
    );

    assert_eq!(handle_layer_result.env, LayerEnv::new());

    // Assert layer directory contents
    let create_file_contents =
        fs::read_to_string(handle_layer_result.path.join(TEST_LAYER_CREATE_FILE_NAME)).ok();

    let update_file_contents =
        fs::read_to_string(handle_layer_result.path.join(TEST_LAYER_UPDATE_FILE_NAME)).ok();

    let residue_file_contents =
        fs::read_to_string(handle_layer_result.path.join(residue_file_name)).ok();

    assert_eq!(
        create_file_contents,
        Some(String::from(TEST_LAYER_CREATE_FILE_CONTENTS))
    );

    assert_eq!(update_file_contents, None);
    assert_eq!(residue_file_contents, None);
}

#[test]
fn create_then_keep() {
    let temp_dir = tempdir().unwrap();
    let context = build_context(&temp_dir);
    let layer_name = random_layer_name();
    let residue_file_name = "RESIDUE.txt";
    let residue_file_data = "RESIDUE DATA";
    let metadata_version_string = String::from("0.1.2");
    let test_layer = TestLayer {
        existing_layer_strategy: ExistingLayerStrategy::Keep,
        write_version: metadata_version_string.clone(),
        write_layer_env: None,
    };

    let handle_layer_result =
        handle_layer(&context, layer_name.clone(), test_layer.clone()).unwrap();

    // Add a random file to the layer directory between handle_layer calls to assess if the layer is
    // kept as-is.
    fs::write(
        handle_layer_result.path.join(residue_file_name),
        residue_file_data,
    )
    .unwrap();

    let handle_layer_result = handle_layer(
        &context,
        layer_name.clone(),
        TestLayer {
            // Since we want to keep an existing layer as-is, this version must not be written
            write_version: String::from("UNUSED_BECAUSE_OF_EXISTING_LAYER_STRATEGY_KEEP"),
            ..test_layer
        },
    )
    .unwrap();

    // Assert layer content metadata, returned and on disk
    let expected_content_layer_metadata = LayerContentMetadata {
        types: Some(LayerTypes {
            launch: TEST_LAYER_LAUNCH,
            build: TEST_LAYER_BUILD,
            cache: TEST_LAYER_CACHE,
        }),
        metadata: TestLayerMetadata {
            version: metadata_version_string,
        },
    };

    assert_eq!(
        &handle_layer_result.content_metadata,
        &expected_content_layer_metadata
    );

    let layer_content_metadata_from_disk: LayerContentMetadata<TestLayerMetadata> = read_toml_file(
        temp_dir
            .path()
            .join("layers")
            .join(format!("{layer_name}.toml")),
    )
    .unwrap();

    // Assert basic LayerData fields
    assert_eq!(
        &layer_content_metadata_from_disk,
        &expected_content_layer_metadata
    );

    assert_eq!(handle_layer_result.name, layer_name);
    assert_eq!(
        handle_layer_result.path,
        temp_dir.path().join("layers").join(layer_name.as_str())
    );

    assert_eq!(handle_layer_result.env, LayerEnv::new());

    // Assert layer directory contents
    let create_file_contents =
        fs::read_to_string(handle_layer_result.path.join(TEST_LAYER_CREATE_FILE_NAME)).ok();

    let update_file_contents =
        fs::read_to_string(handle_layer_result.path.join(TEST_LAYER_UPDATE_FILE_NAME)).ok();

    let residue_file_contents =
        fs::read_to_string(handle_layer_result.path.join(residue_file_name)).ok();

    assert_eq!(
        create_file_contents,
        Some(String::from(TEST_LAYER_CREATE_FILE_CONTENTS))
    );

    assert_eq!(update_file_contents, None);
    assert_eq!(residue_file_contents, Some(String::from(residue_file_data)));
}

#[test]
fn update_with_incompatible_metadata_replace() {
    let temp_dir = tempdir().unwrap();
    let context = build_context(&temp_dir);
    let layer_name = random_layer_name();
    let metadata_version_string = String::from("2.3.4");
    let test_layer = TestLayer {
        existing_layer_strategy: ExistingLayerStrategy::Update,
        write_version: metadata_version_string,
        write_layer_env: None,
    };

    // Create a layer by hand that has incompatible metadata
    let test_layer_dir = temp_dir.path().join("layers").join(layer_name.as_str());
    fs::create_dir_all(test_layer_dir).unwrap();

    let test_layer_toml = temp_dir
        .path()
        .join("layers")
        .join(format!("{layer_name}.toml"));

    fs::write(
        test_layer_toml,
        r#"
[metadata]
v = "3.2.1"
    "#,
    )
    .unwrap();

    let handle_layer_result = handle_layer(&context, layer_name.clone(), test_layer).unwrap();

    // Assert layer content metadata, returned and on disk
    let expected_content_layer_metadata = LayerContentMetadata {
        types: Some(LayerTypes {
            launch: TEST_LAYER_LAUNCH,
            build: TEST_LAYER_BUILD,
            cache: TEST_LAYER_CACHE,
        }),
        metadata: TestLayerMetadata {
            // This is the version from a hand-rolled, incompatible metadata TOML file and
            // intentionally not metadata_version_string.
            version: String::from("3.2.1"),
        },
    };

    assert_eq!(
        &handle_layer_result.content_metadata,
        &expected_content_layer_metadata
    );

    let layer_content_metadata_from_disk: LayerContentMetadata<TestLayerMetadata> = read_toml_file(
        temp_dir
            .path()
            .join("layers")
            .join(format!("{layer_name}.toml")),
    )
    .unwrap();

    // Assert basic LayerData fields
    assert_eq!(
        &layer_content_metadata_from_disk,
        &expected_content_layer_metadata
    );

    assert_eq!(handle_layer_result.name, layer_name);
    assert_eq!(
        handle_layer_result.path,
        temp_dir.path().join("layers").join(layer_name.as_str())
    );

    assert_eq!(handle_layer_result.env, LayerEnv::new());

    // Assert layer directory contents
    let create_file_contents =
        fs::read_to_string(handle_layer_result.path.join(TEST_LAYER_CREATE_FILE_NAME)).ok();

    let update_file_contents =
        fs::read_to_string(handle_layer_result.path.join(TEST_LAYER_UPDATE_FILE_NAME)).ok();

    // Since we create the layer by hand without this file, we can assess that the regular create
    // method of the layer has never been called.
    assert_eq!(create_file_contents, None);

    assert_eq!(
        update_file_contents,
        Some(String::from(TEST_LAYER_UPDATE_FILE_CONTENTS))
    );
}

#[test]
fn update_with_incompatible_metadata_recreate() {
    let temp_dir = tempdir().unwrap();
    let context = build_context(&temp_dir);
    let layer_name = random_layer_name();
    let metadata_version_string = String::from("2.3.4");
    let test_layer = TestLayer {
        existing_layer_strategy: ExistingLayerStrategy::Update,
        write_version: metadata_version_string.clone(),
        write_layer_env: None,
    };

    // Create a layer by hand that has incompatible metadata
    let test_layer_dir = temp_dir.path().join("layers").join(layer_name.as_str());
    fs::create_dir_all(test_layer_dir).unwrap();

    let test_layer_toml = temp_dir
        .path()
        .join("layers")
        .join(format!("{layer_name}.toml"));

    fs::write(
        test_layer_toml,
        r#"
[metadata]
versi_on = "3.2.1"
    "#,
    )
    .unwrap();

    let handle_layer_result = handle_layer(&context, layer_name.clone(), test_layer).unwrap();

    // Assert layer content metadata, returned and on disk
    let expected_content_layer_metadata = LayerContentMetadata {
        types: Some(LayerTypes {
            launch: TEST_LAYER_LAUNCH,
            build: TEST_LAYER_BUILD,
            cache: TEST_LAYER_CACHE,
        }),
        metadata: TestLayerMetadata {
            version: metadata_version_string,
        },
    };

    assert_eq!(
        &handle_layer_result.content_metadata,
        &expected_content_layer_metadata
    );

    let layer_content_metadata_from_disk: LayerContentMetadata<TestLayerMetadata> = read_toml_file(
        temp_dir
            .path()
            .join("layers")
            .join(format!("{layer_name}.toml")),
    )
    .unwrap();

    // Assert basic LayerData fields
    assert_eq!(
        &layer_content_metadata_from_disk,
        &expected_content_layer_metadata
    );

    assert_eq!(handle_layer_result.name, layer_name);
    assert_eq!(
        handle_layer_result.path,
        temp_dir.path().join("layers").join(layer_name.as_str())
    );

    assert_eq!(handle_layer_result.env, LayerEnv::new());

    // Assert layer directory contents
    let create_file_contents =
        fs::read_to_string(handle_layer_result.path.join(TEST_LAYER_CREATE_FILE_NAME)).ok();

    let update_file_contents =
        fs::read_to_string(handle_layer_result.path.join(TEST_LAYER_UPDATE_FILE_NAME)).ok();

    assert_eq!(
        create_file_contents,
        Some(String::from(TEST_LAYER_CREATE_FILE_CONTENTS))
    );

    assert_eq!(update_file_contents, None);
}

#[test]
fn error_handling_no_metadata_toml() {
    let temp_dir = tempdir().unwrap();
    let context = build_context(&temp_dir);
    let layer_name = random_layer_name();

    let layer_dir = temp_dir.path().join("layers").join(layer_name.as_str());
    fs::create_dir_all(layer_dir).unwrap();

    let handle_layer_result = handle_layer(&context, layer_name, TestLayer::default()).unwrap();

    let create_file_contents =
        fs::read_to_string(handle_layer_result.path.join(TEST_LAYER_CREATE_FILE_NAME)).ok();

    let update_file_contents =
        fs::read_to_string(handle_layer_result.path.join(TEST_LAYER_UPDATE_FILE_NAME)).ok();

    assert_eq!(
        create_file_contents,
        Some(String::from(TEST_LAYER_CREATE_FILE_CONTENTS))
    );

    assert_eq!(update_file_contents, None);
}

#[test]
fn error_handling_no_directory() {
    let temp_dir = tempdir().unwrap();
    let context = build_context(&temp_dir);
    let layer_name = random_layer_name();

    let layer_toml_path = temp_dir
        .path()
        .join("layers")
        .join(format!("{layer_name}.toml"));

    fs::write(
        layer_toml_path,
        r#"
[metadata]
version = "3.2.1"
    "#,
    )
    .unwrap();

    let handle_layer_result = handle_layer(&context, layer_name, TestLayer::default()).unwrap();

    // We expect the layer to be recreated from scratch. This means that the version from the
    // existing metadata should not be used.
    assert_ne!(
        handle_layer_result.content_metadata.metadata.version,
        "3.2.1"
    );

    let create_file_contents =
        fs::read_to_string(handle_layer_result.path.join(TEST_LAYER_CREATE_FILE_NAME)).ok();

    let update_file_contents =
        fs::read_to_string(handle_layer_result.path.join(TEST_LAYER_UPDATE_FILE_NAME)).ok();

    assert_eq!(
        create_file_contents,
        Some(String::from(TEST_LAYER_CREATE_FILE_CONTENTS))
    );

    assert_eq!(update_file_contents, None);
}

#[test]
fn write_layer_env() {
    let temp_dir = tempdir().unwrap();
    let context = build_context(&temp_dir);
    let layer_name = random_layer_name();
    let metadata_version_string = String::from("1.0.0");
    let layer_env = LayerEnv::new().chainable_insert(
        Scope::All,
        ModificationBehavior::Append,
        "RANDOM",
        "4", // chosen by fair dice roll, guaranteed to be random.
    );

    let test_layer = TestLayer {
        existing_layer_strategy: ExistingLayerStrategy::Keep,
        write_version: metadata_version_string,
        write_layer_env: Some(layer_env.clone()),
    };

    let handle_layer_result = handle_layer(&context, layer_name.clone(), test_layer).unwrap();

    assert_eq!(handle_layer_result.env, layer_env);

    let layer_env_from_disk =
        LayerEnv::read_from_layer_dir(temp_dir.path().join("layers").join(layer_name.as_str()))
            .unwrap();

    assert_eq!(layer_env_from_disk, layer_env);
}

#[test]
fn default_layer_method_implementations() {
    struct SimpleLayer;

    impl Layer for SimpleLayer {
        type Buildpack = TestBuildpack;
        type Metadata = SimpleLayerMetadata;

        fn types(&self) -> LayerTypes {
            LayerTypes {
                launch: false,
                build: false,
                cache: false,
            }
        }

        fn create(
            &self,
            _context: &BuildContext<Self::Buildpack>,
            _layer_path: &Path,
        ) -> Result<LayerResult<Self::Metadata>, <Self::Buildpack as Buildpack>::Error> {
            LayerResultBuilder::new(SimpleLayerMetadata {
                field_one: String::from("value one"),
                field_two: 2,
            })
            .build()
        }
    }

    #[derive(Deserialize, Serialize, Debug, Eq, PartialEq, Clone)]
    struct SimpleLayerMetadata {
        field_one: String,
        field_two: i32,
    }

    let temp_dir = tempdir().unwrap();
    let context = build_context(&temp_dir);
    let layer_name = random_layer_name();
    let simple_layer = SimpleLayer;

    let simple_layer_metadata = SimpleLayerMetadata {
        field_one: String::from("value one"),
        field_two: 2,
    };

    let layer_data = LayerData {
        name: layer_name,
        path: PathBuf::default(),
        env: LayerEnv::new().chainable_insert(
            Scope::All,
            ModificationBehavior::Default,
            "FOO",
            "bar",
        ),
        content_metadata: LayerContentMetadata {
            types: Some(LayerTypes::default()),
            metadata: simple_layer_metadata.clone(),
        },
    };

    // Assert that the default migrate_incompatible_metadata implementation always returns
    // MetadataMigration::RecreateLayer.
    match simple_layer.migrate_incompatible_metadata(&context, &GenericMetadata::default()) {
        Ok(MetadataMigration::RecreateLayer) => {}
        // Since GenericMetadata does not implement PartialEq, we cannot do an assert_eq here
        _ => panic!("Expected Ok(MetadataMigration::RecreateLayer)!"),
    }

    // Assert that the default existing_layer_strategy implementation always returns
    // ExistingLayerStrategy::Recreate.
    assert_eq!(
        simple_layer
            .existing_layer_strategy(&context, &layer_data)
            .unwrap(),
        ExistingLayerStrategy::Recreate
    );

    // Assert that the default update implementation returns both the layer metadata and environment
    // they way they were.
    let update_result = simple_layer.update(&context, &layer_data).unwrap();

    assert_eq!(update_result.env, Some(layer_data.env));
    assert_eq!(update_result.metadata, simple_layer_metadata);
}

#[test]
fn layer_env_read_write() {
    #[derive(Clone)]
    struct LayerDataTestLayer {
        expected_layer_env: LayerEnv,
    }

    impl Layer for LayerDataTestLayer {
        type Buildpack = TestBuildpack;
        type Metadata = GenericMetadata;

        fn types(&self) -> LayerTypes {
            LayerTypes {
                launch: true,
                build: true,
                cache: true,
            }
        }

        fn create(
            &self,
            _context: &BuildContext<Self::Buildpack>,
            _layer_path: &Path,
        ) -> Result<LayerResult<Self::Metadata>, <Self::Buildpack as Buildpack>::Error> {
            LayerResultBuilder::new(GenericMetadata::default())
                .env(self.expected_layer_env.clone())
                .build()
        }

        fn existing_layer_strategy(
            &self,
            _context: &BuildContext<Self::Buildpack>,
            layer_data: &LayerData<Self::Metadata>,
        ) -> Result<ExistingLayerStrategy, <Self::Buildpack as Buildpack>::Error> {
            assert_eq!(&layer_data.env, &self.expected_layer_env);

            Ok(ExistingLayerStrategy::Update)
        }

        fn update(
            &self,
            _context: &BuildContext<Self::Buildpack>,
            layer_data: &LayerData<Self::Metadata>,
        ) -> Result<LayerResult<Self::Metadata>, <Self::Buildpack as Buildpack>::Error> {
            assert_eq!(&layer_data.env, &self.expected_layer_env);

            LayerResultBuilder::new(GenericMetadata::default()).build()
        }
    }
    let temp_dir = tempdir().unwrap();
    let context = build_context(&temp_dir);
    let layer_name = random_layer_name();

    let layer = LayerDataTestLayer {
        expected_layer_env: LayerEnv::new().chainable_insert(
            Scope::All,
            ModificationBehavior::Override,
            "FOO",
            "bar",
        ),
    };

    let handle_layer_result = handle_layer(&context, layer_name.clone(), layer.clone());
    assert!(handle_layer_result.is_ok());

    let handle_layer_result = handle_layer(&context, layer_name, layer);
    assert!(handle_layer_result.is_ok());

    // See the Layer implementation for more asserts
}

fn build_context(temp_dir: &TempDir) -> BuildContext<TestBuildpack> {
    let layers_dir = temp_dir.path().join("layers");
    let app_dir = temp_dir.path().join("app");
    let buildpack_dir = temp_dir.path().join("buildpack");

    fs::create_dir_all(&layers_dir).unwrap();
    fs::create_dir_all(&app_dir).unwrap();
    fs::create_dir_all(&buildpack_dir).unwrap();

    BuildContext {
        layers_dir,
        app_dir,
        buildpack_dir,
        stack_id: stack_id!("heroku-20"),
        platform: GenericPlatform::new(Env::new()),
        buildpack_plan: BuildpackPlan {
            entries: Vec::new(),
        },
        buildpack_descriptor: SingleBuildpackDescriptor {
            api: LIBCNB_SUPPORTED_BUILDPACK_API,
            buildpack: crate::data::buildpack::Buildpack {
                id: buildpack_id!("libcnb/test"),
                name: None,
                version: BuildpackVersion::new(1, 0, 0),
                homepage: None,
                clear_env: true,
                description: None,
                keywords: Vec::new(),
                licenses: Vec::new(),
                sbom_formats: HashSet::new(),
            },
            stacks: vec![Stack::Any],
            metadata: GenericMetadata::default(),
        },
        store: None,
    }
}

fn random_layer_name() -> LayerName {
    repeat_with(fastrand::alphanumeric)
        .take(15)
        .collect::<String>()
        .parse()
        .unwrap()
}

struct TestBuildpack;

impl Buildpack for TestBuildpack {
    type Platform = GenericPlatform;
    type Metadata = GenericMetadata;
    type Error = TestBuildpackError;

    fn detect(&self, _context: DetectContext<Self>) -> crate::Result<DetectResult, Self::Error> {
        DetectResultBuilder::pass().build()
    }

    fn build(&self, _context: BuildContext<Self>) -> crate::Result<BuildResult, Self::Error> {
        BuildResultBuilder::new().build()
    }
}

#[derive(Debug)]
enum TestBuildpackError {
    IoError(std::io::Error),
}
