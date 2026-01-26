use crate::error::{MutxError, Result};
use directories::ProjectDirs;
use sha2::{Digest, Sha256};
use std::fs;
use std::path::{Path, PathBuf};

/// Derive the lock file path for a given output file
pub fn derive_lock_path(output_path: &Path, is_custom: bool) -> Result<PathBuf> {
    if is_custom {
        // Custom lock paths are used as-is, but must be validated
        return Ok(output_path.to_path_buf());
    }

    // Get canonical absolute path
    let canonical = output_path.canonicalize().or_else(|_| {
        // If file doesn't exist yet, canonicalize parent and append filename
        let parent = output_path
            .parent()
            .ok_or_else(|| MutxError::Other("Output path has no parent".to_string()))?;

        // POLA: Error if parent doesn't exist - don't create it
        if !parent.exists() {
            return Err(MutxError::PathNotFound(parent.to_path_buf()));
        }

        let parent_canonical = parent.canonicalize().map_err(MutxError::Io)?;

        let filename = output_path
            .file_name()
            .ok_or_else(|| MutxError::Other("Output path has no filename".to_string()))?;

        Ok(parent_canonical.join(filename))
    })?;

    // Extract path components
    let components: Vec<_> = canonical.components().collect();

    // Get filename
    let filename = canonical
        .file_name()
        .ok_or_else(|| MutxError::Other("Output path has no filename".to_string()))?
        .to_str()
        .ok_or_else(|| MutxError::Other("Non-UTF8 filename".to_string()))?;

    // Get parent directory name
    let parent_name = canonical
        .parent()
        .and_then(|p| p.file_name())
        .and_then(|n| n.to_str())
        .unwrap_or("root");

    // Build initialism from ancestor directories (excluding parent and filename)
    // Limit to last 3 ancestors for readability (hash provides uniqueness)
    let mut initialism = String::new();
    if components.len() > 2 {
        // Parent is at components.len() - 2 (filename is at components.len() - 1)
        // Get up to 3 ancestors before parent (for human readability)
        let parent_idx = components.len() - 2;
        let start_idx = if parent_idx > 3 {
            parent_idx - 3 // Last 3 ancestors before parent
        } else {
            1 // Start after root
        };

        for component in &components[start_idx..parent_idx] {
            if let Some(name) = component.as_os_str().to_str() {
                if let Some(first_char) = name.chars().next() {
                    if first_char.is_alphanumeric() {
                        initialism.push(first_char.to_ascii_lowercase());
                        initialism.push('.');
                    }
                }
            }
        }
    }

    // Compute hash of canonical path
    let mut hasher = Sha256::new();
    hasher.update(canonical.to_string_lossy().as_bytes());
    let hash_bytes = hasher.finalize();
    let hash = format!("{:x}", hash_bytes);
    let hash_short = &hash[..8];

    // Build lock filename: {initialism}{parent}.{filename}.{hash}.lock
    let lock_filename = format!(
        "{}{}.{}.{}.lock",
        initialism, parent_name, filename, hash_short
    );

    // Get platform cache directory
    let cache_dir = get_lock_cache_dir()?;

    Ok(cache_dir.join(lock_filename))
}

/// Get the platform-specific cache directory for lock files.
///
/// Returns an error if the cache directory cannot be determined
/// (e.g., on systems without a home directory or with permission issues).
///
/// Users can work around this by providing an explicit directory
/// to housekeep commands.
///
/// # Manual Testing
///
/// ```bash
/// # Restrict cache directory permissions
/// chmod 000 ~/.cache
/// mutx housekeep locks
/// # Should show error, not panic
/// chmod 755 ~/.cache  # Restore
/// ```
pub fn get_lock_cache_dir() -> Result<PathBuf> {
    let proj_dirs = ProjectDirs::from("", "", "mutx").ok_or_else(|| {
        MutxError::Other(
            "Failed to determine lock cache directory. \
                 Try specifying an explicit directory with the DIR argument."
                .to_string(),
        )
    })?;

    let cache_dir = proj_dirs.cache_dir().join("locks");

    // Create directory if it doesn't exist
    if !cache_dir.exists() {
        fs::create_dir_all(&cache_dir).map_err(|e| MutxError::CacheDirectoryFailed {
            path: cache_dir.clone(),
            source: e,
        })?;
    }

    Ok(cache_dir)
}

/// Validate that lock path doesn't equal output path
pub fn validate_lock_path(lock_path: &Path, output_path: &Path) -> Result<()> {
    // Canonicalize both paths for comparison
    let lock_canonical = lock_path
        .canonicalize()
        .or_else(|_| Ok::<PathBuf, std::io::Error>(lock_path.to_path_buf()))?;

    let output_canonical = output_path
        .canonicalize()
        .or_else(|_| Ok::<PathBuf, std::io::Error>(output_path.to_path_buf()))?;

    if lock_canonical == output_canonical {
        return Err(MutxError::LockPathCollision {
            lock_path: lock_path.to_path_buf(),
            output_path: output_path.to_path_buf(),
        });
    }

    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    use tempfile::TempDir;

    #[test]
    fn test_get_lock_cache_dir_creates_directory() {
        let cache_dir = get_lock_cache_dir().unwrap();
        assert!(cache_dir.exists());
        assert!(cache_dir.is_dir());
        assert!(cache_dir.to_string_lossy().contains("mutx"));
        assert!(cache_dir.to_string_lossy().contains("locks"));
    }

    #[test]
    fn test_validate_lock_path_collision() {
        let temp = TempDir::new().unwrap();
        let path = temp.path().join("test.txt");

        let result = validate_lock_path(&path, &path);
        assert!(result.is_err());
        assert!(matches!(
            result.unwrap_err(),
            MutxError::LockPathCollision { .. }
        ));
    }

    #[test]
    fn test_validate_lock_path_different() {
        let temp = TempDir::new().unwrap();
        let output = temp.path().join("output.txt");
        let lock = temp.path().join("output.lock");

        let result = validate_lock_path(&lock, &output);
        assert!(result.is_ok());
    }
}
