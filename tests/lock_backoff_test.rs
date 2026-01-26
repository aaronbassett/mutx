use mutx::lock::{FileLock, LockStrategy, TimeoutConfig};
use std::sync::{Arc, Mutex};
use std::thread;
use std::time::{Duration, Instant};
use tempfile::TempDir;

#[test]
fn test_exponential_backoff_timing() {
    let temp = TempDir::new().unwrap();
    let lock_path = temp.path().join("test.lock");

    // Hold a lock in another thread
    let lock_path_clone = lock_path.clone();
    let release_signal = Arc::new(Mutex::new(false));
    let release_clone = release_signal.clone();

    let holder = thread::spawn(move || {
        let _lock = FileLock::acquire(&lock_path_clone, LockStrategy::Wait).unwrap();

        // Wait for signal to release
        loop {
            thread::sleep(Duration::from_millis(50));
            if *release_clone.lock().unwrap() {
                break;
            }
        }
    });

    // Give the holder time to acquire
    thread::sleep(Duration::from_millis(100));

    // Try to acquire with short timeout
    let config = TimeoutConfig::new(Duration::from_millis(500));
    let start = Instant::now();
    let result = FileLock::acquire(&lock_path, LockStrategy::Timeout(config));
    let elapsed = start.elapsed();

    // Should timeout around 500ms (allow some variance for CI environments)
    assert!(result.is_err());
    assert!(elapsed >= Duration::from_millis(450));
    assert!(elapsed <= Duration::from_millis(800));

    // Signal release and cleanup
    *release_signal.lock().unwrap() = true;
    holder.join().unwrap();
}

#[test]
fn test_max_poll_interval_respected() {
    let temp = TempDir::new().unwrap();
    let lock_path = temp.path().join("test.lock");

    // Hold a lock
    let _holder = FileLock::acquire(&lock_path, LockStrategy::Wait).unwrap();

    // Try with custom max interval
    let config = TimeoutConfig::new(Duration::from_millis(2000))
        .with_max_interval(Duration::from_millis(100));

    let start = Instant::now();
    let result = FileLock::acquire(&lock_path, LockStrategy::Timeout(config));
    let elapsed = start.elapsed();

    // Should timeout
    assert!(result.is_err());

    // With 100ms max interval + jitter, should take roughly 2 seconds
    // (multiple attempts at ~100-200ms each)
    assert!(elapsed >= Duration::from_millis(1800));
    assert!(elapsed <= Duration::from_millis(2300));
}
