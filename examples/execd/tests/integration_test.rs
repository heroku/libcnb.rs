//! All integration tests are skipped by default (using the `ignore` attribute)
//! since performing builds is slow. To run them use: `cargo test -- --ignored`.

// Required due to: https://github.com/rust-lang/rust/issues/95513
#![allow(unused_crate_dependencies)]

use libcnb_test::{assert_contains, assert_empty, BuildConfig, TestRunner};

#[test]
#[ignore = "integration test"]
fn basic() {
    TestRunner::default().build(
        BuildConfig::new("heroku/builder:22", "tests/fixtures/empty-app"),
        |context| {
            let command_output = context.run_shell_command("env");
            assert_empty!(command_output.stderr);
            assert_contains!(command_output.stdout, "ROLL_1D6=");
            assert_contains!(command_output.stdout, "ROLL_4D6=");
            assert_contains!(command_output.stdout, "ROLL_1D20=");
        },
    );
}
