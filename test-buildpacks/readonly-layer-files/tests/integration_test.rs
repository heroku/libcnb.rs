//! Integration tests using libcnb-test.
//!
//! All integration tests are skipped by default (using the `ignore` attribute),
//! since performing builds is slow. To run the tests use: `cargo test -- --ignored`

// Enable Clippy lints that are disabled by default.
// https://rust-lang.github.io/rust-clippy/stable/index.html
#![warn(clippy::pedantic)]

use libcnb_test::{BuildConfig, TestRunner};
use std::env::temp_dir;

use std::fs;

#[test]
#[ignore = "integration test"]
fn test() {
    let empty_app_dir = temp_dir().join("empty-app-dir");
    fs::create_dir_all(&empty_app_dir).unwrap();

    let build_config = BuildConfig::new("heroku/builder:22", &empty_app_dir);

    // The test succeeds if we're able to build with a cached layer that has directories with
    // problematic permissions inside it (See the buildpack for details).
    TestRunner::default().build(&build_config, |context| {
        context.rebuild(&build_config, |_| {});
    });
}
