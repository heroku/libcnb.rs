use std::fs;
use std::path::Path;

/// Moves all contents of a directory into another directory, leaving `src_dir` empty.
///
/// # Examples:
/// ```no_run
/// use libherokubuildpack::fs::move_directory_contents;
/// use std::path::PathBuf;
///
/// move_directory_contents(PathBuf::from("foo"), PathBuf::from("bar")).unwrap();
/// ```
/// # Errors:
/// This function will return an error in the following situations, but is not
/// limited to just these cases:
///
/// * `src_dir` does not exist.
/// * `dst_dir` does not exist.
/// * The user lacks the permission to move any of the files in `src_dir`
/// * The user lacks the permission to write files to `dst_dir`
///
/// # Atomicity:
/// This functions makes no atomicity guarantees. It is possible that this function errors after
/// some files already have been moved, leaving `src_dir` and `dst_dir` in an inconsistent state.
pub fn move_directory_contents(
    src_dir: impl AsRef<Path>,
    dst_dir: impl AsRef<Path>,
) -> Result<(), std::io::Error> {
    for dir_entry in fs::read_dir(src_dir.as_ref())? {
        let dir_entry = dir_entry?;
        let relative_path = pathdiff::diff_paths(dir_entry.path(), src_dir.as_ref())
            .ok_or_else(|| std::io::Error::other("std::fs::read_dir unexpectedly returned an entry that is not in the directory that was read."))?;

        fs::rename(dir_entry.path(), dst_dir.as_ref().join(relative_path))?;
    }

    Ok(())
}

#[cfg(test)]
mod test {
    use std::collections::HashMap;
    use std::path::PathBuf;

    use tempfile::tempdir;

    use super::*;

    #[test]
    fn test() {
        let src_dir = tempdir().unwrap();
        let dst_dir = tempdir().unwrap();

        let mut test_data = HashMap::new();
        test_data.insert(PathBuf::from("foo"), String::from("foo"));
        test_data.insert(PathBuf::from("bar"), String::from("bar"));
        test_data.insert(PathBuf::from("baz"), String::from("baz"));
        test_data.insert(
            PathBuf::from("subdir").join("foo"),
            String::from("foosubdir"),
        );
        test_data.insert(
            PathBuf::from("subdir")
                .join("more")
                .join("more")
                .join("file.txt"),
            String::from("Hello World!"),
        );

        for (path, contents) in &test_data {
            if let Some(parent_dir) = path.parent() {
                fs::create_dir_all(src_dir.path().join(parent_dir)).unwrap();
            }

            fs::write(src_dir.path().join(path), contents).unwrap();
        }

        move_directory_contents(&src_dir, &dst_dir).unwrap();

        for (path, expected_contents) in &test_data {
            let actual_contents = fs::read_to_string(dst_dir.path().join(path)).unwrap();
            assert_eq!(expected_contents, &actual_contents);
        }

        assert!(src_dir.path().exists());

        assert!(fs::read_dir(src_dir.path()).unwrap().next().is_none());
    }
}
