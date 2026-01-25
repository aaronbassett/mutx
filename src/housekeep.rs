use anyhow::{Context, Result};
use std::fs::{self, File};
use std::path::{Path, PathBuf};
use std::time::{Duration, SystemTime};
use fs2::FileExt;

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
        if is_lock_file(path) && is_orphaned(path, config.older_than)? {
            if config.dry_run {
                cleaned.push(path.to_path_buf());
            } else {
                fs::remove_file(path)
                    .with_context(|| format!("Failed to remove lock file: {}", path.display()))?;
                cleaned.push(path.to_path_buf());
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
            let base = extract_base_filename(path);
            let mtime = fs::metadata(path)?.modified()?;
            backups.entry(base).or_insert_with(Vec::new).push((path.to_path_buf(), mtime));
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
                    cleaned.push(path.clone());
                } else {
                    fs::remove_file(path)
                        .with_context(|| format!("Failed to remove backup: {}", path.display()))?;
                    cleaned.push(path.clone());
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
    for entry in fs::read_dir(dir)
        .with_context(|| format!("Failed to read directory: {}", dir.display()))?
    {
        let entry = entry?;
        let path = entry.path();

        if path.is_dir() && recursive {
            visit_directory(&path, recursive, visitor)?;
        } else if path.is_file() {
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
    let name = path.file_name().unwrap().to_str().unwrap();
    // Extract base by removing timestamp and backup suffix
    // e.g., "config.json.20260125-143022.backup" -> "config.json"
    if let Some(pos) = name.find(".20") {
        name[..pos].to_string()
    } else if let Some(pos) = name.rfind(".backup") {
        name[..pos].to_string()
    } else if let Some(pos) = name.rfind(".bak") {
        name[..pos].to_string()
    } else {
        name.to_string()
    }
}

fn is_orphaned(lock_path: &Path, older_than: Option<Duration>) -> Result<bool> {
    // Check age filter first
    if let Some(max_age) = older_than {
        let metadata = fs::metadata(lock_path)?;
        let mtime = metadata.modified()?;
        if let Ok(elapsed) = SystemTime::now().duration_since(mtime) {
            if elapsed < max_age {
                return Ok(false);
            }
        }
    }

    // Try to acquire lock - if successful, it's orphaned
    let file = File::open(lock_path)?;
    match file.try_lock_exclusive() {
        Ok(_) => {
            // Successfully locked = orphaned
            let _ = file.unlock();
            Ok(true)
        }
        Err(e) if e.kind() == std::io::ErrorKind::WouldBlock => {
            // Lock held by another process = not orphaned
            Ok(false)
        }
        Err(e) => Err(e.into()),
    }
}
