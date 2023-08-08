use fs_extra::dir::{copy, CopyOptions};
use libcnb_data::buildpack::{BuildpackDescriptor, BuildpackId};
use libcnb_data::buildpack_id;
use libcnb_package::output::BuildpackOutputDirectoryLocator;
use libcnb_package::{read_buildpack_data, read_buildpackage_data, CargoProfile};
use std::env;
use std::io::Read;
use std::path::PathBuf;
use std::process::{Command, Stdio};
use tempfile::{tempdir, TempDir};

#[test]
#[ignore = "integration test"]
fn package_buildpack_in_single_buildpack_project() {
    let packaging_test = BuildpackPackagingTest::new("single_buildpack", X86_64_UNKNOWN_LINUX_MUSL);
    let output = packaging_test.run_libcnb_package();
    assert_eq!(
        output.stdout.trim(),
        [packaging_test.target_dir_name(buildpack_id!("single-buildpack"))].join("\n")
    );
    assert_compiled_buildpack(&packaging_test, buildpack_id!("single-buildpack"));
}

#[test]
#[ignore = "integration test"]
fn package_single_meta_buildpack_in_monorepo_buildpack_project() {
    let packaging_test =
        BuildpackPackagingTest::new("multiple_buildpacks", X86_64_UNKNOWN_LINUX_MUSL);
    let output = packaging_test.run_libcnb_package_from("meta-buildpacks/meta-one");
    assert_eq!(
        output.stdout.trim(),
        [packaging_test.target_dir_name(buildpack_id!("multiple-buildpacks/meta-one"))].join("\n")
    );
    assert_compiled_buildpack(&packaging_test, buildpack_id!("multiple-buildpacks/one"));
    assert_compiled_buildpack(&packaging_test, buildpack_id!("multiple-buildpacks/two"));
    assert_compiled_meta_buildpack(
        &packaging_test,
        buildpack_id!("multiple-buildpacks/meta-one"),
        vec![
            packaging_test.target_dir_name(buildpack_id!("multiple-buildpacks/one")),
            packaging_test.target_dir_name(buildpack_id!("multiple-buildpacks/two")),
            packaging_test
                .dir()
                .join("meta-buildpacks/meta-one/../../buildpacks/not_libcnb")
                .to_string_lossy()
                .to_string(),
            String::from("docker://docker.io/heroku/procfile-cnb:2.0.0"),
        ],
    );
}

#[test]
#[ignore = "integration test"]
fn package_single_buildpack_in_monorepo_buildpack_project() {
    let packaging_test =
        BuildpackPackagingTest::new("multiple_buildpacks", X86_64_UNKNOWN_LINUX_MUSL);
    let output = packaging_test.run_libcnb_package_from("buildpacks/one");
    assert_eq!(
        output.stdout.trim(),
        [packaging_test.target_dir_name(buildpack_id!("multiple-buildpacks/one"))].join("\n")
    );
    assert_compiled_buildpack(&packaging_test, buildpack_id!("multiple-buildpacks/one"));
    assert!(!packaging_test
        .target_dir(buildpack_id!("multiple-buildpacks/two"))
        .exists());
    assert!(!packaging_test
        .target_dir(buildpack_id!("multiple-buildpacks/meta-one"))
        .exists());
}

