use assert_cmd::Command;
use predicates::prelude::*;
use std::fs::{self, File};
use tempfile::TempDir;

#[test]
fn test_housekeep_clean_locks() {
    let dir = TempDir::new().unwrap();

    // Create orphaned lock
    let lock = dir.path().join("file.lock");
    File::create(&lock).unwrap();

    let mut cmd = Command::cargo_bin("mutx").unwrap();
    cmd.arg("housekeep")
        .arg("--clean-locks")
        .arg(dir.path())
        .assert()
        .success()
        .stdout(predicate::str::contains("file.lock"));

    assert!(!lock.exists());
}

#[test]
fn test_housekeep_dry_run() {
    let dir = TempDir::new().unwrap();
    let lock = dir.path().join("file.lock");
    File::create(&lock).unwrap();

    let mut cmd = Command::cargo_bin("mutx").unwrap();
    cmd.arg("housekeep")
        .arg("--clean-locks")
        .arg("--dry-run")
        .arg(dir.path())
        .assert()
        .success()
        .stdout(predicate::str::contains("file.lock"));

    assert!(lock.exists(), "Dry run should not delete");
}

#[test]
fn test_housekeep_clean_backups() {
    let dir = TempDir::new().unwrap();

    // Create backup files
    fs::write(dir.path().join("file.txt.backup"), "backup1").unwrap();
    fs::write(dir.path().join("file.txt.20260125-120000.backup"), "backup2").unwrap();

    let mut cmd = Command::cargo_bin("mutx").unwrap();
    cmd.arg("housekeep")
        .arg("--clean-backups")
        .arg("--keep-newest")
        .arg("1")
        .arg(dir.path())
        .assert()
        .success();

    // Should keep one backup
    let backups: Vec<_> = fs::read_dir(dir.path())
        .unwrap()
        .filter_map(|e| e.ok())
        .filter(|e| e.path().to_str().unwrap().contains("backup"))
        .collect();

    assert_eq!(backups.len(), 1);
}

#[test]
fn test_housekeep_requires_operation() {
    let dir = TempDir::new().unwrap();

    let mut cmd = Command::cargo_bin("mutx").unwrap();
    cmd.arg("housekeep")
        .arg(dir.path())
        .assert()
        .failure()
        .stderr(predicate::str::contains("at least one operation"));
}
