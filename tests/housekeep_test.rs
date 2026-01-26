use mutx::housekeep::{clean_locks, CleanLockConfig};
use std::fs::{self, File};
use std::path::PathBuf;
use std::time::{Duration, SystemTime};
use tempfile::TempDir;

#[test]
fn test_clean_orphaned_locks() {
    let dir = TempDir::new().unwrap();

    // Create orphaned lock file
    let lock1 = dir.path().join("file1.lock");
    File::create(&lock1).unwrap();

    let config = CleanLockConfig {
        dir: dir.path().to_path_buf(),
        recursive: false,
        older_than: None,
        dry_run: false,
    };

    let cleaned = clean_locks(&config).unwrap();

    assert_eq!(cleaned.len(), 1);
    assert_eq!(cleaned[0], lock1);
    assert!(!lock1.exists());
}

#[test]
fn test_skip_active_locks() {
    let dir = TempDir::new().unwrap();

    let lock_path = dir.path().join("active.lock");
    let _active_lock = mutx::FileLock::acquire(&lock_path, mutx::LockStrategy::Wait).unwrap();

    let config = CleanLockConfig {
        dir: dir.path().to_path_buf(),
        recursive: false,
        older_than: None,
        dry_run: false,
    };

    let cleaned = clean_locks(&config).unwrap();

    assert_eq!(cleaned.len(), 0);
    assert!(lock_path.exists());
}

#[test]
fn test_dry_run_doesnt_delete() {
    let dir = TempDir::new().unwrap();
    let lock1 = dir.path().join("file1.lock");
    File::create(&lock1).unwrap();

    let config = CleanLockConfig {
        dir: dir.path().to_path_buf(),
        recursive: false,
        older_than: None,
        dry_run: true,
    };

    let would_clean = clean_locks(&config).unwrap();

    assert_eq!(would_clean.len(), 1);
    assert!(lock1.exists(), "Dry run should not delete");
}

#[test]
fn test_older_than_filter() {
    let dir = TempDir::new().unwrap();

    // Old lock (2 hours ago)
    let old_lock = dir.path().join("old.lock");
    File::create(&old_lock).unwrap();
    let two_hours_ago = SystemTime::now() - Duration::from_secs(2 * 3600);
    filetime::set_file_mtime(
        &old_lock,
        filetime::FileTime::from_system_time(two_hours_ago),
    )
    .unwrap();

    // Recent lock (30 minutes ago)
    let recent_lock = dir.path().join("recent.lock");
    File::create(&recent_lock).unwrap();

    let config = CleanLockConfig {
        dir: dir.path().to_path_buf(),
        recursive: false,
        older_than: Some(Duration::from_secs(3600)), // 1 hour
        dry_run: false,
    };

    let cleaned = clean_locks(&config).unwrap();

    assert_eq!(cleaned.len(), 1);
    assert_eq!(cleaned[0], old_lock);
    assert!(recent_lock.exists(), "Recent lock should not be cleaned");
}

use mutx::housekeep::{clean_backups, CleanBackupConfig};

#[test]
fn test_ignores_user_backup_files() {
    let temp = TempDir::new().unwrap();

    // Create files that look similar but aren't mutx backups
    fs::write(temp.path().join("file.backup"), b"user backup").unwrap();
    fs::write(temp.path().join("file.bak"), b"user bak").unwrap();
    fs::write(
        temp.path().join("file.20260125.backup"),
        b"user dated backup",
    )
    .unwrap();

    // Create actual mutx backup
    fs::write(
        temp.path().join("file.txt.20260125_143000.mutx.backup"),
        b"mutx backup",
    )
    .unwrap();

    let config = CleanBackupConfig {
        dir: temp.path().to_path_buf(),
        recursive: false,
        older_than: Some(Duration::from_secs(0)), // Clean all
        keep_newest: None,
        dry_run: false,
        suffix: ".mutx.backup".to_string(),
    };

    let cleaned = clean_backups(&config).unwrap();

    // Should only clean the one mutx backup
    assert_eq!(cleaned.len(), 1);
    assert!(cleaned[0].to_str().unwrap().contains(".mutx.backup"));

    // User files should still exist
    assert!(temp.path().join("file.backup").exists());
    assert!(temp.path().join("file.bak").exists());
    assert!(temp.path().join("file.20260125.backup").exists());
}

#[test]
fn test_cleans_custom_suffix_backups() {
    let dir = TempDir::new().unwrap();

    // Create backups with custom suffix
    fs::write(dir.path().join("file.txt.bak"), "backup1").unwrap();
    fs::write(dir.path().join("file.txt.20260126_120000.bak"), "backup2").unwrap();

    // Should not touch .mutx.backup files
    fs::write(dir.path().join("other.txt.mutx.backup"), "keep").unwrap();

    let config = CleanBackupConfig {
        dir: dir.path().to_path_buf(),
        recursive: false,
        older_than: None,
        keep_newest: Some(1),
        dry_run: false,
        suffix: ".bak".to_string(),
    };

    let cleaned = clean_backups(&config).unwrap();

    // Should clean one .bak file (keeping newest)
    assert_eq!(cleaned.len(), 1);

    // .mutx.backup file should still exist
    assert!(dir.path().join("other.txt.mutx.backup").exists());
}
