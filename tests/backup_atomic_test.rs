use std::fs;
use tempfile::TempDir;

#[test]
fn test_backup_is_atomic() {
    let temp = TempDir::new().unwrap();
    let source = temp.path().join("source.txt");

    // Create source file
    fs::write(&source, b"original content").unwrap();

    // Simulate failure during backup by making temp directory read-only
    // Backup should either fully succeed or leave no partial files

    // This test will be implemented after we fix the backup module
    // For now, we just verify the module compiles with new signature
}

#[test]
fn test_backup_failure_leaves_no_artifacts() {
    let temp = TempDir::new().unwrap();
    let source = temp.path().join("source.txt");
    let backup_dir = temp.path().join("backups");

    fs::create_dir(&backup_dir).unwrap();
    fs::write(&source, b"content").unwrap();

    // Make backup directory read-only to force failure
    #[cfg(unix)]
    {
        use std::os::unix::fs::PermissionsExt;
        let mut perms = fs::metadata(&backup_dir).unwrap().permissions();
        perms.set_mode(0o444);
        fs::set_permissions(&backup_dir, perms).unwrap();
    }

    // Backup should fail, but not leave partial files
    // We'll verify this after implementing atomic backup
}
