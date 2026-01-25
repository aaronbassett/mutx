use std::io;
use std::path::PathBuf;
use std::time::Duration;
use thiserror::Error;

#[derive(Debug, Error)]
pub enum MutxError {
    #[error("Failed to acquire lock on {path}: timeout after {duration:?}")]
    LockTimeout { path: PathBuf, duration: Duration },

    #[error("Failed to acquire lock on {0}: file is locked by another process")]
    LockWouldBlock(PathBuf),

    #[error("Failed to create lock file {path}: {source}")]
    LockCreationFailed { path: PathBuf, source: io::Error },

    #[error("Failed to acquire lock on {path}: {source}")]
    LockAcquisitionFailed { path: PathBuf, source: io::Error },

    #[error("Failed to write to {path}: {source}")]
    WriteFailed { path: PathBuf, source: io::Error },

    #[error("Failed to create backup of {path}: {source}")]
    BackupFailed { path: PathBuf, source: io::Error },

    #[error("Failed to read from {path}: {source}")]
    ReadFailed { path: PathBuf, source: io::Error },

    #[error("Invalid duration format '{input}': {message}")]
    InvalidDuration { input: String, message: String },

    #[error("Invalid file permissions '{input}': must be octal (e.g., 0644)")]
    InvalidPermissions { input: String },

    #[error("Path does not exist: {0}")]
    PathNotFound(PathBuf),

    #[error("Path is not a file: {0}")]
    NotAFile(PathBuf),

    #[error("Path is not a directory: {0}")]
    NotADirectory(PathBuf),

    #[error("Operation interrupted")]
    Interrupted,

    #[error("Permission denied: {0}")]
    PermissionDenied(String),

    #[error(transparent)]
    Io(#[from] io::Error),

    #[error("{0}")]
    Other(String),
}

impl MutxError {
    pub fn exit_code(&self) -> i32 {
        match self {
            MutxError::LockTimeout { .. } | MutxError::LockWouldBlock(_) => 2,
            MutxError::Interrupted => 3,
            MutxError::PermissionDenied(_) => 1,
            MutxError::Io(e) if e.kind() == io::ErrorKind::PermissionDenied => 1,
            MutxError::Io(e) if e.kind() == io::ErrorKind::Interrupted => 3,
            _ => 1,
        }
    }

    pub fn lock_timeout(duration: Duration) -> Self {
        MutxError::LockTimeout {
            path: PathBuf::new(),
            duration,
        }
    }

    pub fn lock_would_block(path: impl Into<PathBuf>) -> Self {
        MutxError::LockWouldBlock(path.into())
    }
}

pub type Result<T> = std::result::Result<T, MutxError>;

// Maintain backward compatibility
