use crate::error::{MutxError, Result};
use std::path::Path;

/// Check if a path is a symlink and validate against policy
pub fn check_symlink(path: &Path, follow_symlinks: bool) -> Result<()> {
    // If path doesn't exist, it's not a symlink
    if !path.exists() && path.symlink_metadata().is_err() {
        return Ok(());
    }

    // Use symlink_metadata to avoid following the symlink
    match path.symlink_metadata() {
        Ok(metadata) => {
            if metadata.file_type().is_symlink() && !follow_symlinks {
                return Err(MutxError::SymlinkNotAllowed {
                    path: path.to_path_buf(),
                });
            }
            Ok(())
        }
        Err(_) => Ok(()), // Path doesn't exist or not accessible
    }
}

/// Check if a lock path is a symlink (stricter check)
pub fn check_lock_symlink(path: &Path, follow_lock_symlinks: bool) -> Result<()> {
    // If path doesn't exist, it's not a symlink
    if !path.exists() && path.symlink_metadata().is_err() {
        return Ok(());
    }

    match path.symlink_metadata() {
        Ok(metadata) => {
            if metadata.file_type().is_symlink() && !follow_lock_symlinks {
                return Err(MutxError::LockSymlinkNotAllowed {
                    path: path.to_path_buf(),
                });
            }
            Ok(())
        }
        Err(_) => Ok(()),
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::fs;
    use tempfile::TempDir;

    #[test]
    fn test_nonexistent_path_allowed() {
        let temp = TempDir::new().unwrap();
        let path = temp.path().join("nonexistent");

        assert!(check_symlink(&path, false).is_ok());
        assert!(check_lock_symlink(&path, false).is_ok());
    }

    #[test]
    fn test_regular_file_allowed() {
        let temp = TempDir::new().unwrap();
        let path = temp.path().join("file.txt");
        fs::write(&path, b"data").unwrap();

        assert!(check_symlink(&path, false).is_ok());
        assert!(check_lock_symlink(&path, false).is_ok());
    }
}
