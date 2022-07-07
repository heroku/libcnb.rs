# libcnb-test &emsp; [![Docs]][docs.rs] [![Latest Version]][crates.io] [![MSRV]][install-rust]

An experimental integration testing framework for Cloud Native Buildpacks written in Rust with libcnb.rs.

## Experimental

This crate is marked as experimental. It currently implements the most basic building blocks for writing
integration tests with libcnb.rs. Its feature set is deliberately cut down to get ball rolling and get a better feel
which features are required. See [issues tagged with `libcnb-test`][libcnb-test-label] for possible future improvements.
Please use the same tag for feature requests.

[libcnb-test-label]: https://github.com/heroku/libcnb.rs/labels/libcnb-test

## Example

```rust,no_run
// In $CRATE_ROOT/tests/integration_test.rs
use libcnb_test::{assert_contains, TestConfig, TestRunner};

// In your code you'll want to mark your function as a test with `#[test]`.
// It is removed here for compatibility with doctest so this code in the readme
// tests for compilation.
fn test() {
    TestRunner::default().run_test(
        TestConfig::new("heroku/builder:22", "test-fixtures/app"),
        |context| {
            assert_contains!(context.pack_stdout, "---> Maven Buildpack");
            assert_contains!(context.pack_stdout, "---> Installing Maven");
            assert_contains!(context.pack_stdout, "---> Running mvn package");

            context
                .prepare_container()
                .expose_port(12345)
                .start_with_default_process(|container| {
                    assert_eq!(
                        call_test_fixture_service(
                            container.address_for_port(12345).unwrap(),
                            "Hagbard Celine"
                        )
                        .unwrap(),
                        "enileC drabgaH"
                    );
                });
        },
    );
}

fn call_test_fixture_service(addr: std::net::SocketAddr, payload: &str) -> Result<String, ()> {
    unimplemented!()
}
```

## Known issues

- Only local Docker daemons are fully supported. If using Circle CI you must use the
  [`machine` executor](https://circleci.com/docs/2.0/executor-types/#using-machine) rather
  than the [remote docker](https://circleci.com/docs/2.0/building-docker-images/) feature.

[Docs]: https://img.shields.io/docsrs/libcnb-test
[docs.rs]: https://docs.rs/libcnb-test/latest/libcnb_test/
[Latest Version]: https://img.shields.io/crates/v/libcnb-test.svg
[crates.io]: https://crates.io/crates/libcnb-test
[MSRV]: https://img.shields.io/badge/MSRV-rustc_1.59+-lightgray.svg
[install-rust]: https://www.rust-lang.org/tools/install
