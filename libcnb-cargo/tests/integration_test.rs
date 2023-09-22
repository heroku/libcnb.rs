// Enable Clippy lints that are disabled by default.
// https://rust-lang.github.io/rust-clippy/stable/index.html
#![warn(clippy::pedantic)]

use libcnb_common::toml_file::read_toml_file;
use libcnb_data::buildpack::{BuildpackDescriptor, BuildpackId};
use libcnb_data::buildpack_id;
use libcnb_data::package_descriptor::{PackageDescriptor, PackageDescriptorDependency};
use libcnb_package::output::create_packaged_buildpack_dir_resolver;
use libcnb_package::CargoProfile;
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
fn package_single_composite_buildpack_in_monorepo_buildpack_project() {
    let fixture_dir = copy_fixture_to_temp_dir("multiple_buildpacks").unwrap();

    let output = Command::new(CARGO_LIBCNB_BINARY_UNDER_TEST)
        .args(["libcnb", "package", "--release"])
        .current_dir(
            fixture_dir
                .path()
                .join("composite-buildpacks")
                .join("composite-one"),
        )
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
            packaged_buildpack_dir_resolver(&buildpack_id!("multiple-buildpacks/composite-one"))
                .to_string_lossy()
        )
    );

    validate_packaged_composite_buildpack(
        &packaged_buildpack_dir_resolver(&buildpack_id!("multiple-buildpacks/composite-one")),
        &buildpack_id!("multiple-buildpacks/composite-one"),
        &[
            PackageDescriptorDependency::try_from(packaged_buildpack_dir_resolver(&buildpack_id!(
                "multiple-buildpacks/one"
            ))),
            PackageDescriptorDependency::try_from(packaged_buildpack_dir_resolver(&buildpack_id!(
                "multiple-buildpacks/two"
            ))),
            PackageDescriptorDependency::try_from(fixture_dir.path().join("buildpacks/not_libcnb")),
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
                packaged_buildpack_dir_resolver(&buildpack_id!(
                    "multiple-buildpacks/composite-one"
                )),
                packaged_buildpack_dir_resolver(&buildpack_id!("multiple-buildpacks/one")),
                packaged_buildpack_dir_resolver(&buildpack_id!("multiple-buildpacks/two")),
            ]
            .map(|path| path.to_string_lossy().into_owned())
            .join("\n")
        )
    );

    validate_packaged_composite_buildpack(
        &packaged_buildpack_dir_resolver(&buildpack_id!("multiple-buildpacks/composite-one")),
        &buildpack_id!("multiple-buildpacks/composite-one"),
        &[
            PackageDescriptorDependency::try_from(packaged_buildpack_dir_resolver(&buildpack_id!(
                "multiple-buildpacks/one"
            ))),
            PackageDescriptorDependency::try_from(packaged_buildpack_dir_resolver(&buildpack_id!(
                "multiple-buildpacks/two"
            ))),
            PackageDescriptorDependency::try_from(fixture_dir.path().join("buildpacks/not_libcnb")),
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
fn package_non_libcnb_buildpack_in_composite_buildpack_project() {
    let fixture_dir = copy_fixture_to_temp_dir("multiple_buildpacks").unwrap();

    let output = Command::new(CARGO_LIBCNB_BINARY_UNDER_TEST)
        .args(["libcnb", "package", "--release"])
        .current_dir(fixture_dir.path().join("buildpacks/not_libcnb"))
        .output()
        .unwrap();

    assert_ne!(output.status.code(), Some(0));
    assert_eq!(
        String::from_utf8_lossy(&output.stderr),
        "🚚 Preparing package directory...\n🖥\u{fe0f} Gathering Cargo configuration (for x86_64-unknown-linux-musl)\n🏗\u{fe0f} Building buildpack dependency graph...\n🔀 Determining build order...\n❌ No buildpacks found!\n"
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
        "🚚 Preparing package directory...\n🖥\u{fe0f} Gathering Cargo configuration (for x86_64-unknown-linux-musl)\n🏗\u{fe0f} Building buildpack dependency graph...\n🔀 Determining build order...\n❌ No buildpacks found!\n"
    );
}

#[test]
#[ignore = "integration test"]
fn package_command_respects_ignore_files() {
    let fixture_dir = copy_fixture_to_temp_dir("multiple_buildpacks").unwrap();

    // The `ignore` crate supports `.ignore` files. So this first `cargo libcnb package` execution
    // just sanity checks that our ignore rules will be respected.
    let ignore_file = fixture_dir.path().join(".ignore");
    fs::write(&ignore_file, "composite-buildpacks\nbuildpacks\n").unwrap();

    let output = Command::new(CARGO_LIBCNB_BINARY_UNDER_TEST)
        .args(["libcnb", "package", "--release"])
        .current_dir(fixture_dir.path())
        .output()
        .unwrap();

    assert_ne!(output.status.code(), Some(0));
    assert_eq!(
        String::from_utf8_lossy(&output.stderr),
        "🚚 Preparing package directory...\n🖥\u{fe0f} Gathering Cargo configuration (for x86_64-unknown-linux-musl)\n🏗\u{fe0f} Building buildpack dependency graph...\n🔀 Determining build order...\n❌ No buildpacks found!\n"
    );

    fs::remove_file(ignore_file).unwrap();

    // The `ignore` crate supports `.gitignore` files but only if the folder is within a git repository
    // which is the default configuration used in our directory traversal.
    // https://docs.rs/ignore/latest/ignore/struct.WalkBuilder.html#method.require_git
    //
    // So this second `cargo libcnb package` execution just sanity checks that our gitignore rules
    // in a git repository will be respected.
    fs::create_dir(fixture_dir.path().join(".git")).unwrap();
    let ignore_file = fixture_dir.path().join(".gitignore");
    fs::write(ignore_file, "composite-buildpacks\nbuildpacks\n").unwrap();

    let output = Command::new(CARGO_LIBCNB_BINARY_UNDER_TEST)
        .args(["libcnb", "package", "--release"])
        .current_dir(fixture_dir.path())
        .output()
        .unwrap();

    assert_ne!(output.status.code(), Some(0));
    assert_eq!(
        String::from_utf8_lossy(&output.stderr),
        "🚚 Preparing package directory...\n🖥\u{fe0f} Gathering Cargo configuration (for x86_64-unknown-linux-musl)\n🏗\u{fe0f} Building buildpack dependency graph...\n🔀 Determining build order...\n❌ No buildpacks found!\n"
    );
}

fn validate_packaged_buildpack(packaged_buildpack_dir: &Path, buildpack_id: &BuildpackId) {
    assert!(packaged_buildpack_dir.join("buildpack.toml").exists());
    assert!(packaged_buildpack_dir.join("package.toml").exists());
    assert!(packaged_buildpack_dir.join("bin").join("build").exists());
    assert!(packaged_buildpack_dir.join("bin").join("detect").exists());

    assert_eq!(
        &read_toml_file::<BuildpackDescriptor>(packaged_buildpack_dir.join("buildpack.toml"))
            .unwrap()
            .buildpack()
            .id,
        buildpack_id
    );
}

fn validate_packaged_composite_buildpack(
    packaged_buildpack_dir: &Path,
    buildpack_id: &BuildpackId,
    expected_package_descriptor_dependencies: &[PackageDescriptorDependency],
) {
    assert!(packaged_buildpack_dir.join("buildpack.toml").exists());
    assert!(packaged_buildpack_dir.join("package.toml").exists());

    assert_eq!(
        &read_toml_file::<BuildpackDescriptor>(packaged_buildpack_dir.join("buildpack.toml"))
            .unwrap()
            .buildpack()
            .id,
        buildpack_id
    );

    assert_eq!(
        read_toml_file::<PackageDescriptor>(packaged_buildpack_dir.join("package.toml"))
            .unwrap()
            .dependencies,
        expected_package_descriptor_dependencies
    );
}

fn copy_fixture_to_temp_dir(name: &str) -> Result<TempDir, std::io::Error> {
    let fixture_dir = PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .join("tests/fixtures")
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
        .and_then(|temp_dir| copy_dir_recursively(&fixture_dir, temp_dir.path()).map(|()| temp_dir))
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
