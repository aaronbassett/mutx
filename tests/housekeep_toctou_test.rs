use mutx::housekeep::{clean_locks, CleanLockConfig};
use std::fs::File;
use std::thread;
use std::time::Duration;
use tempfile::TempDir;

#[test]
fn test_clean_locks_handles_concurrent_deletion() {
    let temp = TempDir::new().unwrap();
    let lock_path = temp.path().join("test.lock");

    // Create a lock file
    File::create(&lock_path).unwrap();

    let config = CleanLockConfig {
        dir: temp.path().to_path_buf(),
        recursive: false,
        older_than: None,
        dry_run: false,
    };

    // Start cleanup in background
    let config_clone = config.clone();
    let handle = thread::spawn(move || clean_locks(&config_clone));

    // Delete the file while cleanup is running (simulate TOCTOU)
    thread::sleep(Duration::from_millis(10));
    let _ = std::fs::remove_file(&lock_path);

    // Should not panic, should handle gracefully
    let result = handle.join().unwrap();
    assert!(
        result.is_ok(),
        "Cleanup should handle concurrent deletion gracefully"
    );
}
