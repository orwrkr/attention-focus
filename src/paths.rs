//! Path identity checks that protect application storage and export destinations.
use std::{
    fs, io,
    path::{Component, Path, PathBuf},
};

fn invalid(message: &str) -> io::Error {
    io::Error::other(message)
}

/// Resolve existing ancestors, including junctions, before appending missing components.
/// Parent components are rejected rather than guessed across a missing directory.
pub fn resolve(path: &Path) -> io::Result<PathBuf> {
    if !path.is_absolute() || path.components().any(|c| matches!(c, Component::ParentDir)) {
        return Err(invalid(&crate::i18n::message(
            "errors.absolute_path_required",
            &[],
        )));
    }
    match fs::canonicalize(path) {
        Ok(path) => Ok(path),
        Err(error) if error.kind() == io::ErrorKind::NotFound => {
            let parent = path.parent().ok_or(error)?;
            Ok(resolve(parent)?.join(
                path.file_name()
                    .ok_or_else(|| invalid(&crate::i18n::message("errors.invalid_path", &[])))?,
            ))
        }
        Err(error) => Err(error),
    }
}

fn linked(path: &Path) -> io::Result<bool> {
    let metadata = fs::symlink_metadata(path)?;
    #[cfg(windows)]
    {
        use std::os::windows::fs::MetadataExt;
        Ok(metadata.file_attributes() & 0x400 != 0)
    }
    #[cfg(not(windows))]
    {
        Ok(metadata.file_type().is_symlink())
    }
}

/// Reject redirected app files (including SQLite journals) before writable initialization.
pub fn check_storage(directory: &Path) -> io::Result<()> {
    match fs::read_dir(directory) {
        Ok(entries) => {
            for entry in entries {
                let path = entry?.path();
                if linked(&path)? {
                    return Err(invalid("Application storage contains a linked entry."));
                }
                if path.is_dir() {
                    check_storage(&path)?;
                }
            }
        }
        Err(error) if error.kind() == io::ErrorKind::NotFound => (),
        Err(error) => return Err(error),
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn resolving_rejects_relative_and_parent_paths_and_keeps_missing_names() {
        let temp = tempfile::tempdir().unwrap();
        assert!(resolve(Path::new("relative")).is_err());
        assert!(resolve(&temp.path().join("a").join("..").join("b")).is_err());
        let missing = resolve(&temp.path().join("missing").join("file.txt")).unwrap();
        assert_eq!(missing.file_name().unwrap(), "file.txt");
        assert_eq!(
            missing.parent().unwrap().parent().unwrap(),
            fs::canonicalize(temp.path()).unwrap()
        );
    }

    #[test]
    fn storage_check_accepts_missing_and_plain_directories() {
        let temp = tempfile::tempdir().unwrap();
        assert!(check_storage(&temp.path().join("missing")).is_ok());
        fs::create_dir(temp.path().join("nested")).unwrap();
        fs::write(temp.path().join("nested/file"), b"Synthetic").unwrap();
        assert!(check_storage(temp.path()).is_ok());
    }

    #[cfg(windows)]
    #[test]
    fn storage_check_rejects_junctions_and_aliases_resolve_to_their_target() {
        let temp = tempfile::tempdir().unwrap();
        let storage = temp.path().join("storage");
        let target = temp.path().join("target");
        fs::create_dir_all(&storage).unwrap();
        fs::create_dir_all(&target).unwrap();
        let alias = storage.join("alias");
        let status = std::process::Command::new("powershell.exe")
            .args(["-NoProfile", "-NonInteractive", "-Command", "$ErrorActionPreference='Stop'; New-Item -ItemType Junction -Path $env:AF_TEST_LINK -Target $env:AF_TEST_TARGET | Out-Null"])
            .env("AF_TEST_LINK", &alias)
            .env("AF_TEST_TARGET", &target)
            .status()
            .unwrap();
        assert!(
            status.success(),
            "Junctions are needed to verify this rule."
        );
        assert!(check_storage(&storage).is_err());
        assert_eq!(
            resolve(&alias.join("file")).unwrap(),
            fs::canonicalize(&target).unwrap().join("file")
        );
        fs::remove_dir(&alias).unwrap();
        assert!(check_storage(&storage).is_ok());
    }
}
