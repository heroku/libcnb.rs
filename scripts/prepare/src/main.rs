use comrak::arena_tree::Node;
use comrak::nodes::NodeValue::{Heading, List, Text};
use comrak::nodes::{Ast, NodeHeading};
use std::cell::RefCell;
use std::{
    env,
    path::{Path, PathBuf},
};
// use toml;

fn main() {
    let root = libcnb_root().expect("Could not determine root of libcnb.rs");
    let changelog = root.join("CHANGELOG.md");
    let cargo = root.join("Cargo.toml");

    println!("Found project root at {}", root.display());
    println!("\n## Detecting semver from CHANGELOG.md\n");

    let changelog_contents =
        std::fs::read_to_string(&changelog).expect("Could not read in CHANGELOG.md");
    let semver_level = semver_from_changelog(&changelog_contents)
        .expect("Could not determine SemVer from CHANGELOG.md");

    println!("semver change is: {:?}", semver_level);
    println!("\n## Updating Cargo.toml {}\n", cargo.display());

    let mut doc = toml_doc_from(&cargo).expect("Could not parse Cargo.toml");

    let old_version =
        workspace_version_from_doc(&doc).expect("Could not parse version from Cargo.toml");
    println!("Last release was version: '{}'", old_version);

    let new_version =
        apply_semver(&old_version, semver_level).expect("Could not parse version as semver");
    println!("Updated version {}", new_version);

    update_versions_in_doc(&mut doc, &new_version);

    write_doc_to_file(doc, &cargo).expect("Could not update Cargo.toml");

    println!("\n## Updating CHANGELOG.md {}\n", changelog.display());

    let unreleased_re = regex::Regex::new(r"## \[Unreleased\]").expect("Regex is invalid");
    if unreleased_re.is_match(&changelog_contents) {
        let changelog_contents = unreleased_re
            .replacen(
                &changelog_contents,
                1,
                format!(
                    "## [Unreleased]\n\n{}",
                    version_ymd_md_string(&new_version, chrono::Local::now())
                ),
            )
            .to_string();

        std::fs::write(changelog, changelog_contents).expect("Could not update CHANGELOG.md");
    } else {
        panic!("Could not find `## [Unreleased]` in changelog")
    }
}

fn is_header(node: &Node<RefCell<Ast>>, desired_level: u32) -> bool {
    if let Heading(NodeHeading { level, .. }) = &(*node.data.borrow()).value {
        if level == &desired_level {
            return true;
        }
    }
    false
}

fn child_contains(node: &Node<RefCell<Ast>>, desired_contains: &str) -> bool {
    if let Some(node) = node.first_child() {
        if let Text(contents) = &(*node.data.borrow()).value {
            if std::str::from_utf8(&contents)
                .unwrap()
                .contains(desired_contains)
            {
                return true;
            }
        }
    }
    false
}

fn sibling_is_list(node: &Node<RefCell<Ast>>) -> bool {
    if let Some(node) = node.next_sibling() {
        if let List(_) = &(*node.data.borrow()).value {
            return true;
        }
    }
    false
}

fn semver_from_changelog(changelog: &str) -> Result<SemVerLevel, BePreparedError> {
    let arena = comrak::Arena::new();
    let root = comrak::parse_document(&arena, changelog, &comrak::ComrakOptions::default());
    let mut nodes = root.children().peekable();

    // Find the Unreleased section, it's a header node that has a single child, a text node
    let mut unreleased = None;
    while let Some(node) = nodes.next() {
        if is_header(node, 2) && child_contains(node, "[Unreleased]") {
            unreleased = Some(node);
            break;
        }
    }

    if unreleased.is_none() {
        return Err(BePreparedError::Editorial(String::from(
            "Missing the `## Unreleased` header from CHANGELOG.md",
        )));
    }

    // Check search sub headers to find fixed, added, and changed header nodes
    let mut fixed = None;
    let mut changed = None;
    let mut added = None;
    while let Some(node) = nodes.next() {
        if is_header(node, 2) {
            // Too far
            break;
        }

        if is_header(node, 3) {
            if child_contains(node, "Fixed") {
                fixed = Some(node)
            }
            if child_contains(node, "Added") {
                added = Some(node)
            }
            if child_contains(node, "Changed") {
                changed = Some(node)
            }
        }
    }

    if let Some(node) = changed {
        if sibling_is_list(node) {
            return Ok(SemVerLevel::Major);
        }
    } else {
        println!("Warning: Changelog.md is missing a `### Fixed` section");
    }

    if let Some(node) = added {
        if sibling_is_list(node) {
            return Ok(SemVerLevel::Minor);
        }
    } else {
        println!("Warning: Changelog.md is missing a `### Added` section");
    }

    if let Some(node) = fixed {
        if sibling_is_list(node) {
            return Ok(SemVerLevel::Patch);
        }
    } else {
        println!("Warning: Changelog.md is missing a `### Added` section");
    }

    Err(BePreparedError::Editorial(String::from(
        "No changes found in the CHANGELOG.md cannot continue",
    )))
}

