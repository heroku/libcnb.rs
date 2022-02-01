use libcnb_test::{BuildpackReference, IntegrationTest};
use tempfile::tempdir;

#[test]
#[should_panic(expected = "pack command failed with exit-code 1!

pack stdout:


pack stderr:
ERROR: failed to build: failed to fetch builder image 'index.docker.io/libcnb/void-builder:doesntexist': image 'index.docker.io/libcnb/void-builder:doesntexist' does not exist on the daemon: not found
")]
fn panic_on_unsuccessful_pack_run() {
    let temp_app_dir = tempdir().unwrap();
    IntegrationTest::new("libcnb/void-builder:doesntexist", temp_app_dir.path())
        .buildpacks(vec![BuildpackReference::Other(String::from("libcnb/void"))])
        .run(|_| {});
}
