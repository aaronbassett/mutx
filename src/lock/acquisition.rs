use crate::error::{MutxError, Result};
use fs2::FileExt;
use rand::Rng;
use std::fs::{File, OpenOptions};
use std::io;
use std::path::{Path, PathBuf};
use std::time::{Duration, Instant};
use tracing::debug;

/// Check if an I/O error indicates lock contention (file locked by another process)
fn is_lock_contention(e: &io::Error) -> bool {
    // Check for WouldBlock (Unix)
    if e.kind() == io::ErrorKind::WouldBlock {
        return true;
    }
    // Check for Windows-specific lock errors
    // ERROR_LOCK_VIOLATION (33) - file region is locked
    // ERROR_SHARING_VIOLATION (32) - file in use by another process
    #[cfg(windows)]
    if let Some(code) = e.raw_os_error() {
        if code == 33 || code == 32 {
            return true;
        }
    }
    false
}

#[derive(Debug, Clone)]
pub struct TimeoutConfig {
    pub duration: Duration,
    pub max_poll_interval: Duration,
}

impl TimeoutConfig {
    pub fn new(duration: Duration) -> Self {
        Self {
            duration,
            max_poll_interval: Duration::from_millis(1000),
        }
    }

    pub fn with_max_interval(mut self, max_interval: Duration) -> Self {
        self.max_poll_interval = max_interval;
        self
    }
}

#[derive(Debug, Clone)]
pub enum LockStrategy {
    Wait,
    NoWait,
    Timeout(TimeoutConfig),
}

#[derive(Debug)]
pub struct FileLock {
    #[allow(dead_code)]
    file: File,
    path: PathBuf,
}

impl FileLock {
    /// Acquire an exclusive lock on the specified file
    pub fn acquire(lock_path: &Path, strategy: LockStrategy) -> Result<Self> {
        debug!(
            "Acquiring lock: {} (strategy: {:?})",
            lock_path.display(),
            strategy
        );

        // Create lock file
        let mut opts = OpenOptions::new();
        opts.create(true).write(true).truncate(true);

        // On Unix, use O_NOFOLLOW to reject symlinks at OS level
        #[cfg(unix)]
        {
            use std::os::unix::fs::OpenOptionsExt;
            opts.custom_flags(libc::O_NOFOLLOW);
        }

        let file = opts
            .open(lock_path)
            .map_err(|e| MutxError::LockCreationFailed {
                path: lock_path.to_path_buf(),
                source: e,
            })?;

        // Acquire lock based on strategy
        match strategy {
            LockStrategy::Wait => {
                file.lock_exclusive()
                    .map_err(|e| MutxError::LockAcquisitionFailed {
                        path: lock_path.to_path_buf(),
                        source: e,
                    })?;
            }
            LockStrategy::NoWait => {
                file.try_lock_exclusive().map_err(|e| {
                    if is_lock_contention(&e) {
                        MutxError::LockWouldBlock(lock_path.to_path_buf())
                    } else {
                        MutxError::LockAcquisitionFailed {
                            path: lock_path.to_path_buf(),
                            source: e,
                        }
                    }
                })?;
            }
            LockStrategy::Timeout(config) => {
                let start = Instant::now();
                let mut current_interval = Duration::from_millis(10);
                let mut rng = rand::thread_rng();

                loop {
                    match file.try_lock_exclusive() {
                        Ok(_) => break,
                        Err(e) if is_lock_contention(&e) => {
                            if start.elapsed() >= config.duration {
                                return Err(MutxError::LockTimeout {
                                    path: lock_path.to_path_buf(),
                                    duration: config.duration,
                                });
                            }

                            // Calculate sleep time with backoff + jitter
                            let base_interval = current_interval.min(config.max_poll_interval);
                            let jitter = Duration::from_millis(rng.gen_range(0..100));
                            let sleep_time = base_interval + jitter;

                            std::thread::sleep(sleep_time);

                            // Exponential backoff for next iteration (1.5x multiplier)
                            current_interval = Duration::from_millis(
                                (current_interval.as_millis() as f64 * 1.5) as u64,
                            );
                        }
                        Err(e) => {
                            return Err(MutxError::LockAcquisitionFailed {
                                path: lock_path.to_path_buf(),
                                source: e,
                            });
                        }
                    }
                }
            }
        }

        debug!("Lock acquired: {}", lock_path.display());

        Ok(FileLock {
            file,
            path: lock_path.to_path_buf(),
        })
    }

    /// Get the lock file path
    pub fn path(&self) -> &Path {
        &self.path
    }
}

impl Drop for FileLock {
    fn drop(&mut self) {
        // Lock is automatically released when file handle is dropped
        // We do NOT delete the lock file - it persists for proper mutual exclusion
        // Run `mutx housekeep --locks` to clean orphaned locks
        debug!("Lock released (file persists): {}", self.path.display());
    }
}
