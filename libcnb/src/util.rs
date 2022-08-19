use std::fs;
use std::fs::Permissions;
use std::path::Path;

/// Removes [`std::io::Error`] values from a [`Result`] that have the
/// [`std::io::ErrorKind::NotFound`] error kind by replacing them with the default value for `T`.
pub(crate) fn default_on_not_found<T: Default>(
    result: Result<T, std::io::Error>,
) -> Result<T, std::io::Error> {
    match result {
        Err(io_error) => match io_error.kind() {
            std::io::ErrorKind::NotFound => Ok(T::default()),
            _ => Err(io_error),
        },
        other => other,
    }
}

/// Recursively removes the given path, similar to [`std::fs::remove_dir_all`].
///
/// Compared to `remove_dir_all`, this function behaves more like `rm -rf` on UNIX systems. It will
/// delete files and directories even if their permissions would normally prevent deletion as long
/// as the current user is the owner of these files (or root).
pub(crate) fn remove_recursively<P: AsRef<Path>>(path: P) -> std::io::Result<()> {
    if path.as_ref().symlink_metadata()?.is_dir() {
        // To delete a directory, the current user must have the permission to write and list the
        // directory (to empty it before deleting). To reduce the possibility of permission errors,
        // we try to set the correct permissions before attempting to delete the directory and the
        // files within it.
        let permissions = if cfg!(target_family = "unix") {
            use std::os::unix::fs::PermissionsExt;
            Permissions::from_mode(0o777)
        } else {
            let mut permissions = path.as_ref().metadata()?.permissions();
            permissions.set_readonly(false);
            permissions
        };

        fs::set_permissions(&path, permissions)?;

        for entry in fs::read_dir(&path)? {
            let path = entry?.path();

            // Since the directory structure could be a deep, blowing the stack is a real danger
            // here. We use the `stacker` crate to allocate stack on the heap if we're running out
            // of stack space.
            //
            // Neither the minimum stack size nor the amount of bytes allocated when we run out of
            // stack space are backed by data/science. They're "best guesses", if you have reason
            // to believe they need to change, you're probably right.
            stacker::maybe_grow(4096, 32768, || remove_recursively(&path))?;
        }

        fs::remove_dir(&path)
    } else {
        fs::remove_file(&path)
    }
}

#[cfg(test)]
mod tests {
    use crate::util::{default_on_not_found, remove_recursively};
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

        remove_recursively(temp_dir.path()).unwrap();
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

        remove_recursively(temp_dir.path()).unwrap();
    }
}
