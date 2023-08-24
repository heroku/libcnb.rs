// Enable Clippy lints that are disabled by default.
// https://rust-lang.github.io/rust-clippy/stable/index.html
#![warn(clippy::pedantic)]

use libcnb_data::buildpack::BuildpackId;
use libcnb_data::buildpack_id;
use libcnb_data::buildpackage::PackageDescriptorDependency;
use libcnb_package::output::create_packaged_buildpack_dir_resolver;
use libcnb_package::{read_buildpack_data, read_package_descriptor_data, CargoProfile};
use std::io::ErrorKind;
use std::path::{Path, PathBuf};
use std::process::Command;
use std::{env, fs};
use tempfile::{tempdir_in, TempDir};

#[test]
#[ignore = "integration test"]
fn package_buildpack_in_single_buildpack_project() {
    let fixture_dir = copy_fixture_to_temp_dir("single_buildpack").unwrap();
    let buildpack_id = buildpack_id!("single-buildpack");

    let output = Command::new(CARGO_LIBCNB_BINARY_UNDER_TEST)
        .args(["libcnb", "package", "--release"])
        .current_dir(&fixture_dir)
        .output()
        .unwrap();

    let packaged_buildpack_dir = create_packaged_buildpack_dir_resolver(
        &fixture_dir.path().join(DEFAULT_PACKAGE_DIR_NAME),
        CargoProfile::Release,
        X86_64_UNKNOWN_LINUX_MUSL,
    )(&buildpack_id);

    assert_eq!(
        String::from_utf8_lossy(&output.stdout),
        format!("{}\n", packaged_buildpack_dir.to_string_lossy())
    );

    validate_packaged_buildpack(&packaged_buildpack_dir, &buildpack_id);
}

#[test]
#[ignore = "integration test"]
fn package_single_meta_buildpack_in_monorepo_buildpack_project() {
    let fixture_dir = copy_fixture_to_temp_dir("multiple_buildpacks").unwrap();

    let output = Command::new(CARGO_LIBCNB_BINARY_UNDER_TEST)
        .args(["libcnb", "package", "--release"])
        .current_dir(fixture_dir.path().join("meta-buildpacks").join("meta-one"))
        .output()
        .unwrap();

    let packaged_buildpack_dir_resolver = create_packaged_buildpack_dir_resolver(
        &fixture_dir.path().join(DEFAULT_PACKAGE_DIR_NAME),
        CargoProfile::Release,
        X86_64_UNKNOWN_LINUX_MUSL,
    );

    assert_eq!(
        String::from_utf8_lossy(&output.stdout),
        format!(
            "{}\n",
            packaged_buildpack_dir_resolver(&buildpack_id!("multiple-buildpacks/meta-one"))
                .to_string_lossy()
        )
    );

    validate_packaged_meta_buildpack(
        &packaged_buildpack_dir_resolver(&buildpack_id!("multiple-buildpacks/meta-one")),
        &buildpack_id!("multiple-buildpacks/meta-one"),
        &[
            PackageDescriptorDependency::try_from(packaged_buildpack_dir_resolver(&buildpack_id!(
                "multiple-buildpacks/one"
            ))),
            PackageDescriptorDependency::try_from(packaged_buildpack_dir_resolver(&buildpack_id!(
                "multiple-buildpacks/two"
            ))),
            PackageDescriptorDependency::try_from(
                fixture_dir
                    .path()
                    .join("meta-buildpacks/meta-one/../../buildpacks/not_libcnb"),
            ),
            PackageDescriptorDependency::try_from("docker://docker.io/heroku/example:1.2.3"),
        ]
        .into_iter()
        .collect::<Result<Vec<_>, _>>()
        .unwrap(),
    );

    for buildpack_id in [
        buildpack_id!("multiple-buildpacks/one"),
        buildpack_id!("multiple-buildpacks/two"),
    ] {
        validate_packaged_buildpack(
            &packaged_buildpack_dir_resolver(&buildpack_id),
            &buildpack_id,
        );
    }
}

