mod types;

pub use types::{MutxError, Result};

// Re-export for convenience
pub use MutxError as Error;
pub use MutxError as ErrorKind;
