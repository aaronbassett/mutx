//! Atomic file write library with file locking support

pub mod lock;
pub mod write;
pub mod backup;

pub use lock::{FileLock, LockStrategy};
pub use write::{AtomicWriter, WriteMode};
pub use backup::{BackupConfig, create_backup};
