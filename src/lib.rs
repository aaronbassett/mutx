//! Atomic file write library with file locking support

pub mod lock;

pub use lock::{FileLock, LockStrategy};
