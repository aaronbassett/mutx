use mutx::lock::{FileLock, LockStrategy, TimeoutConfig};
use std::time::Duration;
use tempfile::NamedTempFile;

#[test]
fn test_lock_acquire_and_release() {
    let temp = NamedTempFile::new().unwrap();
    let lock_path = temp.path().with_extension("lock");

    let lock = FileLock::acquire(&lock_path, LockStrategy::Wait).unwrap();
    assert!(lock_path.exists());

    drop(lock);
    // Lock file now persists after release (changed behavior in v1.1.0)
    assert!(lock_path.exists(), "Lock file should persist for proper mutual exclusion");
}

#[test]
fn test_lock_no_wait_fails_when_locked() {
    let temp = NamedTempFile::new().unwrap();
    let lock_path = temp.path().with_extension("lock");

    let _lock1 = FileLock::acquire(&lock_path, LockStrategy::Wait).unwrap();

    let result = FileLock::acquire(&lock_path, LockStrategy::NoWait);
    assert!(result.is_err());
    let err_msg = result.unwrap_err().to_string();
    assert!(
        err_msg.contains("Failed to acquire lock") || err_msg.contains("locked"),
        "Expected error about lock failure, got: {}",
        err_msg
    );
}

#[test]
fn test_lock_timeout() {
    let temp = NamedTempFile::new().unwrap();
    let lock_path = temp.path().with_extension("lock");

    let _lock1 = FileLock::acquire(&lock_path, LockStrategy::Wait).unwrap();

    let start = std::time::Instant::now();
    let config = TimeoutConfig::new(Duration::from_millis(1000));
    let result = FileLock::acquire(&lock_path, LockStrategy::Timeout(config));
    let elapsed = start.elapsed();

    assert!(result.is_err());
    assert!(elapsed >= Duration::from_millis(900));  // Allow some variance
    assert!(elapsed < Duration::from_millis(1500));
}
