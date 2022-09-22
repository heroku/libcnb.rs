use fs_extra::dir::CopyOptions;
use std::ffi::OsStr;
use std::path::{Path, PathBuf};
use tempfile::{tempdir, TempDir};

/// Copies an application directory to a temporary location.
pub(crate) fn copy_app(app_dir: impl AsRef<Path>) -> Result<AppDir, PrepareAppError> {
    tempdir()
        .map_err(PrepareAppError::CreateTempDirError)
        .and_then(|temp_app_dir| {
            fs_extra::dir::copy(
                app_dir.as_ref(),
                temp_app_dir.path(),
                &CopyOptions {
                    content_only: true,
                    ..CopyOptions::default()
                },
            )
            .map_err(PrepareAppError::CopyAppError)
            .map(|_| temp_app_dir.into())
        })
}

#[derive(Debug)]
pub enum PrepareAppError {
    CreateTempDirError(std::io::Error),
    CopyAppError(fs_extra::error::Error),
}

pub(crate) enum AppDir {
    Temporary(TempDir),
    Unmanaged(PathBuf),
}

impl AppDir {
    pub fn as_path(&self) -> &Path {
        match self {
            AppDir::Temporary(temp_dir) => temp_dir.path(),
            AppDir::Unmanaged(path) => path,
        }
    }
}

impl AsRef<OsStr> for AppDir {
    fn as_ref(&self) -> &OsStr {
        self.as_path().as_os_str()
    }
}

impl From<PathBuf> for AppDir {
    fn from(value: PathBuf) -> Self {
        AppDir::Unmanaged(value)
    }
}

impl From<TempDir> for AppDir {
    fn from(value: TempDir) -> Self {
        AppDir::Temporary(value)
    }
}

#[cfg(test)]
mod tests {
    use std::collections::HashMap;
    use std::path::PathBuf;
    use tempfile::tempdir;

    #[test]
    fn copy_app() {
        let source_app_dir = tempdir().unwrap();

        let files = HashMap::from([
            (PathBuf::from("file1.txt"), String::from("all")),
            (PathBuf::from("file2.txt"), String::from("your")),
            (
                PathBuf::from("base").join("are").join("file3.txt"),
                String::from("belong to us!"),
            ),
        ]);

        // Create files in temporary directory
        for (path, contents) in &files {
            let absolute_path = source_app_dir.path().join(path);

            if let Some(dir) = absolute_path.parent() {
                std::fs::create_dir_all(dir).unwrap();
            }

            std::fs::write(absolute_path, contents).unwrap();
        }

        let temp_app_dir = super::copy_app(source_app_dir.path()).unwrap();

        for (path, contents) in files {
            let absolute_path = temp_app_dir.as_path().join(path);

            assert_eq!(std::fs::read_to_string(absolute_path).unwrap(), contents);
        }
    }
}