#[test]
#[ignore = "integration test"]
fn package_all_buildpacks_in_monorepo_buildpack_project() {
    let packaging_test =
        BuildpackPackagingTest::new("multiple_buildpacks", X86_64_UNKNOWN_LINUX_MUSL);
    let output = packaging_test.run_libcnb_package();
    assert_eq!(
        output.stdout.trim(),
        [
            packaging_test
                .dir()
                .join("buildpacks/not_libcnb")
                .to_string_lossy()
                .to_string(),
            packaging_test.target_dir_name(buildpack_id!("multiple-buildpacks/meta-one")),
            packaging_test.target_dir_name(buildpack_id!("multiple-buildpacks/one")),
            packaging_test.target_dir_name(buildpack_id!("multiple-buildpacks/two")),
        ]
        .join("\n")
    );
    assert_compiled_buildpack(&packaging_test, buildpack_id!("multiple-buildpacks/one"));
    assert_compiled_buildpack(&packaging_test, buildpack_id!("multiple-buildpacks/two"));
    assert_compiled_meta_buildpack(
        &packaging_test,
        buildpack_id!("multiple-buildpacks/meta-one"),
        vec![
            packaging_test.target_dir_name(buildpack_id!("multiple-buildpacks/one")),
            packaging_test.target_dir_name(buildpack_id!("multiple-buildpacks/two")),
            packaging_test
                .dir()
                .join("meta-buildpacks/meta-one/../../buildpacks/not_libcnb")
                .to_string_lossy()
                .to_string(),
            String::from("docker://docker.io/heroku/procfile-cnb:2.0.0"),
        ],
    );
}

#[test]
#[ignore = "integration test"]
fn package_non_libcnb_buildpack_in_meta_buildpack_project() {
    let packaging_test =
        BuildpackPackagingTest::new("multiple_buildpacks", X86_64_UNKNOWN_LINUX_MUSL);
    let output = packaging_test.run_libcnb_package_from("buildpacks/not_libcnb");
    assert_eq!(
        output.stdout.trim(),
        [packaging_test
            .dir()
            .join("buildpacks/not_libcnb")
            .to_string_lossy()
            .to_string()]
        .join("\n")
    );
    assert!(!packaging_test
        .target_dir(buildpack_id!("multiple-buildpacks/one"))
        .exists());
    assert!(!packaging_test
        .target_dir(buildpack_id!("multiple-buildpacks/two"))
        .exists());
    assert!(!packaging_test
        .target_dir(buildpack_id!("multiple-buildpacks/meta-one"))
        .exists());
}

#[test]
#[ignore = "integration test"]
fn package_command_error_when_run_in_project_with_no_buildpacks() {
    let packaging_test = BuildpackPackagingTest::new("no_buildpacks", X86_64_UNKNOWN_LINUX_MUSL);
    let output = packaging_test.run_libcnb_package();
    assert_ne!(output.code, Some(0));
    assert_eq!(
        output.stderr,
        "Determining automatic cross-compile settings...\n🔍 Locating buildpacks...\n❌ No buildpacks found!\n"
    );
}

fn assert_compiled_buildpack(packaging_test: &BuildpackPackagingTest, buildpack_id: BuildpackId) {
    let buildpack_target_dir = PathBuf::from(packaging_test.target_dir_name(buildpack_id.clone()));

    assert!(buildpack_target_dir.exists());
    assert!(buildpack_target_dir.join("buildpack.toml").exists());
    assert!(buildpack_target_dir.join("package.toml").exists());
    assert!(buildpack_target_dir.join("bin").join("build").exists());
    assert!(buildpack_target_dir.join("bin").join("detect").exists());

    let buildpack_data = read_buildpack_data(&buildpack_target_dir).unwrap();
    let id = match buildpack_data.buildpack_descriptor {
        BuildpackDescriptor::Single(descriptor) => descriptor.buildpack.id,
        BuildpackDescriptor::Meta(descriptor) => descriptor.buildpack.id,
    };
    assert_eq!(id, buildpack_id);
}

fn assert_compiled_meta_buildpack(
    packaging_test: &BuildpackPackagingTest,
    buildpack_id: BuildpackId,
    dependencies: Vec<String>,
) {
    let buildpack_target_dir = PathBuf::from(packaging_test.target_dir_name(buildpack_id.clone()));

    assert!(buildpack_target_dir.exists());
    assert!(buildpack_target_dir.join("buildpack.toml").exists());
    assert!(buildpack_target_dir.join("package.toml").exists());

    let buildpack_data = read_buildpack_data(&buildpack_target_dir).unwrap();
    let id = match buildpack_data.buildpack_descriptor {
        BuildpackDescriptor::Single(descriptor) => descriptor.buildpack.id,
        BuildpackDescriptor::Meta(descriptor) => descriptor.buildpack.id,
    };
    assert_eq!(id, buildpack_id);

    let buildpackage_data = read_buildpackage_data(buildpack_target_dir)
        .unwrap()
        .unwrap();
    let compiled_uris: Vec<_> = buildpackage_data
        .buildpackage_descriptor
        .dependencies
        .iter()
        .map(|buildpackage_uri| buildpackage_uri.uri.to_string())
        .collect();
    assert_eq!(compiled_uris, dependencies);
}

