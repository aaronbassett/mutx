use mutx::lock::{FileLock, LockStrategy};
use tempfile::NamedTempFile;
use std::time::Duration;

#[test]
fn test_lock_acquire_and_release() {
    let temp = NamedTempFile::new().unwrap();
    let lock_path = temp.path().with_extension("lock");

    let lock = FileLock::acquire(&lock_path, LockStrategy::Wait).unwrap();
    assert!(lock_path.exists());

    drop(lock);
    assert!(!lock_path.exists(), "Lock file should be cleaned up");
}

#[test]
fn test_lock_no_wait_fails_when_locked() {
    let temp = NamedTempFile::new().unwrap();
    let lock_path = temp.path().with_extension("lock");

    let _lock1 = FileLock::acquire(&lock_path, LockStrategy::Wait).unwrap();

    let result = FileLock::acquire(&lock_path, LockStrategy::NoWait);
    assert!(result.is_err());
    let err_msg = result.unwrap_err().to_string();
    assert!(err_msg.contains("Failed to acquire lock") || err_msg.contains("locked"),
            "Expected error about lock failure, got: {}", err_msg);
}

#[test]
fn test_lock_timeout() {
    let temp = NamedTempFile::new().unwrap();
    let lock_path = temp.path().with_extension("lock");

    let _lock1 = FileLock::acquire(&lock_path, LockStrategy::Wait).unwrap();

    let start = std::time::Instant::now();
    let result = FileLock::acquire(
        &lock_path,
        LockStrategy::Timeout(Duration::from_secs(1))
    );
    let elapsed = start.elapsed();

    assert!(result.is_err());
    assert!(elapsed >= Duration::from_secs(1));
    assert!(elapsed < Duration::from_secs(2));
}
