use libcnb_test::IntegrationTest;

#[test]
fn test() {
    let temp_dir = tempfile::tempdir().unwrap();
    IntegrationTest::new("heroku/buildpacks:20", temp_dir.path()).run_test(|context| {
        context.start_container(&[], |context| {
            let output = context.shell_exec("env");

            assert!(output.stdout.contains("FOO=bar"));
            assert!(output.stdout.contains("BAR=baz"));
            assert!(output.stdout.contains("HELLO=world"));
        })
    })
}
