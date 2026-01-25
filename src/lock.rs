use anyhow::{bail, Context, Result};
use fs2::FileExt;
use std::fs::{File, OpenOptions};
use std::io;
use std::path::{Path, PathBuf};
use std::time::{Duration, Instant};

#[derive(Debug, Clone)]
pub enum LockStrategy {
    Wait,
    NoWait,
    Timeout(Duration),
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
        // Create lock file
        let file = OpenOptions::new()
            .create(true)
            .write(true)
            .truncate(true)
            .open(lock_path)
            .with_context(|| format!("Failed to create lock file: {}", lock_path.display()))?;

        // Acquire lock based on strategy
        match strategy {
            LockStrategy::Wait => {
                file.lock_exclusive()
                    .with_context(|| format!("Failed to acquire lock: {}", lock_path.display()))?;
            }
            LockStrategy::NoWait => {
                file.try_lock_exclusive()
                    .map_err(|e| match e.kind() {
                        io::ErrorKind::WouldBlock => {
                            anyhow::anyhow!("File locked by another process")
                        }
                        _ => anyhow::Error::new(e),
                    })
                    .with_context(|| format!("Failed to acquire lock: {}", lock_path.display()))?;
            }
            LockStrategy::Timeout(duration) => {
                let start = Instant::now();
                loop {
                    match file.try_lock_exclusive() {
                        Ok(_) => break,
                        Err(e) if e.kind() == io::ErrorKind::WouldBlock => {
                            if start.elapsed() >= duration {
                                bail!("Lock acquisition timeout after {}s", duration.as_secs());
                            }
                            std::thread::sleep(Duration::from_millis(100));
                        }
                        Err(e) => {
                            return Err(e).with_context(|| {
                                format!("Failed to acquire lock: {}", lock_path.display())
                            });
                        }
                    }
                }
            }
        }

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
        // Unlock is automatic when file handle is dropped
        // Try to delete lock file (best effort)
        let _ = std::fs::remove_file(&self.path);
    }
}
