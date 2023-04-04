use fs_extra::dir::{copy, CopyOptions};
use libcnb_data::buildpack::BuildpackDescriptor;
use libcnb_package::{read_buildpack_data, read_buildpackage_data};
use std::env;
use std::fs::canonicalize;
use std::io::Read;
use std::path::{Path, PathBuf};
use std::process::{Command, Output, Stdio};
use tempfile::{tempdir, TempDir};

#[test]
#[ignore = "integration test"]
fn package_buildpack_in_single_buildpack_project() {
    let (_tempdir, buildpack_project) = create_buildpack_project_from_fixture("single_buildpack");

    let output = package_project_for_release(&buildpack_project);

    validate_stdout_include_compiled_directories(&buildpack_project, &output, ["single-buildpack"]);
    validate_compiled_buildpack(&buildpack_project, "single-buildpack");
}

#[test]
#[ignore = "integration test"]
fn package_single_meta_buildpack_in_monorepo_buildpack_project() {
    let (_tempdir, buildpack_project) =
        create_buildpack_project_from_fixture("multiple_buildpacks");

    let output = package_project_for_release(buildpack_project.join("meta-buildpacks/meta-one"));

    validate_stdout_include_compiled_directories(
        &buildpack_project,
        &output,
        ["multiple-buildpacks/meta-one"],
    );
    validate_compiled_buildpack(&buildpack_project, "multiple-buildpacks/one");
    validate_compiled_buildpack(&buildpack_project, "multiple-buildpacks/two");
    validate_meta_buildpack(
        &buildpack_project,
        "multiple-buildpacks/meta-one",
        [
            get_compiled_buildpack_directory(&buildpack_project, "multiple-buildpacks/one")
                .to_string_lossy(),
            get_compiled_buildpack_directory(&buildpack_project, "multiple-buildpacks/two")
                .to_string_lossy(),
            String::from("docker://docker.io/heroku/procfile-cnb:2.0.0").into(),
        ],
    );
}

#[test]
#[ignore = "integration test"]
fn package_single_buildpack_in_monorepo_buildpack_project() {
    let (_tempdir, buildpack_project) =
        create_buildpack_project_from_fixture("multiple_buildpacks");

    let output = package_project_for_release(buildpack_project.join("buildpacks/one"));

    validate_stdout_include_compiled_directories(
        &buildpack_project,
        &output,
        ["multiple-buildpacks/one"],
    );
    validate_compiled_buildpack(&buildpack_project, "multiple-buildpacks/one");
    assert!(
        !get_compiled_buildpack_directory(&buildpack_project, "multiple-buildpacks/two").exists()
    );
    assert!(
        !get_compiled_buildpack_directory(&buildpack_project, "multiple-buildpacks/meta-one")
            .exists()
    );
}

#[test]
#[ignore = "integration test"]
fn package_all_buildpacks_in_monorepo_buildpack_project() {
    let (_tempdir, buildpack_project) =
        create_buildpack_project_from_fixture("multiple_buildpacks");

    let output = package_project_for_release(&buildpack_project);

    validate_stdout_include_compiled_directories(
        &buildpack_project,
        &output,
        [
            "multiple-buildpacks/one",
            "multiple-buildpacks/two",
            "multiple-buildpacks/meta-one",
        ],
    );
    validate_compiled_buildpack(&buildpack_project, "multiple-buildpacks/one");
    validate_compiled_buildpack(&buildpack_project, "multiple-buildpacks/two");
    validate_meta_buildpack(
        &buildpack_project,
        "multiple-buildpacks/meta-one",
        [
            get_compiled_buildpack_directory(&buildpack_project, "multiple-buildpacks/one")
                .to_string_lossy(),
            get_compiled_buildpack_directory(&buildpack_project, "multiple-buildpacks/two")
                .to_string_lossy(),
            String::from("docker://docker.io/heroku/procfile-cnb:2.0.0").into(),
        ],
    );
}

fn create_buildpack_project_from_fixture(buildpack_project_fixture: &str) -> (TempDir, PathBuf) {
    let source_directory = env::current_dir()
        .unwrap()
        .join("fixtures")
        .join(buildpack_project_fixture);
    let target_directory = tempdir().unwrap();
    let copy_options = CopyOptions::new();
    copy(source_directory, &target_directory, &copy_options).unwrap();
    let buildpack_project = target_directory.path().join(buildpack_project_fixture);
    (target_directory, buildpack_project)
}

