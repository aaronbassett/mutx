use mutx::backup::{create_backup, validate_backup_suffix, BackupConfig};
use mutx::MutxError;
use std::fs;
use tempfile::TempDir;

#[test]
fn test_validate_backup_suffix_rejects_empty() {
    let result = validate_backup_suffix("");
    assert!(result.is_err());
    match result.unwrap_err() {
        MutxError::Other(msg) => assert_eq!(msg, "Backup suffix cannot be empty"),
        _ => panic!("Expected Other error"),
    }
}

#[test]
fn test_validate_backup_suffix_rejects_single_dot() {
    let result = validate_backup_suffix(".");
    assert!(result.is_err());
    match result.unwrap_err() {
        MutxError::Other(msg) => assert_eq!(msg, "Backup suffix cannot be a single dot"),
        _ => panic!("Expected Other error"),
    }
}

#[test]
fn test_validate_backup_suffix_accepts_valid() {
    assert!(validate_backup_suffix(".bak").is_ok());
    assert!(validate_backup_suffix(".backup").is_ok());
    assert!(validate_backup_suffix("~").is_ok());
}

#[test]
fn test_create_backup_rejects_empty_suffix() {
    let dir = TempDir::new().unwrap();
    let target = dir.path().join("test.txt");
    fs::write(&target, "original content").unwrap();

    let config = BackupConfig {
        source: target.clone(),
        suffix: String::new(), // empty suffix
        directory: None,
        timestamp: false,
    };

    let result = create_backup(&config);
    assert!(result.is_err());
    match result.unwrap_err() {
        MutxError::Other(msg) => assert!(msg.contains("Backup suffix cannot be empty")),
        _ => panic!("Expected Other error for empty suffix"),
    }
}

#[test]
fn test_create_backup_rejects_single_dot_suffix() {
    let dir = TempDir::new().unwrap();
    let target = dir.path().join("test.txt");
    fs::write(&target, "original content").unwrap();

    let config = BackupConfig {
        source: target.clone(),
        suffix: ".".to_string(), // single dot
        directory: None,
        timestamp: false,
    };

    let result = create_backup(&config);
    assert!(result.is_err());
    match result.unwrap_err() {
        MutxError::Other(msg) => assert!(msg.contains("Backup suffix cannot be a single dot")),
        _ => panic!("Expected Other error for single dot suffix"),
    }
}

#[test]
fn test_create_backup_accepts_valid_suffix() {
    let dir = TempDir::new().unwrap();
    let target = dir.path().join("test.txt");
    fs::write(&target, "original content").unwrap();

    let config = BackupConfig {
        source: target.clone(),
        suffix: ".bak".to_string(),
        directory: None,
        timestamp: false,
    };

    let result = create_backup(&config);
    assert!(result.is_ok());
    let backup_path = dir.path().join("test.txt.bak");
    assert!(backup_path.exists());
    assert_eq!(
        fs::read_to_string(&backup_path).unwrap(),
        "original content"
    );
}
