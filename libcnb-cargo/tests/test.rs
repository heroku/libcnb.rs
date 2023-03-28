use assert_cmd::Command;
use fs_extra::dir::{copy, CopyOptions};
use libcnb_package::read_buildpack_data;
use std::path::{Path, PathBuf};
use std::process::Output;
use tempfile::{tempdir, TempDir};

#[test]
fn package_buildpack_in_single_buildpack_project() {
    let (_tempdir, buildpack_project) = create_buildpack_project_from_fixture("single_buildpack");

    let output = package_project_for_release(&buildpack_project);

    validate_stdout_include_compiled_directories(&buildpack_project, &output, ["single-buildpack"]);
    validate_compiled_buildpack(&buildpack_project, "single-buildpack");
}

#[test]
fn package_single_meta_buildpack_in_monorepo_buildpack_project() {
    let (_tempdir, buildpack_project) =
        create_buildpack_project_from_fixture("multiple_buildpacks");

    let output = package_project_for_release(&buildpack_project.join("meta-buildpacks/meta-one"));

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
    validate_meta_buildpack(&buildpack_project, "multiple-buildpacks/meta-one");
}

#[test]
fn package_single_buildpack_in_monorepo_buildpack_project() {
    let (_tempdir, buildpack_project) =
        create_buildpack_project_from_fixture("multiple_buildpacks");

    let output = package_project_for_release(&buildpack_project.join("buildpacks/one"));

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
    validate_meta_buildpack(&buildpack_project, "multiple-buildpacks/meta-one");
}

fn create_buildpack_project_from_fixture(buildpack_project_fixture: &str) -> (TempDir, PathBuf) {
    let source_directory = std::env::current_dir()
        .unwrap()
        .join("fixtures")
        .join(buildpack_project_fixture);
    let target_directory = tempdir().unwrap();
    let copy_options = CopyOptions::new();
    copy(&source_directory, &target_directory, &copy_options).unwrap();
    let buildpack_project = target_directory.path().join(buildpack_project_fixture);
    (target_directory, buildpack_project)
}

fn package_project_for_release<PathRef: AsRef<Path>>(working_dir: PathRef) -> Output {
    let mut cmd = Command::cargo_bin("cargo-libcnb").unwrap();
    cmd.args(["libcnb", "package", "--release"]);
    cmd.current_dir(working_dir);
    let output = cmd.unwrap();
    println!("STDOUT:\n{}", String::from_utf8_lossy(&*output.stdout));
    println!("STDERR:\n{}", String::from_utf8_lossy(&*output.stderr));
    output
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

    let buildpack_metadata = read_buildpack_data(&target_compile_dir).unwrap();
    assert_eq!(
        buildpack_metadata
            .buildpack_descriptor
            .buildpack
            .id
            .to_string(),
        buildpack_id
    );
}

fn validate_meta_buildpack<PathRef: AsRef<Path>>(buildpack_project: PathRef, buildpack_id: &str) {
    let target_compile_dir = get_compiled_buildpack_directory(buildpack_project, buildpack_id);

    assert!(target_compile_dir.exists());
    assert!(target_compile_dir.join("buildpack.toml").exists());
    assert!(target_compile_dir.join("package.toml").exists());
}

fn get_compiled_buildpack_directory<PathAsRef: AsRef<Path>>(
    buildpack_project: PathAsRef,
    buildpack_id: &str,
) -> PathBuf {
    buildpack_project
        .as_ref()
        .join("target")
        .join("buildpack")
        .join(buildpack_id.replace("/", "_"))
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
    let stdout: Vec<_> = String::from_utf8_lossy(&*output.stdout)
        .lines()
        .map(|line| PathBuf::from(line))
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
