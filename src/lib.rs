//! Atomic file write library with file locking support

pub mod error;
pub mod lock;
pub mod write;
pub mod backup;
pub mod housekeep;

// Re-export for convenience
pub use error::{MutxError, Result};
pub use backup::{create_backup, BackupConfig};
pub use housekeep::{clean_backups, clean_locks, CleanBackupConfig, CleanLockConfig};
pub use lock::{FileLock, LockStrategy};
pub use write::{AtomicWriter, WriteMode};
