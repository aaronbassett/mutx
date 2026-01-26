use mutx::backup::{create_backup, BackupConfig};
use std::fs;
use tempfile::TempDir;

#[test]
fn test_backup_filename_format_with_timestamp() {
    let temp = TempDir::new().unwrap();
    let source = temp.path().join("data.txt");
    fs::write(&source, b"content").unwrap();

    let config = BackupConfig {
        source: source.clone(),
        suffix: ".mutx.backup".to_string(),
        directory: None,
        timestamp: true,
    };

    let backup_path = create_backup(&config).unwrap();
    let filename = backup_path.file_name().unwrap().to_str().unwrap();

    // Should match: data.txt.YYYYMMDD_HHMMSS.mutx.backup
    assert!(filename.starts_with("data.txt."));
    assert!(filename.ends_with(".mutx.backup"));

    // Extract timestamp part (between data.txt. and .mutx.backup)
    let parts: Vec<&str> = filename.split('.').collect();
    // parts: ["data", "txt", "YYYYMMDD_HHMMSS", "mutx", "backup"]
    assert_eq!(parts.len(), 5);

    let timestamp = parts[2];
    assert_eq!(timestamp.len(), 15); // YYYYMMDD_HHMMSS
    assert_eq!(timestamp.chars().nth(8), Some('_'));

    let date_part = &timestamp[..8];
    let time_part = &timestamp[9..];
    assert!(date_part.chars().all(|c| c.is_ascii_digit()));
    assert!(time_part.chars().all(|c| c.is_ascii_digit()));
}

#[test]
fn test_backup_filename_format_without_timestamp() {
    let temp = TempDir::new().unwrap();
    let source = temp.path().join("config.json");
    fs::write(&source, b"{}").unwrap();

    let config = BackupConfig {
        source,
        suffix: ".mutx.backup".to_string(),
        directory: None,
        timestamp: false,
    };

    let backup_path = create_backup(&config).unwrap();
    let filename = backup_path.file_name().unwrap().to_str().unwrap();

    // Without timestamp: config.json.mutx.backup
    assert_eq!(filename, "config.json.mutx.backup");
}
