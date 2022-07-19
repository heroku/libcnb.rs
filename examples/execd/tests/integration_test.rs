//! Integration tests using libcnb-test.
//!
//! All integration tests are skipped by default (using the `ignore` attribute),
//! since performing builds is slow. To run the tests use: `cargo test -- --ignored`

// Enable Clippy lints that are disabled by default.
// https://rust-lang.github.io/rust-clippy/stable/index.html
#![warn(clippy::pedantic)]

use libcnb_test::{assert_contains, assert_empty, BuildConfig, TestRunner};

#[test]
#[ignore]
fn basic() {
    TestRunner::default().build(
        BuildConfig::new("heroku/builder:22", "test-fixtures/empty-app"),
        |context| {
            let log_output = context.run_shell_command("env");
            assert_empty!(log_output.stderr);
            assert_contains!(log_output.stdout, "ROLL_1D6=");
            assert_contains!(log_output.stdout, "ROLL_4D6=");
            assert_contains!(log_output.stdout, "ROLL_1D20=");
        },
    );
}
