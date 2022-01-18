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
    use tempfile::tempdir;

    #[test]
    fn absolute_app_path() {
        let source_app_dir = tempdir().unwrap();
        let _file1_path = source_app_dir.path().join("file1.txt");
        let _file2_path = source_app_dir.path().join("file2.txt");
        let _file3_path = source_app_dir.path().join("subdir").join("file3.txt");

        let _app_dir = copy_app(&source_app_dir.path()).unwrap();
    }
}
