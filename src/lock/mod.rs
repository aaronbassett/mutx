mod acquisition;
mod path;

pub use acquisition::{FileLock, LockStrategy, TimeoutConfig};
pub use path::{derive_lock_path, get_lock_cache_dir, validate_lock_path};
