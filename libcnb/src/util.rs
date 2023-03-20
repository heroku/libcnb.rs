use std::fs;
use std::fs::Permissions;
use std::path::Path;

/// Removes [`std::io::Error`] values from a [`Result`] that have the
/// [`std::io::ErrorKind::NotFound`] error kind by replacing them with the default value for `T`.
pub(crate) fn default_on_not_found<T: Default>(
    result: Result<T, std::io::Error>,
) -> Result<T, std::io::Error> {
    match result {
        Err(io_error) if is_not_found_error_kind(&io_error) => Ok(T::default()),
        other => other,
    }
}

/// Checks if the error kind of the given [`std::io::Error`]  is [`std::io::ErrorKind::NotFound`].
pub(crate) fn is_not_found_error_kind(error: &std::io::Error) -> bool {
    matches!(error.kind(), std::io::ErrorKind::NotFound)
}

/// Recursively removes the given path, similar to [`std::fs::remove_dir_all`].
///
/// Compared to `remove_dir_all`, this function behaves more like `rm -rf` on UNIX systems.
/// It will delete directories even if their permissions would normally prevent deletion as
/// long as the current user is the owner of them (or root).
pub(crate) fn remove_dir_recursively(dir: &Path) -> std::io::Result<()> {
    // To delete a directory, the current user must have the permission to write and list the
    // directory (to empty it before deleting). To reduce the possibility of permission errors,
    // we try to set the correct permissions before attempting to delete the directory and the
    // files within it.
    let permissions = if cfg!(target_family = "unix") {
        use std::os::unix::fs::PermissionsExt;
        Permissions::from_mode(0o777)
    } else {
        let mut permissions = dir.symlink_metadata()?.permissions();

        // `clippy::permissions_set_readonly_false` warns about making a file writable by everyone
        // on UNIX systems when `set_readonly(false)` is used. We use it as a fallback for non
        // UNIXes so disabling the lint is correct here. In addition, the file/directory that we
        // make writable will be deleted afterwards.
        #[allow(clippy::permissions_set_readonly_false)]
        permissions.set_readonly(false);

        permissions
    };

    fs::set_permissions(dir, permissions)?;

    for entry in fs::read_dir(dir)? {
        let entry = entry?;
        let path = entry.path();

        if entry.file_type()?.is_dir() {
            remove_dir_recursively(&path)?;
        } else {
            fs::remove_file(path)?;
        }
    }

    fs::remove_dir(dir)
}

#[cfg(test)]
mod tests {
    use crate::util::{default_on_not_found, remove_dir_recursively};
    use std::fs;
    use std::fs::Permissions;
    use std::io::ErrorKind;
    use tempfile::tempdir;

    #[test]
    fn default_on_not_found_with_notfound() {
        let not_found_io_error = std::io::Error::from(ErrorKind::NotFound);

        assert_eq!(
            default_on_not_found::<Option<String>>(Err(not_found_io_error)).unwrap(),
            None
        );
    }

    #[test]
    fn default_on_not_found_with_brokenpipe() {
        let broken_pipe_io_error = std::io::Error::from(ErrorKind::BrokenPipe);

        assert!(default_on_not_found::<Option<String>>(Err(broken_pipe_io_error)).is_err());
    }

    #[test]
    fn default_on_not_found_with_ok() {
        assert_eq!(default_on_not_found(Ok("Hello!")).unwrap(), "Hello!");
    }

    #[test]
    #[cfg(target_family = "unix")]
    fn remove_recursively_readonly_directory() {
        use std::os::unix::fs::PermissionsExt;

        let temp_dir = tempdir().unwrap();
        let directory = temp_dir.path().join("sub_dir");
        fs::create_dir_all(&directory).unwrap();
        fs::write(directory.join("destination.txt"), "LV-426").unwrap();
        fs::set_permissions(&directory, Permissions::from_mode(0o555)).unwrap();

        remove_dir_recursively(temp_dir.path()).unwrap();
    }

    #[test]
    #[cfg(target_family = "unix")]
    fn remove_recursively_no_executable_bit_directory() {
        use std::os::unix::fs::PermissionsExt;

        let temp_dir = tempdir().unwrap();
        let directory = temp_dir.path().join("sub_dir");
        fs::create_dir_all(&directory).unwrap();
        fs::write(directory.join("destination.txt"), "LV-426").unwrap();
        fs::set_permissions(&directory, Permissions::from_mode(0o666)).unwrap();

        remove_dir_recursively(temp_dir.path()).unwrap();
    }
}
