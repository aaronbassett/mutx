use anyhow::Error;
use std::fmt;

#[derive(Debug)]
pub enum ErrorKind {
    LockFailed,
    Timeout,
    PermissionDenied,
    Interrupted,
    General,
}

pub struct AppError {
    kind: ErrorKind,
    source: Error,
}

impl AppError {
    pub fn new(kind: ErrorKind, source: Error) -> Self {
        AppError { kind, source }
    }

    pub fn exit_code(&self) -> i32 {
        match self.kind {
            ErrorKind::LockFailed | ErrorKind::Timeout => 2,
            ErrorKind::Interrupted => 3,
            ErrorKind::PermissionDenied | ErrorKind::General => 1,
        }
    }

    pub fn from_anyhow(err: Error) -> Self {
        let msg = err.to_string().to_lowercase();

        let kind = if msg.contains("lock") && msg.contains("timeout") {
            ErrorKind::Timeout
        } else if msg.contains("failed to acquire lock") || msg.contains("locked") || msg.contains("would block") {
            ErrorKind::LockFailed
        } else if msg.contains("permission denied") {
            ErrorKind::PermissionDenied
        } else if msg.contains("interrupt") {
            ErrorKind::Interrupted
        } else {
            ErrorKind::General
        };

        AppError { kind, source: err }
    }
}

impl fmt::Display for AppError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{:?}", self.source)
    }
}

impl fmt::Debug for AppError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{}", self.source)
    }
}
