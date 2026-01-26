use mutx::backup::{create_backup, BackupConfig};
use std::fs;
use tempfile::TempDir;

#[test]
fn test_simple_backup_creation() {
    let dir = TempDir::new().unwrap();
    let target = dir.path().join("test.txt");
    fs::write(&target, "original content").unwrap();

    let config = BackupConfig {
        source: target.clone(),
        suffix: ".mutx.backup".to_string(),
        directory: None,
        timestamp: false,
    };

    create_backup(&config).unwrap();

    let backup_path = dir.path().join("test.txt.mutx.backup");
    assert!(backup_path.exists());
    assert_eq!(
        fs::read_to_string(&backup_path).unwrap(),
        "original content"
    );
}

#[test]
fn test_backup_with_timestamp() {
    let dir = TempDir::new().unwrap();
    let target = dir.path().join("test.txt");
    fs::write(&target, "original").unwrap();

    let config = BackupConfig {
        source: target.clone(),
        suffix: ".mutx.backup".to_string(),
        directory: None,
        timestamp: true,
    };

    let backup_path = create_backup(&config).unwrap();

    assert!(backup_path.exists());
    assert!(backup_path
        .file_name()
        .unwrap()
        .to_str()
        .unwrap()
        .contains("test.txt."));
    assert!(backup_path
        .file_name()
        .unwrap()
        .to_str()
        .unwrap()
        .contains(".mutx.backup"));
    assert_eq!(fs::read_to_string(&backup_path).unwrap(), "original");
}

#[test]
fn test_backup_to_directory() {
    let dir = TempDir::new().unwrap();
    let backup_dir = dir.path().join("backups");
    fs::create_dir(&backup_dir).unwrap();

    let target = dir.path().join("test.txt");
    fs::write(&target, "original").unwrap();

    let config = BackupConfig {
        source: target.clone(),
        suffix: ".mutx.backup".to_string(),
        directory: Some(backup_dir.clone()),
        timestamp: false,
    };

    create_backup(&config).unwrap();

    let backup_path = backup_dir.join("test.txt.mutx.backup");
    assert!(backup_path.exists());
    assert_eq!(fs::read_to_string(&backup_path).unwrap(), "original");
}

#[test]
fn test_backup_nonexistent_file_fails() {
    let dir = TempDir::new().unwrap();
    let target = dir.path().join("nonexistent.txt");

    let config = BackupConfig {
        source: target.clone(),
        suffix: ".mutx.backup".to_string(),
        directory: None,
        timestamp: false,
    };

    let result = create_backup(&config);
    assert!(result.is_err());
}