#[test]
#[ignore = "integration test"]
fn package_single_buildpack_in_monorepo_buildpack_project() {
    let fixture_dir = copy_fixture_to_temp_dir("multiple_buildpacks").unwrap();
    let buildpack_id = buildpack_id!("multiple-buildpacks/one");

    let output = Command::new(CARGO_LIBCNB_BINARY_UNDER_TEST)
        .args(["libcnb", "package", "--release"])
        .current_dir(fixture_dir.path().join("buildpacks/one"))
        .output()
        .unwrap();

    let packaged_buildpack_dir = create_packaged_buildpack_dir_resolver(
        &fixture_dir.path().join(DEFAULT_PACKAGE_DIR_NAME),
        CargoProfile::Release,
        X86_64_UNKNOWN_LINUX_MUSL,
    )(&buildpack_id);

    assert_eq!(
        String::from_utf8_lossy(&output.stdout),
        format!("{}\n", packaged_buildpack_dir.to_string_lossy())
    );

    validate_packaged_buildpack(&packaged_buildpack_dir, &buildpack_id);
}

#[test]
#[ignore = "integration test"]
fn package_all_buildpacks_in_monorepo_buildpack_project() {
    let fixture_dir = copy_fixture_to_temp_dir("multiple_buildpacks").unwrap();

    let dependent_buildpack_ids = [
        buildpack_id!("multiple-buildpacks/one"),
        buildpack_id!("multiple-buildpacks/two"),
    ];

    let output = Command::new(CARGO_LIBCNB_BINARY_UNDER_TEST)
        .args(["libcnb", "package", "--release"])
        .current_dir(&fixture_dir)
        .output()
        .unwrap();

    let packaged_buildpack_dir_resolver = create_packaged_buildpack_dir_resolver(
        &fixture_dir.path().join(DEFAULT_PACKAGE_DIR_NAME),
        CargoProfile::Release,
        X86_64_UNKNOWN_LINUX_MUSL,
    );

    assert_eq!(
        String::from_utf8_lossy(&output.stdout),
        format!(
            "{}\n",
            [
                fixture_dir.path().join("buildpacks/not_libcnb"),
                packaged_buildpack_dir_resolver(&buildpack_id!("multiple-buildpacks/meta-one")),
                packaged_buildpack_dir_resolver(&buildpack_id!("multiple-buildpacks/one")),
                packaged_buildpack_dir_resolver(&buildpack_id!("multiple-buildpacks/two"))
            ]
            .map(|path| path.to_string_lossy().into_owned())
            .join("\n")
        )
    );

    validate_packaged_meta_buildpack(
        &packaged_buildpack_dir_resolver(&buildpack_id!("multiple-buildpacks/meta-one")),
        &buildpack_id!("multiple-buildpacks/meta-one"),
        &[
            PackageDescriptorDependency::try_from(packaged_buildpack_dir_resolver(&buildpack_id!(
                "multiple-buildpacks/one"
            ))),
            PackageDescriptorDependency::try_from(packaged_buildpack_dir_resolver(&buildpack_id!(
                "multiple-buildpacks/two"
            ))),
            PackageDescriptorDependency::try_from(
                fixture_dir
                    .path()
                    .join("meta-buildpacks/meta-one/../../buildpacks/not_libcnb"),
            ),
            PackageDescriptorDependency::try_from("docker://docker.io/heroku/example:1.2.3"),
        ]
        .into_iter()
        .collect::<Result<Vec<_>, _>>()
        .unwrap(),
    );

    for buildpack_id in dependent_buildpack_ids {
        validate_packaged_buildpack(
            &packaged_buildpack_dir_resolver(&buildpack_id),
            &buildpack_id,
        );
    }
}

#[test]
#[ignore = "integration test"]
fn package_non_libcnb_buildpack_in_meta_buildpack_project() {
    let fixture_dir = copy_fixture_to_temp_dir("multiple_buildpacks").unwrap();

    let output = Command::new(CARGO_LIBCNB_BINARY_UNDER_TEST)
        .args(["libcnb", "package", "--release"])
        .current_dir(fixture_dir.path().join("buildpacks/not_libcnb"))
        .output()
        .unwrap();

    assert_eq!(
        String::from_utf8_lossy(&output.stdout),
        format!(
            "{}\n",
            fixture_dir
                .path()
                .join("buildpacks/not_libcnb")
                .to_string_lossy()
        )
    );
}

