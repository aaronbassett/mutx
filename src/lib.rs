//! Atomic file write library with file locking support

pub mod lock;
pub mod write;
pub mod backup;
pub mod housekeep;
pub mod error;

pub use lock::{FileLock, LockStrategy};
pub use write::{AtomicWriter, WriteMode};
pub use backup::{BackupConfig, create_backup};
pub use housekeep::{clean_locks, clean_backups, CleanLockConfig, CleanBackupConfig};
