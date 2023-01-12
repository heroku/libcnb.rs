use crate::cargo_versions::cargo_doc_apply_level;
use crate::changelog::{changelog_doc_update_versions, semver_from_changelog};
use std::{env, path::PathBuf};

mod cargo_versions;
mod changelog;

fn main() {
    let root = libcnb_root().expect("Could not determine root of libcnb.rs");
    let changelog = root.join("CHANGELOG.md");
    let cargo = root.join("Cargo.toml");
    let changelog_contents =
        std::fs::read_to_string(&changelog).expect("Could not read in CHANGELOG.md");
    let cargo_contents = std::fs::read_to_string(&cargo).expect("Could not read in Cargo.toml");

    println!("Found project root at {}", root.display());
    println!("\n## Detecting semver from CHANGELOG.md\n");

    let semver_level = semver_from_changelog(&changelog_contents)
        .expect("Could not determine SemVer from CHANGELOG.md");

    println!("semver change is: {:?}", semver_level);
    println!("\n## Updating Cargo.toml {}\n", cargo.display());

    let cargo_doc = cargo_doc_apply_level(&cargo_contents, &semver_level)
        .expect("Could not update Cargo.toml versions");

    println!("Last release was version: '{}'", cargo_doc.current_version);
    println!("Updated version {}", cargo_doc.next_version);
    println!("\n## Updating CHANGELOG.md {}\n", changelog.display());

    let changelog_doc = changelog_doc_update_versions(&changelog_contents, &cargo_doc)
        .expect("Could not update CHANGELOG.md");

    std::fs::write(cargo, cargo_doc.document.to_string()).expect("Could not write to Cargo.toml");
    std::fs::write(changelog, changelog_doc).expect("Could not update CHANGELOG.md");
}

#[derive(Debug)]
enum FindRootError {
    IoError(std::io::Error),
    NoSuchDirectoryError(String),
}

fn libcnb_root() -> Result<PathBuf, FindRootError> {
    let path = env::current_dir().map_err(FindRootError::IoError)?;
    let mut path_ancestors = path.as_path().ancestors();

    let root = path_ancestors
        .find(|p| matches!(p.file_name(), Some(name) if name.to_str() == Some("libcnb.rs")))
        .ok_or_else(|| {
            FindRootError::NoSuchDirectoryError(format!(
                "Could not find directory named `libcnb.rs` in a parent of {}",
                path.display()
            ))
        })?;

    Ok(root.to_path_buf())
}
