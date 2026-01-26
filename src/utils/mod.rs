mod duration;
pub mod symlink;

pub use duration::parse_duration;
pub use symlink::{check_lock_symlink, check_symlink};
