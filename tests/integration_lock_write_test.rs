use mutx::{AtomicWriter, FileLock, LockStrategy, WriteMode};
use std::fs;
use std::io::Write;
use tempfile::TempDir;

#[test]
fn test_lock_and_write_integration() {
    let dir = TempDir::new().unwrap();
    let target = dir.path().join("test.txt");
    let lock_path = target.with_extension("lock");

    let _lock = FileLock::acquire(&lock_path, LockStrategy::Wait).unwrap();

    let mut writer = AtomicWriter::new(&target, WriteMode::Simple).unwrap();
    writer.write_all(b"locked write").unwrap();
    writer.commit().unwrap();

    assert_eq!(fs::read_to_string(&target).unwrap(), "locked write");

    drop(_lock);
    assert!(!lock_path.exists());
}

#[test]
fn test_concurrent_write_blocks() {
    use std::sync::Arc;
    use std::sync::atomic::{AtomicBool, Ordering};
    use std::thread;
    use std::time::Duration;

    let dir = TempDir::new().unwrap();
    let target = Arc::new(dir.path().join("test.txt"));
    let lock_path = target.with_extension("lock");

    let blocked = Arc::new(AtomicBool::new(false));
    let blocked_clone = blocked.clone();
    let target_clone = target.clone();

    // Thread 1: Hold lock for 1 second
    let t1 = thread::spawn(move || {
        let _lock = FileLock::acquire(&lock_path, LockStrategy::Wait).unwrap();
        thread::sleep(Duration::from_millis(500));
    });

    // Give thread 1 time to acquire lock
    thread::sleep(Duration::from_millis(100));

    // Thread 2: Try to acquire with no-wait
    let t2 = thread::spawn(move || {
        let lock_path2 = target_clone.with_extension("lock");
        let result = FileLock::acquire(&lock_path2, LockStrategy::NoWait);
        if result.is_err() {
            blocked_clone.store(true, Ordering::SeqCst);
        }
    });

    t1.join().unwrap();
    t2.join().unwrap();

    assert!(blocked.load(Ordering::SeqCst), "Second lock should have been blocked");
}
