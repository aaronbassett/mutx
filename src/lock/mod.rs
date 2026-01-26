mod acquisition;
mod path;

pub use acquisition::{FileLock, LockStrategy, TimeoutConfig};
pub use path::{derive_lock_path, validate_lock_path};
