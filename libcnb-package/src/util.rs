use std::path::{Component, Path, PathBuf};

/// Recursively calculate the size of a directory and its contents in bytes.
///
/// # Errors
///
/// Returns `Err` if an IO error occurred during the size calucaltion.
// Not using `fs_extra::dir::get_size` since it doesn't handle symlinks correctly:
// https://github.com/webdesus/fs_extra/issues/59
pub fn calculate_dir_size(path: impl AsRef<Path>) -> std::io::Result<u64> {
    let mut size_in_bytes = 0;

    // The size of the directory entry (ie: its metadata only, not the directory contents).
    size_in_bytes += path.as_ref().metadata()?.len();

    for entry in std::fs::read_dir(&path)? {
        let entry = entry?;
        let metadata = entry.metadata()?;

        if metadata.is_dir() {
            size_in_bytes += calculate_dir_size(entry.path())?;
        } else {
            size_in_bytes += metadata.len();
        }
    }

    Ok(size_in_bytes)
}

#[must_use]
pub fn absolutize_path(path: &Path, parent: &Path) -> PathBuf {
    if path.is_relative() {
        normalize_path(&parent.join(path))
    } else {
        PathBuf::from(path)
    }
}

#[must_use]
pub fn normalize_path(path: &Path) -> PathBuf {
    let mut components = path.components().peekable();

    let mut result = if let Some(component @ Component::Prefix(..)) = components.peek().copied() {
        components.next();
        PathBuf::from(component.as_os_str())
    } else {
        PathBuf::new()
    };

    for component in components {
        match component {
            Component::Prefix(..) => unreachable!(),
            Component::RootDir => {
                result.push(component.as_os_str());
            }
            Component::CurDir => {}
            Component::ParentDir => {
                result.pop();
            }
            Component::Normal(component) => {
                result.push(component);
            }
        }
    }

    result
}