fn package_project_for_release<PathRef: AsRef<Path>>(working_dir: PathRef) -> Output {
    let mut cmd = Command::new(cargo_bin("cargo-libcnb"))
        .args(["libcnb", "package", "--release"])
        .current_dir(working_dir)
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

    Output {
        status,
        stdout: stdout.as_bytes().to_vec(),
        stderr: stderr.as_bytes().to_vec(),
    }
}

fn cargo_bin(name: &str) -> PathBuf {
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

    env::var_os(format!("CARGO_BIN_EXE_{name}"))
        .map(|p| p.into())
        .unwrap_or_else(|| target_dir.join(format!("{name}{suffix}")))
}

fn validate_compiled_buildpack<PathRef: AsRef<Path>>(
    buildpack_project: PathRef,
    buildpack_id: &str,
) {
    let target_compile_dir = get_compiled_buildpack_directory(buildpack_project, buildpack_id);

    assert!(target_compile_dir.exists());
    assert!(target_compile_dir.join("buildpack.toml").exists());
    assert!(target_compile_dir.join("bin").join("build").exists());
    assert!(target_compile_dir.join("bin").join("detect").exists());

    let buildpack_data = read_buildpack_data(&target_compile_dir).unwrap();
    let id = match buildpack_data.buildpack_descriptor {
        BuildpackDescriptor::Single(descriptor) => descriptor.buildpack.id,
        BuildpackDescriptor::Meta(descriptor) => descriptor.buildpack.id,
    };
    assert_eq!(id.to_string(), buildpack_id);
}

fn validate_meta_buildpack<
    PathRef: AsRef<Path>,
    IntoStringIterator: IntoIterator<Item = IntoString>,
    IntoString: Into<String>,
>(
    buildpack_project: PathRef,
    buildpack_id: &str,
    dependency_uris: IntoStringIterator,
) {
    let target_compile_dir = get_compiled_buildpack_directory(buildpack_project, buildpack_id);

    assert!(target_compile_dir.exists());
    assert!(target_compile_dir.join("buildpack.toml").exists());
    assert!(target_compile_dir.join("package.toml").exists());

    let buildpackage_data = read_buildpackage_data(target_compile_dir).unwrap();
    let compiled_uris: Vec<_> = buildpackage_data
        .buildpackage_descriptor
        .dependencies
        .iter()
        .map(|buildpackage_uri| buildpackage_uri.uri.clone())
        .collect();
    let dependency_uris: Vec<_> = dependency_uris
        .into_iter()
        .map(|dependency_uri| {
            let dependency_uri: String = dependency_uri.into();
            if dependency_uri.starts_with('/') {
                let absolute_path = canonicalize(PathBuf::from(dependency_uri)).unwrap();
                String::from(absolute_path.to_string_lossy())
            } else {
                dependency_uri
            }
        })
        .collect();
    assert_eq!(compiled_uris, dependency_uris);
}

fn get_compiled_buildpack_directory<PathAsRef: AsRef<Path>>(
    buildpack_project: PathAsRef,
    buildpack_id: &str,
) -> PathBuf {
    buildpack_project
        .as_ref()
        .join("target")
        .join("buildpack")
        .join("release")
        .join(buildpack_id.replace('/', "_"))
}

fn validate_stdout_include_compiled_directories<
    PathAsRef: AsRef<Path>,
    IntoStringIterator: IntoIterator<Item = IntoString>,
    IntoString: Into<String>,
>(
    buildpack_project: PathAsRef,
    output: &Output,
    buildpack_ids: IntoStringIterator,
) {
    let stdout: Vec<_> = String::from_utf8_lossy(&output.stdout)
        .lines()
        .map(PathBuf::from)
        .collect();
    let targets: Vec<_> = buildpack_ids
        .into_iter()
        .map(|item| {
            get_compiled_buildpack_directory(&buildpack_project, &item.into())
                .canonicalize()
                .unwrap()
        })
        .collect();
    assert_eq!(stdout, targets);
}
