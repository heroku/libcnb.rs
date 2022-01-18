use fs_extra::dir::CopyOptions;
use std::env;
use std::env::VarError;
use std::path::{Path, PathBuf};
use tempfile::{tempdir, TempDir};

/// Copies an application directory to a temporary location.
///
/// Relative paths are treated relative to the Crate's root.
pub(crate) fn copy_app(app_dir: impl AsRef<Path>) -> Result<TempDir, PrepareAppError> {
    let absolute_app_dir = if app_dir.as_ref().is_absolute() {
        PathBuf::from(app_dir.as_ref())
    } else {
        env::var("CARGO_MANIFEST_DIR")
            .map_err(PrepareAppError::CannotDetermineManifestDir)
            .map(|cargo_manifest_dir| PathBuf::from(cargo_manifest_dir).join(app_dir.as_ref()))?
    };

    tempdir()
        .map_err(PrepareAppError::CreateTempDirError)
        .and_then(|temp_app_dir| {
            fs_extra::dir::copy(
                absolute_app_dir,
                temp_app_dir.path(),
                &CopyOptions {
                    content_only: true,
                    ..Default::default()
                },
            )
            .map_err(PrepareAppError::CopyAppError)
            .map(|_| temp_app_dir)
        })
}

#[derive(Debug)]
pub enum PrepareAppError {
    CannotDetermineManifestDir(VarError),
    CreateTempDirError(std::io::Error),
    CopyAppError(fs_extra::error::Error),
}

#[cfg(test)]
mod test {
    use super::*;
    use std::collections::HashMap;
    use tempfile::tempdir;

    #[test]
    fn absolute_app_path() {
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

            std::fs::write(absolute_path, &contents).unwrap();
        }

        let temp_app_dir = copy_app(&source_app_dir.path()).unwrap();

        for (path, contents) in files {
            let absolute_path = temp_app_dir.path().join(path);

            assert_eq!(std::fs::read_to_string(absolute_path).unwrap(), contents);
        }
    }
}