#[test]
#[ignore = "integration test"]
fn package_command_error_when_run_in_project_with_no_buildpacks() {
    let fixture_dir = copy_fixture_to_temp_dir("no_buildpacks").unwrap();

    let output = Command::new(CARGO_LIBCNB_BINARY_UNDER_TEST)
        .args(["libcnb", "package", "--release"])
        .current_dir(&fixture_dir)
        .output()
        .unwrap();

    assert_ne!(output.status.code(), Some(0));
    assert_eq!(
        String::from_utf8_lossy(&output.stderr),
        "🔍 Locating buildpacks...\n❌ No buildpacks found!\n"
    );
}

fn validate_packaged_buildpack(packaged_buildpack_dir: &Path, buildpack_id: &BuildpackId) {
    assert!(packaged_buildpack_dir.join("buildpack.toml").exists());
    assert!(packaged_buildpack_dir.join("package.toml").exists());
    assert!(packaged_buildpack_dir.join("bin").join("build").exists());
    assert!(packaged_buildpack_dir.join("bin").join("detect").exists());

    assert_eq!(
        &read_buildpack_data(packaged_buildpack_dir)
            .unwrap()
            .buildpack_descriptor
            .buildpack()
            .id,
        buildpack_id
    );
}

fn validate_packaged_meta_buildpack(
    packaged_buildpack_dir: &Path,
    buildpack_id: &BuildpackId,
    expected_package_descriptor_dependencies: &[PackageDescriptorDependency],
) {
    assert!(packaged_buildpack_dir.join("buildpack.toml").exists());
    assert!(packaged_buildpack_dir.join("package.toml").exists());

    assert_eq!(
        &read_buildpack_data(packaged_buildpack_dir)
            .unwrap()
            .buildpack_descriptor
            .buildpack()
            .id,
        buildpack_id
    );

    assert_eq!(
        read_package_descriptor_data(packaged_buildpack_dir)
            .unwrap()
            .unwrap()
            .package_descriptor
            .dependencies,
        expected_package_descriptor_dependencies
    );
}

fn copy_fixture_to_temp_dir(name: &str) -> Result<TempDir, std::io::Error> {
    let fixture_dir = PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .join("fixtures")
        .join(name);

    // Instead of using `tempfile::tempdir` directly, we get the temporary directory ourselves and
    // canonicalize it before creating a temporary directory inside. We do this since on some
    // operating systems (macOS specifically, see: https://github.com/rust-lang/rust/issues/99608),
    // `env::temp_dir` will return a path with symlinks in it and `TempDir` doesn't allow
    // canonicalization after the fact.
    //
    // Since libcnb-cargo itself also canonicalizes, we need to do the same so we can compare
    // paths when they're output as strings.
    env::temp_dir()
        .canonicalize()
        .and_then(tempdir_in)
        .and_then(|temp_dir| copy_dir_recursively(&fixture_dir, temp_dir.path()).map(|_| temp_dir))
}

fn copy_dir_recursively(source: &Path, destination: &Path) -> std::io::Result<()> {
    match fs::create_dir(destination) {
        Err(io_error) if io_error.kind() == ErrorKind::AlreadyExists => Ok(()),
        other => other,
    }?;

    for entry in fs::read_dir(source)? {
        let dir_entry = entry?;

        if dir_entry.file_type()?.is_dir() {
            copy_dir_recursively(&dir_entry.path(), &destination.join(dir_entry.file_name()))?;
        } else {
            fs::copy(dir_entry.path(), destination.join(dir_entry.file_name()))?;
        }
    }

    Ok(())
}

const X86_64_UNKNOWN_LINUX_MUSL: &str = "x86_64-unknown-linux-musl";
const CARGO_LIBCNB_BINARY_UNDER_TEST: &str = env!("CARGO_BIN_EXE_cargo-libcnb");
const DEFAULT_PACKAGE_DIR_NAME: &str = "packaged";
