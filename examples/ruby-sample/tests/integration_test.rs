//! All integration tests are skipped by default (using the `ignore` attribute)
//! since performing builds is slow. To run them use: `cargo test -- --ignored`.

// Enable Clippy lints that are disabled by default.
// https://rust-lang.github.io/rust-clippy/stable/index.html
#![warn(clippy::pedantic)]

use libcnb_test::{
    assert_contains, assert_not_contains, BuildConfig, ContainerConfig, PackResult, TestRunner,
};
use std::io::{Read, Write};
use std::net;
use std::net::ToSocketAddrs;
use std::time::Duration;
use std::{fs, io};

#[test]
#[ignore = "integration test"]
fn basic() {
    let config = BuildConfig::new("heroku/builder:20", "tests/fixtures/simple-ruby-app");

    TestRunner::default().build(&config, |context| {
        assert_contains!(context.pack_stdout, "---> Ruby Buildpack");
        assert_contains!(context.pack_stdout, "---> Installing bundler");
        assert_contains!(context.pack_stdout, "---> Installing gems");

        context.start_container(
            ContainerConfig::new()
                .env("PORT", TEST_PORT.to_string())
                .expose_port(TEST_PORT),
            |container| {
                std::thread::sleep(Duration::from_secs(2));

                assert_eq!(
                    call_test_fixture_service(
                        container.address_for_port(TEST_PORT),
                        "Hello World!"
                    )
                    .unwrap(),
                    "!dlroW olleH"
                );
            },
        );

        assert_contains!(
            context.run_shell_command("ruby --version").stdout,
            "ruby 2.7.0p0"
        );

        context.rebuild(&config, |context| {
            assert_not_contains!(context.pack_stdout, "---> Installing bundler");
            assert_not_contains!(context.pack_stdout, "---> Installing gems");
        });
    });
}

#[test]
#[ignore = "integration test"]
fn missing_gemfile_lock() {
    TestRunner::default().build(
        BuildConfig::new("heroku/builder:20", "tests/fixtures/simple-ruby-app")
            .app_dir_preprocessor(|path| fs::remove_file(path.join("Gemfile.lock")).unwrap())
            .expected_pack_result(PackResult::Failure),
        |context| {
            assert_contains!(
                context.pack_stdout,
                "ERROR: No buildpack groups passed detection."
            );
        },
    );
}

fn call_test_fixture_service<A>(a: A, payload: impl AsRef<str>) -> io::Result<String>
where
    A: ToSocketAddrs,
{
    let mut stream = net::TcpStream::connect(a)?;

    stream.write_all(format!("{}\n", payload.as_ref()).as_bytes())?;

    let mut buffer = Vec::new();
    stream.read_to_end(&mut buffer)?;

    Ok(String::from_utf8_lossy(&buffer).to_string())
}

const TEST_PORT: u16 = 12346;
