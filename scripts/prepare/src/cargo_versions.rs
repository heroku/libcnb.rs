#[derive(Debug, Clone, Eq, PartialEq, PartialOrd, Ord)]
pub(crate) enum SemVerLevel {
    Major,
    Minor,
    Patch,
}

pub(crate) struct CargoVersionUpdate {
    pub current_version: String,
    pub next_version: String,
    pub document: toml_edit::Document,
}

#[derive(Debug)]
pub(crate) enum BePreparedError {
    TomlError(toml_edit::TomlError),
    SemverError(semver::Error),
    Editorial(String),
}

/// Update Cargo.toml contents according to the SemVerLevel given (major, minor, patch)
///
pub(crate) fn cargo_doc_apply_level(
    toml_string: &str,
    level: &SemVerLevel,
) -> Result<CargoVersionUpdate, BePreparedError> {
    let mut document = toml_string
        .parse::<toml_edit::Document>()
        .map_err(BePreparedError::TomlError)?;

    let current_version = workspace_version_from_doc(&document)?;
    let next_version = next_version_from_semver(&current_version, level)?;

    document["workspace"]["package"]["version"] = toml_edit::value(next_version.clone());

    let dependencies = document["workspace"]["dependencies"]
        .as_table()
        .unwrap()
        .iter()
        .map(|(name, _)| name.to_string())
        .collect::<Vec<String>>();

    for dependency in dependencies.iter() {
        document["workspace"]["dependencies"][dependency]["version"] =
            toml_edit::value(next_version.clone());
    }

    Ok(CargoVersionUpdate {
        current_version,
        next_version,
        document,
    })
}

fn next_version_from_semver(
    old_version: &str,
    level: &SemVerLevel,
) -> Result<String, BePreparedError> {
    let mut version = semver::Version::parse(old_version).map_err(BePreparedError::SemverError)?;
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
        .ok_or_else(|| {
            BePreparedError::Editorial(String::from(
                "Expected workspace.package.version to exist in toml document but it did not ",
            ))
        })?
        .as_str()
        .ok_or_else(|| {
            BePreparedError::Editorial(String::from(
                "Expected workspace.package.version to be a string but it is not",
            ))
        })?
        .to_string();

    Ok(version)
}

#[cfg(test)]
mod tests {
    use super::*;
    use libcnb_test::assert_contains;

    #[test]
    fn cargo_doc_apply_level_updates_toml_correctly() {
        let cargo_toml = r#"
[workspace.package]
version = "0.11.3"
rust-version = "1.64"
edition = "2021"
license = "BSD-3-Clause"

[workspace.dependencies]
libcnb = { version = "0.11.3", path = "libcnb" }
libcnb-data = { version = "0.11.3", path = "libcnb-data" }
"#;

        let doc = cargo_doc_apply_level(cargo_toml, &SemVerLevel::Major).unwrap();
        assert_eq!(doc.current_version, "0.11.3");
        assert_eq!(doc.next_version, "1.0.0");

        let doc_string = doc.document.to_string();

        assert_contains!(
            doc_string,
            r#"
[workspace.package]
version = "1.0.0"
"#
        );
        assert_contains!(
            doc_string,
            r#"
libcnb = { version = "1.0.0", path = "libcnb" }
"#
        );

        assert_contains!(
            doc_string,
            r#"
libcnb-data = { version = "1.0.0", path = "libcnb-data" }
"#
        );
    }
}
