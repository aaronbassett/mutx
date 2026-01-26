use crate::error::{MutxError, Result};
use chrono::Local;
use std::fs;
use std::path::PathBuf;
use tracing::debug;

#[derive(Debug, Clone)]
pub struct BackupConfig {
    pub source: PathBuf,
    pub suffix: String,
    pub directory: Option<PathBuf>,
    pub timestamp: bool,
}

/// Create a backup of the specified file using atomic operations
pub fn create_backup(config: &BackupConfig) -> Result<PathBuf> {
    let source = &config.source;

    // Verify source exists
    if !source.exists() {
        return Err(MutxError::PathNotFound(source.clone()));
    }

    if !source.is_file() {
        return Err(MutxError::NotAFile(source.clone()));
    }

    // Generate backup filename
    let backup_path = generate_backup_path(config)?;

    // Ensure backup directory exists
    if let Some(parent) = backup_path.parent() {
        fs::create_dir_all(parent).map_err(|e| MutxError::BackupFailed {
            path: source.clone(),
            source: e,
        })?;
    }

    debug!(
        "Creating atomic backup: {} -> {}",
        source.display(),
        backup_path.display()
    );

    // Atomic backup using copy-to-temp + rename strategy
    let temp_backup = backup_path.with_extension("tmp");

    // Copy to temporary file
    fs::copy(source, &temp_backup).map_err(|e| MutxError::BackupFailed {
        path: source.clone(),
        source: e,
    })?;

    // Atomically rename temp to final backup name
    fs::rename(&temp_backup, &backup_path).map_err(|e| {
        // Cleanup temp file on failure
        let _ = fs::remove_file(&temp_backup);
        MutxError::BackupFailed {
            path: source.clone(),
            source: e,
        }
    })?;

    debug!("Backup created: {}", backup_path.display());
    Ok(backup_path)
}

fn generate_backup_path(config: &BackupConfig) -> Result<PathBuf> {
    let filename = config
        .source
        .file_name()
        .ok_or_else(|| MutxError::Other("Invalid source filename".to_string()))?
        .to_string_lossy();

    let backup_name = if config.timestamp {
        let timestamp = Local::now().format("%Y%m%d_%H%M%S");
        format!("{}.{}{}", filename, timestamp, config.suffix)
    } else {
        format!("{}{}", filename, config.suffix)
    };

    let backup_path = if let Some(dir) = &config.directory {
        dir.join(backup_name)
    } else {
        config
            .source
            .parent()
            .ok_or_else(|| MutxError::Other("Source file has no parent directory".to_string()))?
            .join(backup_name)
    };

    Ok(backup_path)
}

#[cfg(test)]
mod tests {
    use super::*;
    use tempfile::TempDir;

    #[test]
    fn test_generate_backup_path_simple() {
        let temp = TempDir::new().unwrap();
        let source = temp.path().join("test.txt");

        let config = BackupConfig {
            source,
            suffix: ".mutx.backup".to_string(),
            directory: None,
            timestamp: false,
        };

        let path = generate_backup_path(&config).unwrap();
        assert_eq!(
            path.file_name().unwrap().to_str().unwrap(),
            "test.txt.mutx.backup"
        );
    }

    #[test]
    fn test_generate_backup_path_with_directory() {
        let temp = TempDir::new().unwrap();
        let source = temp.path().join("test.txt");
        let backup_dir = temp.path().join("backups");

        let config = BackupConfig {
            source,
            suffix: ".mutx.backup".to_string(),
            directory: Some(backup_dir.clone()),
            timestamp: false,
        };

        let path = generate_backup_path(&config).unwrap();
        assert_eq!(path.parent().unwrap(), backup_dir);
    }
}
