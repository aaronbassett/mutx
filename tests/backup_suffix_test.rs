use mutx::backup::{create_backup, BackupConfig};
use std::fs;
use tempfile::TempDir;

#[test]
fn test_custom_suffix_without_timestamp() {
    let dir = TempDir::new().unwrap();
    let target = dir.path().join("test.txt");
    fs::write(&target, "original content").unwrap();

    let config = BackupConfig {
        source: target.clone(),
        suffix: ".bak".to_string(),
        directory: None,
        timestamp: false,
    };

    let backup_path = create_backup(&config).unwrap();

    assert_eq!(
        backup_path.file_name().unwrap().to_str().unwrap(),
        "test.txt.bak"
    );
    assert!(backup_path.exists());
    assert_eq!(
        fs::read_to_string(&backup_path).unwrap(),
        "original content"
    );
}

#[test]
fn test_custom_suffix_with_timestamp() {
    let dir = TempDir::new().unwrap();
    let target = dir.path().join("test.txt");
    fs::write(&target, "original content").unwrap();

    let config = BackupConfig {
        source: target.clone(),
        suffix: ".bak".to_string(),
        directory: None,
        timestamp: true,
    };

    let backup_path = create_backup(&config).unwrap();

    let filename = backup_path.file_name().unwrap().to_str().unwrap();
    assert!(filename.starts_with("test.txt."));
    assert!(filename.ends_with(".bak"));
    assert!(backup_path.exists());
    assert_eq!(
        fs::read_to_string(&backup_path).unwrap(),
        "original content"
    );
}

#[test]
fn test_default_suffix() {
    let dir = TempDir::new().unwrap();
    let target = dir.path().join("test.txt");
    fs::write(&target, "original content").unwrap();

    let config = BackupConfig {
        source: target.clone(),
        suffix: ".mutx.backup".to_string(),
        directory: None,
        timestamp: false,
    };

    let backup_path = create_backup(&config).unwrap();

    assert_eq!(
        backup_path.file_name().unwrap().to_str().unwrap(),
        "test.txt.mutx.backup"
    );
    assert!(backup_path.exists());
}
