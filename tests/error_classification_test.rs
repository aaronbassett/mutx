use mutx::error::{MutxError, ErrorKind};
use std::io;

#[test]
fn test_lock_timeout_error_classification() {
    let err = MutxError::lock_timeout(std::time::Duration::from_secs(5));
    assert_eq!(err.exit_code(), 2);
}

#[test]
fn test_lock_would_block_error_classification() {
    let err = MutxError::lock_would_block("test.lock");
    assert_eq!(err.exit_code(), 2);
}

#[test]
fn test_permission_error_classification() {
    let io_err = io::Error::from(io::ErrorKind::PermissionDenied);
    let err = MutxError::from(io_err);
    assert_eq!(err.exit_code(), 1);
}

#[test]
fn test_interrupted_error_classification() {
    let io_err = io::Error::from(io::ErrorKind::Interrupted);
    let err = MutxError::from(io_err);
    assert_eq!(err.exit_code(), 3);
}
