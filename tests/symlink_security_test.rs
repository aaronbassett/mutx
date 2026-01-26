use mutx::utils::{check_lock_symlink, check_symlink};
use mutx::MutxError;
use std::fs;
use tempfile::TempDir;

#[test]
#[cfg(unix)]
fn test_rejects_symlink_by_default() {
    use std::os::unix::fs as unix_fs;

    let temp = TempDir::new().unwrap();
    let real_file = temp.path().join("real.txt");
    let symlink = temp.path().join("link.txt");

    fs::write(&real_file, b"data").unwrap();
    unix_fs::symlink(&real_file, &symlink).unwrap();

    let result = check_symlink(&symlink, false);
    assert!(result.is_err());
    assert!(matches!(
        result.unwrap_err(),
        MutxError::SymlinkNotAllowed { .. }
    ));
}

#[test]
#[cfg(unix)]
fn test_allows_symlink_when_enabled() {
    use std::os::unix::fs as unix_fs;

    let temp = TempDir::new().unwrap();
    let real_file = temp.path().join("real.txt");
    let symlink = temp.path().join("link.txt");

    fs::write(&real_file, b"data").unwrap();
    unix_fs::symlink(&real_file, &symlink).unwrap();

    let result = check_symlink(&symlink, true);
    assert!(result.is_ok());
}

#[test]
fn test_allows_regular_file() {
    let temp = TempDir::new().unwrap();
    let file = temp.path().join("regular.txt");
    fs::write(&file, b"data").unwrap();

    let result = check_symlink(&file, false);
    assert!(result.is_ok());
}

#[test]
fn test_allows_nonexistent_file() {
    let temp = TempDir::new().unwrap();
    let file = temp.path().join("nonexistent.txt");

    let result = check_symlink(&file, false);
    assert!(result.is_ok());
}

#[test]
#[cfg(unix)]
fn test_lock_symlink_rejected_by_default() {
    use std::os::unix::fs as unix_fs;

    let temp = TempDir::new().unwrap();
    let real_file = temp.path().join("real.lock");
    let symlink = temp.path().join("link.lock");

    fs::write(&real_file, b"").unwrap();
    unix_fs::symlink(&real_file, &symlink).unwrap();

    let result = check_lock_symlink(&symlink, false);
    assert!(result.is_err());
    assert!(matches!(
        result.unwrap_err(),
        MutxError::LockSymlinkNotAllowed { .. }
    ));
}

#[test]
#[cfg(unix)]
fn test_lock_symlink_allowed_with_flag() {
    use std::os::unix::fs as unix_fs;

    let temp = TempDir::new().unwrap();
    let real_file = temp.path().join("real.lock");
    let symlink = temp.path().join("link.lock");

    fs::write(&real_file, b"").unwrap();
    unix_fs::symlink(&real_file, &symlink).unwrap();

    let result = check_lock_symlink(&symlink, true);
    assert!(result.is_ok());
}
