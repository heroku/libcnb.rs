//! Integration tests using libcnb-test.
//!
//! All integration tests are skipped by default (using the `ignore` attribute),
//! since performing builds is slow. To run the tests use: `cargo test -- --ignored`

use libcnb_test::IntegrationTest;
use std::io;
use std::io::{Read, Write};
use std::net;
use std::net::ToSocketAddrs;
use std::time::Duration;

#[test]
#[ignore]
fn basic() {
    IntegrationTest::new("heroku/buildpacks:20", "test-fixtures/simple-ruby-app").run_test(
        |context| {
            assert!(context.pack_stdout.contains("---> Ruby Buildpack"));
            assert!(context.pack_stdout.contains("---> Installing bundler"));
            assert!(context.pack_stdout.contains("---> Installing gems"));

            context.start_container(&[12345], |container| {
                std::thread::sleep(Duration::from_secs(1));

                assert_eq!(
                    call_test_fixture_service(
                        container.address_for_port(12345).unwrap(),
                        "Hello World!"
                    )
                    .unwrap(),
                    "!dlroW olleH"
                );

                assert!(container
                    .shell_exec("ruby --version")
                    .stdout
                    .contains("ruby 2.7.0p0"));
            });
        },
    );
}

fn call_test_fixture_service<A>(a: A, payload: impl AsRef<str>) -> io::Result<String>
where
    A: ToSocketAddrs,
{
    let mut stream = net::TcpStream::connect(a)?;

    stream.write_all(format!("{}\n", payload.as_ref()).as_bytes())?;

    let mut buffer = vec![];
    stream.read_to_end(&mut buffer)?;

    Ok(format!("{}", String::from_utf8_lossy(&buffer)))
}
