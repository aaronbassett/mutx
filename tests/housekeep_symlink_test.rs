use mutx::housekeep::{clean_locks, CleanLockConfig};
use std::fs::{self, File};
use tempfile::TempDir;

#[test]
#[cfg(unix)]
fn test_housekeep_skips_symlinks_by_default() {
    use std::os::unix::fs as unix_fs;

    let temp = TempDir::new().unwrap();

    // Create a real file outside the directory
    let external_file = temp.path().join("external.txt");
    fs::write(&external_file, b"important").unwrap();

    // Create a subdirectory with a symlink pointing out
    let subdir = temp.path().join("locks");
    fs::create_dir(&subdir).unwrap();

    let symlink = subdir.join("dangerous.lock");
    unix_fs::symlink(&external_file, &symlink).unwrap();

    // Clean should skip the symlink
    let config = CleanLockConfig {
        dir: subdir,
        recursive: false,
        older_than: None,
        dry_run: false,
    };

    let cleaned = clean_locks(&config).unwrap();

    // No files should be cleaned (symlink was skipped)
    assert_eq!(cleaned.len(), 0);

    // External file should still exist
    assert!(external_file.exists());
}

#[test]
#[cfg(unix)]
fn test_housekeep_does_not_traverse_symlinked_directories() {
    use std::os::unix::fs as unix_fs;

    let temp = TempDir::new().unwrap();

    // Create external directory with a lock file
    let external_dir = temp.path().join("external");
    fs::create_dir(&external_dir).unwrap();
    let external_lock = external_dir.join("important.lock");
    File::create(&external_lock).unwrap();

    // Create directory with symlink to external
    let scan_dir = temp.path().join("scan");
    fs::create_dir(&scan_dir).unwrap();

    let dir_symlink = scan_dir.join("link_to_external");
    unix_fs::symlink(&external_dir, &dir_symlink).unwrap();

    // Recursive clean should not follow directory symlink
    let config = CleanLockConfig {
        dir: scan_dir,
        recursive: true,
        older_than: None,
        dry_run: false,
    };

    let cleaned = clean_locks(&config).unwrap();

    // Nothing should be cleaned
    assert_eq!(cleaned.len(), 0);

    // External lock should still exist
    assert!(external_lock.exists());
}
