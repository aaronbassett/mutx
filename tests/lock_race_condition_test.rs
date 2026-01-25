use mutx::lock::{FileLock, LockStrategy};
use std::fs;
use std::path::PathBuf;
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::Arc;
use std::thread;
use std::time::Duration;
use tempfile::TempDir;

#[test]
fn test_lock_cleanup_race_condition() {
    let temp = TempDir::new().unwrap();
    let lock_path = temp.path().join("test.lock");

    let success = Arc::new(AtomicBool::new(false));
    let success_clone = success.clone();

    // Thread 1: Acquire and hold lock
    let lock_path_clone = lock_path.clone();
    let t1 = thread::spawn(move || {
        let _lock = FileLock::acquire(&lock_path_clone, LockStrategy::Wait).unwrap();
        thread::sleep(Duration::from_millis(100));
        // Lock dropped here
    });

    thread::sleep(Duration::from_millis(50));

    // Thread 2: Try to acquire same lock
    let t2 = thread::spawn(move || {
        thread::sleep(Duration::from_millis(200));
        // After thread 1 drops, this should succeed
        let _lock = FileLock::acquire(&lock_path, LockStrategy::Wait).unwrap();
        success_clone.store(true, Ordering::SeqCst);
    });

    t1.join().unwrap();
    t2.join().unwrap();

    assert!(
        success.load(Ordering::SeqCst),
        "Second lock acquisition should succeed"
    );
}

#[test]
fn test_lock_drop_always_succeeds() {
    let temp = TempDir::new().unwrap();
    let lock_path = temp.path().join("test.lock");

    {
        let _lock = FileLock::acquire(&lock_path, LockStrategy::Wait).unwrap();
        // Make lock file read-only to simulate permission issues
        #[cfg(unix)]
        {
            use std::os::unix::fs::PermissionsExt;
            let mut perms = fs::metadata(&lock_path).unwrap().permissions();
            perms.set_mode(0o444);
            fs::set_permissions(&lock_path, perms).unwrap();
        }
    } // Drop should not panic even if removal fails

    // Test passes if we get here without panic
}
