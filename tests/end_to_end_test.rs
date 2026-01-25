use assert_cmd::Command;
use predicates::prelude::*;
use std::fs;
use std::thread;
use std::time::Duration;
use tempfile::TempDir;

#[test]
fn test_full_workflow_with_backup_and_lock() {
    let dir = TempDir::new().unwrap();
    let target = dir.path().join("config.json");

    // Initial write
    fs::write(&target, r#"{"version": 1}"#).unwrap();

    // Update with backup
    let mut cmd = Command::cargo_bin("mutx").unwrap();
    cmd.arg("--backup")
        .arg("--backup-timestamp")
        .arg("-v")
        .arg(&target)
        .write_stdin(r#"{"version": 2}"#)
        .assert()
        .success()
        .stderr(predicate::str::contains("Lock acquired"))
        .stderr(predicate::str::contains("Backup created"));

    // Verify content updated
    assert_eq!(fs::read_to_string(&target).unwrap(), r#"{"version": 2}"#);

    // Verify backup exists
    let backups: Vec<_> = fs::read_dir(dir.path())
        .unwrap()
        .filter_map(|e| e.ok())
        .filter(|e| e.path().to_str().unwrap().contains("backup"))
        .collect();
    assert_eq!(backups.len(), 1);
}

#[test]
fn test_concurrent_writers_with_locking() {
    let dir = TempDir::new().unwrap();
    let target = dir.path().join("counter.txt");
    fs::write(&target, "0").unwrap();

    let handles: Vec<_> = (0..5)
        .map(|i| {
            let target = target.clone();
            thread::spawn(move || {
                thread::sleep(Duration::from_millis(i * 100));

                let mut cmd = Command::cargo_bin("mutx").unwrap();
                cmd.arg(&target)
                    .write_stdin(format!("writer-{}", i))
                    .assert()
                    .success();
            })
        })
        .collect();

    for h in handles {
        h.join().unwrap();
    }

    // File should have one writer's content (not corrupted)
    let content = fs::read_to_string(&target).unwrap();
    assert!(content.starts_with("writer-"));
}

#[test]
fn test_streaming_large_file() {
    let dir = TempDir::new().unwrap();
    let output = dir.path().join("large.txt");

    // Generate 1MB of data
    let data = "x".repeat(1024 * 1024);

    let mut cmd = Command::cargo_bin("mutx").unwrap();
    cmd.arg("--stream")
        .arg(&output)
        .write_stdin(data.clone())
        .assert()
        .success();

    assert_eq!(fs::read_to_string(&output).unwrap(), data);
}

#[test]
fn test_housekeep_full_workflow() {
    let dir = TempDir::new().unwrap();

    // Create various files
    fs::write(dir.path().join("file1.txt"), "data").unwrap();
    fs::write(dir.path().join("file1.txt.backup"), "old").unwrap();
    fs::write(dir.path().join("file2.lock"), "").unwrap();

    // Dry run first
    let mut cmd = Command::cargo_bin("mutx").unwrap();
    cmd.arg("housekeep")
        .arg("--all")
        .arg("--dry-run")
        .arg(dir.path())
        .assert()
        .success()
        .stdout(predicate::str::contains("file2.lock"));

    // Verify nothing deleted
    assert!(dir.path().join("file2.lock").exists());

    // Real cleanup
    let mut cmd = Command::cargo_bin("mutx").unwrap();
    cmd.arg("housekeep")
        .arg("--all")
        .arg(dir.path())
        .assert()
        .success();

    // Verify lock cleaned
    assert!(!dir.path().join("file2.lock").exists());
}