struct BuildpackPackagingTest {
    fixture_name: String,
    temp_dir: TempDir,
    release_build: bool,
    target_triple: String,
}

struct TestOutput {
    stdout: String,
    stderr: String,
    code: Option<i32>,
}

impl BuildpackPackagingTest {
    fn new(fixture_name: &str, target_triple: &str) -> Self {
        let source_directory = env::current_dir()
            .unwrap()
            .join("fixtures")
            .join(fixture_name);
        let target_directory = tempdir().unwrap();
        let copy_options = CopyOptions::new();
        copy(source_directory, &target_directory, &copy_options).unwrap();
        BuildpackPackagingTest {
            fixture_name: fixture_name.to_string(),
            temp_dir: target_directory,
            release_build: true,
            target_triple: String::from(target_triple),
        }
    }

    fn dir(&self) -> PathBuf {
        self.temp_dir
            .path()
            .canonicalize()
            .unwrap()
            .join(&self.fixture_name)
    }

    fn target_dir_name(&self, buildpack_id: BuildpackId) -> String {
        self.target_dir(buildpack_id)
            .canonicalize()
            .unwrap()
            .to_string_lossy()
            .to_string()
    }

    fn target_dir(&self, buildpack_id: BuildpackId) -> PathBuf {
        let root_dir = self.dir().join("target");
        let cargo_profile = if self.release_build {
            CargoProfile::Release
        } else {
            CargoProfile::Dev
        };
        let locator = BuildpackOutputDirectoryLocator::new(
            root_dir,
            cargo_profile,
            self.target_triple.clone(),
        );
        locator.get(&buildpack_id)
    }

    fn run_libcnb_package(&self) -> TestOutput {
        self.run_libcnb_package_from(".")
    }

    fn run_libcnb_package_from(&self, from_dir: &str) -> TestOutput {
        // borrowed from assert_cmd
        let suffix = env::consts::EXE_SUFFIX;

        let target_dir = env::current_exe()
            .ok()
            .map(|mut path| {
                path.pop();
                if path.ends_with("deps") {
                    path.pop();
                }
                path
            })
            .unwrap();

        let name = "cargo-libcnb";

        let cargo_libcnb = env::var_os(format!("CARGO_BIN_EXE_{name}"))
            .map(|p| p.into())
            .unwrap_or_else(|| target_dir.join(format!("{name}{suffix}")));

        let mut cmd = Command::new(cargo_libcnb)
            .args(["libcnb", "package", "--release"])
            .current_dir(self.dir().join(from_dir))
            .stdout(Stdio::piped())
            .stderr(Stdio::piped())
            .spawn()
            .unwrap();

        let status = cmd.wait().unwrap();

        let mut stdout = String::new();
        cmd.stdout
            .take()
            .unwrap()
            .read_to_string(&mut stdout)
            .unwrap();
        println!("STDOUT:\n{stdout}");

        let mut stderr = String::new();
        cmd.stderr
            .take()
            .unwrap()
            .read_to_string(&mut stderr)
            .unwrap();
        println!("STDERR:\n{stderr}");

        TestOutput {
            stdout: String::from_utf8_lossy(stdout.as_bytes()).to_string(),
            stderr: String::from_utf8_lossy(stderr.as_bytes()).to_string(),
            code: status.code(),
        }
    }
}

const X86_64_UNKNOWN_LINUX_MUSL: &str = "x86_64-unknown-linux-musl";
