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
