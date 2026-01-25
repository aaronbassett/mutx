use mutx::backup::{BackupConfig, create_backup};
use std::fs;
use tempfile::TempDir;

#[test]
fn test_simple_backup_creation() {
    let dir = TempDir::new().unwrap();
    let target = dir.path().join("test.txt");
    fs::write(&target, "original content").unwrap();

    let config = BackupConfig {
        suffix: ".backup".to_string(),
        timestamp: false,
        backup_dir: None,
    };

    create_backup(&target, &config).unwrap();

    let backup_path = target.with_extension("txt.backup");
    assert!(backup_path.exists());
    assert_eq!(fs::read_to_string(&backup_path).unwrap(), "original content");
}

#[test]
fn test_backup_with_timestamp() {
    let dir = TempDir::new().unwrap();
    let target = dir.path().join("test.txt");
    fs::write(&target, "original").unwrap();

    let config = BackupConfig {
        suffix: ".backup".to_string(),
        timestamp: true,
        backup_dir: None,
    };

    let backup_path = create_backup(&target, &config).unwrap();

    assert!(backup_path.exists());
    assert!(backup_path.file_name().unwrap().to_str().unwrap().contains("test.txt."));
    assert!(backup_path.file_name().unwrap().to_str().unwrap().contains(".backup"));
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
        suffix: ".backup".to_string(),
        timestamp: false,
        backup_dir: Some(backup_dir.clone()),
    };

    create_backup(&target, &config).unwrap();

    let backup_path = backup_dir.join("test.txt.backup");
    assert!(backup_path.exists());
    assert_eq!(fs::read_to_string(&backup_path).unwrap(), "original");
}

#[test]
fn test_backup_nonexistent_file_skips() {
    let dir = TempDir::new().unwrap();
    let target = dir.path().join("nonexistent.txt");

    let config = BackupConfig {
        suffix: ".backup".to_string(),
        timestamp: false,
        backup_dir: None,
    };

    let result = create_backup(&target, &config);
    assert!(result.is_ok());

    let backup_path = target.with_extension("txt.backup");
    assert!(!backup_path.exists());
}
