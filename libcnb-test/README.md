# libcnb-test

An experimental integration testing framework for Cloud Native Buildpacks written in Rust with libcnb.rs.

## Experimental

This crate is marked as experimental. It currently implements the most basic building blocks for writing
integration tests with libcnb.rs. Its feature set is deliberately cut down to get ball rolling and get a better feel
which features are required. See [issues tagged with `libcnb-test`][libcnb-test-label] for possible future improvements.
Please use the same tag for feature requests.

[libcnb-test-label]: https://github.com/Malax/libcnb.rs/labels/libcnb-test

## Example

```rust,no_run
// In $CRATE_ROOT/test/integration_test.rs
use libcnb_test::{IntegrationTest, BuildpackReference};

#[test]
fn test() {
    IntegrationTest::new("heroku/buildpacks:20", "test-fixtures/app")
        .buildpacks(vec![
            BuildpackReference::Other(String::from("heroku/openjdk")),
            BuildpackReference::Crate,
        ])
        .run_test(|context| {
            assert!(context.pack_stdout.contains("---> Maven Buildpack"));
            assert!(context.pack_stdout.contains("---> Installing Maven"));
            assert!(context.pack_stdout.contains("---> Running mvn package"));

            context.start_container(&[12345], |container| {
                assert_eq!(
                    call_test_fixture_service(
                        container.address_for_port(12345).unwrap(),
                        "Hagbard Celine"
                    )
                    .unwrap(),
                    "enileC drabgaH"
                );
            });
        });
}

fn call_test_fixture_service(addr: std::net::SocketAddr, payload: &str) -> Result<String, ()> {
   unimplemented!()
}
```

## Configuration

### Main buildpack binary

A buildpack crate can contain multiple binaries. To determine which binary is the main buildpack binary, the convention is:
1. If there is only one binary, use that.
2. If there are multiple, use the one that has the same name as the crate.

If this convention does not work for your buildpack, you can explicitly configure the main buildpack binary in `Cargo.toml`:

```toml
[package]
name = "my-buildpack"
version = "1.0.0"
edition = "2021"
rust-version = "1.56"

[package.metadata.libcnb]
buildpack-target = "buildpack-bin-target-name"

[dependencies]
# ...
```
