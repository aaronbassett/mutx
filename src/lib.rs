//! Atomic file write library with file locking support

pub mod backup;
pub mod error;
pub mod housekeep;
pub mod lock;
pub mod write;

pub use backup::{create_backup, BackupConfig};
pub use housekeep::{clean_backups, clean_locks, CleanBackupConfig, CleanLockConfig};
pub use lock::{FileLock, LockStrategy};
pub use write::{AtomicWriter, WriteMode};
