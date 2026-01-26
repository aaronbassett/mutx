use crate::error::{MutxError, Result};
use fs2::FileExt;
use std::fs::{self, File};
use std::path::{Path, PathBuf};
use std::time::{Duration, SystemTime};
use tracing::{debug, warn};

#[derive(Debug, Clone)]
pub struct CleanLockConfig {
    pub dir: PathBuf,
    pub recursive: bool,
    pub older_than: Option<Duration>,
    pub dry_run: bool,
}

#[derive(Debug, Clone)]
pub struct CleanBackupConfig {
    pub dir: PathBuf,
    pub recursive: bool,
    pub older_than: Option<Duration>,
    pub keep_newest: Option<usize>,
    pub dry_run: bool,
}

/// Clean orphaned lock files
pub fn clean_locks(config: &CleanLockConfig) -> Result<Vec<PathBuf>> {
    let mut cleaned = Vec::new();

    visit_directory(&config.dir, config.recursive, &mut |path| {
        if is_lock_file(path) {
            match is_orphaned(path, config.older_than) {
                Ok(true) => {
                    if config.dry_run {
                        debug!("Would remove lock: {}", path.display());
                        cleaned.push(path.to_path_buf());
                    } else {
                        match fs::remove_file(path) {
                            Ok(_) => {
                                debug!("Removed orphaned lock: {}", path.display());
                                cleaned.push(path.to_path_buf());
                            }
                            Err(e) if e.kind() == std::io::ErrorKind::NotFound => {
                                // File already deleted (TOCTOU race) - this is fine
                                debug!("Lock file already removed: {}", path.display());
                            }
                            Err(e) => {
                                warn!("Failed to remove lock file {}: {}", path.display(), e);
                                // Continue processing other files
                            }
                        }
                    }
                }
                Ok(false) => {
                    debug!("Lock file in use, skipping: {}", path.display());
                }
                Err(e) => {
                    warn!("Error checking lock file {}: {}", path.display(), e);
                    // Continue processing other files
                }
            }
        }
        Ok(())
    })?;

    Ok(cleaned)
}

/// Clean old backup files
pub fn clean_backups(config: &CleanBackupConfig) -> Result<Vec<PathBuf>> {
    use std::collections::HashMap;

    let mut backups: HashMap<String, Vec<(PathBuf, SystemTime)>> = HashMap::new();

    // Collect all backups grouped by base filename
    visit_directory(&config.dir, config.recursive, &mut |path| {
        if is_backup_file(path) {
            if let Ok(metadata) = fs::metadata(path) {
                if let Ok(mtime) = metadata.modified() {
                    let base = extract_base_filename(path);
                    backups
                        .entry(base)
                        .or_default()
                        .push((path.to_path_buf(), mtime));
                }
            }
        }
        Ok(())
    })?;

    let mut cleaned = Vec::new();

    // Process each group of backups
    for (_, mut group) in backups {
        // Sort by modification time (newest first)
        group.sort_by(|a, b| b.1.cmp(&a.1));

        for (idx, (path, mtime)) in group.iter().enumerate() {
            let mut should_delete = false;

            // Check keep_newest
            if let Some(keep) = config.keep_newest {
                if idx >= keep {
                    should_delete = true;
                }
            }

            // Check older_than
            if let Some(max_age) = config.older_than {
                if let Ok(elapsed) = SystemTime::now().duration_since(*mtime) {
                    if elapsed > max_age {
                        should_delete = true;
                    }
                }
            }

            if should_delete {
                if config.dry_run {
                    debug!("Would remove backup: {}", path.display());
                    cleaned.push(path.clone());
                } else {
                    match fs::remove_file(path) {
                        Ok(_) => {
                            debug!("Removed old backup: {}", path.display());
                            cleaned.push(path.clone());
                        }
                        Err(e) if e.kind() == std::io::ErrorKind::NotFound => {
                            debug!("Backup file already removed: {}", path.display());
                        }
                        Err(e) => {
                            warn!("Failed to remove backup {}: {}", path.display(), e);
                        }
                    }
                }
            }
        }
    }

    Ok(cleaned)
}

fn visit_directory<F>(dir: &Path, recursive: bool, visitor: &mut F) -> Result<()>
where
    F: FnMut(&Path) -> Result<()>,
{
    let entries = fs::read_dir(dir).map_err(|e| MutxError::ReadFailed {
        path: dir.to_path_buf(),
        source: e,
    })?;

    for entry in entries {
        let entry = entry.map_err(MutxError::Io)?;
        let path = entry.path();

        // Get file type WITHOUT following symlinks
        let file_type = entry.file_type().map_err(MutxError::Io)?;

        // Skip symlinks entirely (don't traverse, don't process)
        if file_type.is_symlink() {
            debug!("Skipping symlink: {}", path.display());
            continue;
        }

        if file_type.is_dir() && recursive {
            visit_directory(&path, recursive, visitor)?;
        } else if file_type.is_file() {
            visitor(&path)?;
        }
    }
    Ok(())
}

fn is_lock_file(path: &Path) -> bool {
    path.extension().and_then(|s| s.to_str()) == Some("lock")
}

fn is_backup_file(path: &Path) -> bool {
    if let Some(name) = path.file_name().and_then(|s| s.to_str()) {
        name.contains(".backup") || name.contains(".bak")
    } else {
        false
    }
}

fn extract_base_filename(path: &Path) -> String {
    path.file_name()
        .and_then(|n| n.to_str())
        .map(|name| {
            // Extract base by removing timestamp and backup suffix
            if let Some(pos) = name.find(".20") {
                name[..pos].to_string()
            } else if let Some(pos) = name.rfind(".backup") {
                name[..pos].to_string()
            } else if let Some(pos) = name.rfind(".bak") {
                name[..pos].to_string()
            } else {
                name.to_string()
            }
        })
        .unwrap_or_else(|| "unknown".to_string())
}

fn is_orphaned(lock_path: &Path, older_than: Option<Duration>) -> Result<bool> {
    // Check age filter first
    if let Some(max_age) = older_than {
        let metadata = fs::metadata(lock_path).map_err(MutxError::Io)?;
        let mtime = metadata.modified().map_err(MutxError::Io)?;
        if let Ok(elapsed) = SystemTime::now().duration_since(mtime) {
            if elapsed < max_age {
                return Ok(false);
            }
        }
    }

    // Try to acquire lock - if successful, it's orphaned
    let file = File::open(lock_path).map_err(MutxError::Io)?;

    match file.try_lock_exclusive() {
        Ok(_) => {
            // Successfully locked = orphaned
            // Lock released when file is dropped
            Ok(true)
        }
        Err(e) if e.kind() == std::io::ErrorKind::WouldBlock => {
            // Lock held by another process = not orphaned
            Ok(false)
        }
        Err(e) => Err(MutxError::Io(e)),
    }
}
