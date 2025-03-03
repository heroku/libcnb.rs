// Required due to: https://github.com/rust-lang/rust/issues/95513
#![allow(unused_crate_dependencies)]

use libcnb_test::{BuildConfig, BuildpackReference, TestRunner, assert_contains};
use std::env::temp_dir;
use std::fs;

#[test]
#[ignore = "integration test"]
fn test_tracing_export_file() {
    let empty_app_dir = temp_dir().join("empty-app-dir");
    fs::create_dir_all(&empty_app_dir).unwrap();

    let mut build_config = BuildConfig::new("heroku/builder:22", &empty_app_dir);

    // Telemetry file exports are not persisted to the build's resulting image,
    // so to test that contents are emitted, a second buildpack is used to read
    // the contents during the build.
    build_config.buildpacks([
        BuildpackReference::CurrentCrate,
        BuildpackReference::Other(format!(
            "file://{}/tests/fixtures/buildpacks/tracing-reader",
            env!("CARGO_MANIFEST_DIR")
        )),
    ]);

    TestRunner::default().build(&build_config, |context| {
        // Ensure expected span names for detect and build phases are present
        // in the file export contents.
        assert_contains!(context.pack_stdout, "libcnb-detect");
        assert_contains!(context.pack_stdout, "libcnb-build");
    });
}
