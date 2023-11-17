use std::path::{Component, Path, PathBuf};

#[must_use]
pub fn absolutize_path(path: &Path, parent: &Path) -> PathBuf {
    if path.is_relative() {
        normalize_path(&parent.join(path))
    } else {
        PathBuf::from(path)
    }
}

/// Normalizes a path without it needing to exist on the file system.
///
/// Works similarly to [`std::fs::canonicalize`] but without using the file system. This means that
/// symbolic links will not be resolved. In return, it can be used before creating a path on the
/// file system.
#[must_use]
fn normalize_path(path: &Path) -> PathBuf {
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

#[cfg(test)]
mod test {
    use super::normalize_path;
    use std::path::PathBuf;

    #[test]
    fn test_normalize_path() {
        assert_eq!(
            normalize_path(&PathBuf::from("/foo/bar/baz")),
            PathBuf::from("/foo/bar/baz")
        );

        assert_eq!(
            normalize_path(&PathBuf::from("/foo/bar/../baz")),
            PathBuf::from("/foo/baz")
        );

        assert_eq!(
            normalize_path(&PathBuf::from("/foo/bar/./././././baz")),
            PathBuf::from("/foo/bar/baz")
        );

        assert_eq!(
            normalize_path(&PathBuf::from("/foo/bar/../../23/42/../.././hello.txt")),
            PathBuf::from("/hello.txt")
        );

        assert_eq!(
            normalize_path(&PathBuf::from("foo/bar/../../23/42/../.././hello.txt")),
            PathBuf::from("hello.txt")
        );
    }
}
