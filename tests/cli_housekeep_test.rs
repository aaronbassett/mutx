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
        .arg("locks")
        .arg("--verbose")
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
        .arg("locks")
        .arg("--dry-run")
        .arg("--verbose")
        .arg(dir.path())
        .assert()
        .success()
        .stdout(predicate::str::contains("file.lock"));

    assert!(lock.exists(), "Dry run should not delete");
}

#[test]
fn test_housekeep_clean_backups() {
    let dir = TempDir::new().unwrap();

    // Create backup files in the new format
    fs::write(dir.path().join("file.txt.mutx.backup"), "backup1").unwrap();
    fs::write(
        dir.path().join("file.txt.20260125_120000.mutx.backup"),
        "backup2",
    )
    .unwrap();

    let mut cmd = Command::cargo_bin("mutx").unwrap();
    cmd.arg("housekeep")
        .arg("backups")
        .arg("--keep-newest")
        .arg("1")
        .arg(dir.path())
        .assert()
        .success();

    // Should keep one backup (the newest one)
    let backups: Vec<_> = fs::read_dir(dir.path())
        .unwrap()
        .filter_map(|e| e.ok())
        .filter(|e| e.path().to_str().unwrap().contains(".mutx.backup"))
        .collect();

    assert_eq!(backups.len(), 1);
}

#[test]
fn test_housekeep_requires_subcommand() {
    let mut cmd = Command::cargo_bin("mutx").unwrap();
    cmd.arg("housekeep")
        .assert()
        .failure()
        .stderr(predicate::str::contains("Usage: mutx housekeep <COMMAND>"));
}
