use std::{
    env,
    path::{Path, PathBuf},
};
// use toml;

fn main() {
    let root = libcnb_root().expect("Could not determine root of libcnb.rs");
    println!("Found project root at {}", root.display());

    let cargo = root.join("Cargo.toml");
    println!("\n## Updating Cargo.toml {}\n", cargo.display());

    let level = SemVerLevel::Patch;
    let mut doc = toml_doc_from(&cargo).expect("Could not parse Cargo.toml");

    let old_version =
        workspace_version_from_doc(&doc).expect("Could not parse version from Cargo.toml");
    println!("Last release was version '{}'", old_version);

    let new_version = apply_semver(&old_version, level).expect("Could not parse version as semver");
    println!("Updated version {}", new_version);

    update_versions_in_doc(&mut doc, &new_version);

    write_doc_to_file(doc, &cargo).expect("Could not update Cargo.toml")
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