fn version_ymd_md_string<D: chrono::Datelike>(version: &str, now: D) -> String {
    format!(
        "## [{}] {}-{:0width$}-{:0width$}",
        version,
        now.year(),
        now.month(),
        now.day(),
        width = 2,
    )
}

fn write_doc_to_file(doc: toml_edit::Document, cargo_toml: &Path) -> Result<(), BePreparedError> {
    std::fs::write(cargo_toml, doc.to_string()).map_err(BePreparedError::IoError)?;
    Ok(())
}

fn toml_doc_from(file: &Path) -> Result<toml_edit::Document, BePreparedError> {
    let toml_string = std::fs::read_to_string(file).map_err(BePreparedError::IoError)?;
    let doc = toml_string
        .parse::<toml_edit::Document>()
        .map_err(BePreparedError::TomlError)?;
    Ok(doc)
}

fn apply_semver(old_version: &str, level: SemVerLevel) -> Result<String, BePreparedError> {
    let mut version = semver::Version::parse(&old_version).map_err(BePreparedError::SemverError)?;
    match level {
        SemVerLevel::Major => {
            version.major += 1;
            version.minor = 0;
            version.patch = 0;
        }
        SemVerLevel::Minor => {
            version.minor += 1;
            version.patch = 0;
        }
        SemVerLevel::Patch => {
            version.patch += 1;
        }
    }

    Ok(version.to_string())
}

fn workspace_version_from_doc(doc: &toml_edit::Document) -> Result<String, BePreparedError> {
    let version = doc["workspace"]["package"]["version"]
        .as_value() // Needed to get raw value without formatting
        .ok_or(BePreparedError::Editorial(String::from(
            "Expected workspace.package.version to exist in toml document but it did not ",
        )))?
        .as_str()
        .ok_or(BePreparedError::Editorial(String::from(
            "Expected workspace.package.version to be a string but it is not",
        )))?
        .to_string();

    Ok(version)
}

fn update_versions_in_doc(doc: &mut toml_edit::Document, new_version: &str) {
    doc["workspace"]["package"]["version"] = toml_edit::value(new_version.clone());

    let dependencies = doc["workspace"]["dependencies"]
        .as_table()
        .unwrap()
        .iter()
        .map(|(name, _)| name.to_string())
        .collect::<Vec<String>>();

    for dependency in dependencies.iter() {
        doc["workspace"]["dependencies"][dependency]["version"] =
            toml_edit::value(new_version.clone());
    }
}

fn libcnb_root() -> Result<PathBuf, BePreparedError> {
    let path = env::current_dir().map_err(BePreparedError::IoError)?;
    let mut path_ancestors = path.as_path().ancestors();

    let root = path_ancestors
        .find(|p| matches!(p.file_name(), Some(name) if name.to_str() == Some("libcnb.rs")))
        .ok_or_else(|| {
            BePreparedError::NoSuchDirectory(format!(
                "Could not find directory named `libcnb.rs` in a parent of {}",
                path.display()
            ))
        })?;

    Ok(root.to_path_buf())
}

#[derive(Debug, Clone, Eq, PartialEq, PartialOrd, Ord)]
enum SemVerLevel {
    Major,
    Minor,
    Patch,
}

#[derive(Debug)]
enum BePreparedError {
    IoError(std::io::Error),
    NoSuchDirectory(String),
    TomlError(toml_edit::TomlError),
    SemverError(semver::Error),
    Editorial(String),
}

#[cfg(test)]
mod tests {

    use super::*;
    use chrono::prelude::*;

    #[test]
    fn semver_errors_without_unreleased() {
        let changelog = r"

## Ooops unr3l3453d accidentally deleted
            ";
        assert!(semver_from_changelog(changelog).is_err());
    }

    #[test]
    fn semver_from_changelog_finds_major_minor_patch_okay() {
        let changelog = r"
## [Unreleased]

### Fixed

- The world

### Changed

- The game

### Added

- Good measure
            ";
        assert_eq!(
            SemVerLevel::Major,
            semver_from_changelog(changelog).unwrap()
        );

        let changelog = r"
## [Unreleased]

### Fixed

- The world

### Changed

### Added

- Good measure
";
        assert_eq!(
            SemVerLevel::Minor,
            semver_from_changelog(changelog).unwrap()
        );

        let changelog = r"
## [Unreleased]

### Fixed

- The world

### Changed

### Added

";
        assert_eq!(
            SemVerLevel::Patch,
            semver_from_changelog(changelog).unwrap()
        );
    }

    #[test]
    fn test_ymd_version() {
        let date = NaiveDate::from_ymd_opt(2000, 1, 12).unwrap();
        assert_eq!(
            "## [1.1.1] 2000-01-12",
            version_ymd_md_string("1.1.1", date)
        );
    }
}
