use anyhow::{Context, Result};
use std::fs;
use std::path::{Path, PathBuf};

#[derive(Debug, Clone)]
pub struct BackupConfig {
    pub suffix: String,
    pub timestamp: bool,
    pub backup_dir: Option<PathBuf>,
}

/// Create a backup of the target file
pub fn create_backup(target: &Path, config: &BackupConfig) -> Result<PathBuf> {
    // Skip if target doesn't exist
    if !target.exists() {
        return Ok(PathBuf::new());
    }

    // Generate backup filename
    let filename = target.file_name()
        .context("Invalid target filename")?
        .to_str()
        .context("Filename is not valid UTF-8")?;

    let backup_name = if config.timestamp {
        let timestamp = chrono::Local::now().format("%Y%m%d-%H%M%S");
        format!("{}.{}{}", filename, timestamp, config.suffix)
    } else {
        format!("{}{}", filename, config.suffix)
    };

    // Determine backup location
    let backup_path = if let Some(ref backup_dir) = config.backup_dir {
        if !backup_dir.exists() {
            anyhow::bail!(
                "Backup directory does not exist: {}\nHint: Create it first: mkdir -p {}",
                backup_dir.display(),
                backup_dir.display()
            );
        }
        backup_dir.join(backup_name)
    } else {
        target.with_file_name(backup_name)
    };

    // Create backup using atomic rename
    fs::copy(target, &backup_path)
        .with_context(|| format!("Failed to create backup: {}", backup_path.display()))?;

    Ok(backup_path)
}
