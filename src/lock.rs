use crate::error::{MutxError, Result};
use fs2::FileExt;
use std::fs::{File, OpenOptions};
use std::io;
use std::path::{Path, PathBuf};
use std::time::{Duration, Instant};
use tracing::{debug, warn};

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
        debug!("Acquiring lock: {} (strategy: {:?})", lock_path.display(), strategy);

        // Create lock file
        let file = OpenOptions::new()
            .create(true)
            .write(true)
            .truncate(true)
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
                file.try_lock_exclusive()
                    .map_err(|e| match e.kind() {
                        io::ErrorKind::WouldBlock => {
                            MutxError::LockWouldBlock(lock_path.to_path_buf())
                        }
                        _ => MutxError::LockAcquisitionFailed {
                            path: lock_path.to_path_buf(),
                            source: e,
                        },
                    })?;
            }
            LockStrategy::Timeout(duration) => {
                let start = Instant::now();
                loop {
                    match file.try_lock_exclusive() {
                        Ok(_) => break,
                        Err(e) if e.kind() == io::ErrorKind::WouldBlock => {
                            if start.elapsed() >= duration {
                                return Err(MutxError::LockTimeout {
                                    path: lock_path.to_path_buf(),
                                    duration,
                                });
                            }
                            std::thread::sleep(Duration::from_millis(100));
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
        // Unlock is automatic when file handle is dropped
        // Try to delete lock file (best effort, never panic)
        match std::fs::remove_file(&self.path) {
            Ok(_) => debug!("Lock file removed: {}", self.path.display()),
            Err(e) => {
                // Log but don't fail - lock is already released
                warn!(
                    "Failed to remove lock file {} (non-fatal): {}",
                    self.path.display(),
                    e
                );
            }
        }
    }
}
