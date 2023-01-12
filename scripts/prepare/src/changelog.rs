use crate::cargo_versions::{CargoVersionUpdate, SemVerLevel};

use comrak::arena_tree::Node;
use comrak::nodes::NodeValue::{Heading, List, Text};
use comrak::nodes::{Ast, NodeHeading};
use std::cell::RefCell;

/// Convert a changelog markdown string into semver level
///
/// This code parses the changelog to determine the the semver level change (major, minor, patch)
/// based on what was documented as modified.
///
/// Emits a warning to stderr if any one of: fixed, changed, or added sections are
/// not present under `## [Unreleased]`
///
/// Errors if none of those sections are present.
pub(crate) fn semver_from_changelog(changelog: &str) -> Result<SemVerLevel, String> {
    let mut unreleased = None;
    let mut fixed = None;
    let mut changed = None;
    let mut added = None;

    let arena = comrak::Arena::new();
    let root = comrak::parse_document(&arena, changelog, &comrak::ComrakOptions::default());
    let mut nodes = root.children().peekable();

    // Find the Unreleased section, it's a header node that has a single child, a text node
    for node in nodes.by_ref() {
        if is_header(node, 2) && child_contains(node, "[Unreleased]") {
            unreleased = Some(node);
            break;
        }
    }

    if unreleased.is_none() {
        return Err(String::from(
            "Missing the `## Unreleased` header from CHANGELOG.md",
        ));
    }

    // Check search sub headers to find fixed, added, and changed header nodes
    for node in nodes {
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

    [
        (changed, SemVerLevel::Major, "Changed"),
        (added, SemVerLevel::Minor, "Added"),
        (fixed, SemVerLevel::Patch, "Fixed"),
    ]
    .iter()
    .filter_map(|(node, level, name)| {
        node.and_then(|node| Some((node, level))) // Unwrap Some(node) and continue
            .or_else(|| {
                // Emit warning and filter
                eprintln!("Warning: Changelog.md is missing a `### {}` section", name);
                None
            })
    })
    .find_map(|(node, level)| {
        if sibling_is_list(node) {
            Some(level.clone())
        } else {
            None
        }
    })
    .ok_or_else(|| String::from("No changes found in the CHANGELOG.md cannot continue"))
}

#[derive(Debug)]
pub(crate) enum ChangelogError {
    RegexError(regex::Error),
    CannotFindUnreleasedSection(String),
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

/// LOL
pub(crate) fn changelog_doc_update_versions(
    contents: &str,
    cargo_doc: &CargoVersionUpdate,
) -> Result<String, ChangelogError> {
    let unreleased_re =
        regex::Regex::new(r"## \[Unreleased\]").map_err(ChangelogError::RegexError)?;

    if unreleased_re.is_match(contents) {
        let new_contents = unreleased_re
            .replacen(
                contents,
                1,
                format!(
                    "## [Unreleased]\n\n{}",
                    version_ymd_md_string(&cargo_doc.next_version, chrono::Local::now())
                ),
            )
            .to_string();
        Ok(new_contents)
    } else {
        Err(ChangelogError::CannotFindUnreleasedSection(String::from(
            "Could not find `## [Unreleased]` in changelog",
        )))
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
            if std::str::from_utf8(contents)
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

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_ymd_version() {
        let date = chrono::NaiveDate::from_ymd_opt(2000, 1, 12).unwrap();
        assert_eq!(
            "## [1.1.1] 2000-01-12",
            version_ymd_md_string("1.1.1", date)
        );
    }

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
}
