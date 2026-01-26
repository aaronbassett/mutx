use assert_cmd::Command;
use predicates::prelude::*;
use std::fs;
use tempfile::TempDir;

/// Helper function to create a test directory with backup files
fn setup_test_dir_with_backups() -> TempDir {
    let temp_dir = TempDir::new().unwrap();
    let test_file = temp_dir.path().join("file.txt.mutx.backup");
    fs::write(&test_file, "backup content").unwrap();
    temp_dir
}

/// Helper function to create a test directory with lock files
fn setup_test_dir_with_locks() -> TempDir {
    let temp_dir = TempDir::new().unwrap();
    let lock_file = temp_dir.path().join("file.txt.mutx.lock");
    fs::write(&lock_file, "lock content").unwrap();
    temp_dir
}

#[test]
fn test_dry_run_shows_would_clean() {
    let temp_dir = setup_test_dir_with_backups();

    let mut cmd = Command::cargo_bin("mutx").unwrap();
    cmd.arg("housekeep")
        .arg("backups")
        .arg("--dry-run")
        .arg("--older-than")
        .arg("0s")
        .arg(temp_dir.path());

    cmd.assert()
        .success()
        .stdout(predicate::str::contains("Would clean 1 backup file(s)"));
}

#[test]
fn test_normal_run_shows_cleaned() {
    let temp_dir = setup_test_dir_with_backups();

    let mut cmd = Command::cargo_bin("mutx").unwrap();
    cmd.arg("housekeep")
        .arg("backups")
        .arg("--older-than")
        .arg("0s")
        .arg(temp_dir.path());

    cmd.assert()
        .success()
        .stdout(predicate::str::contains("Cleaned 1 backup file(s)"));
}

#[test]
fn test_dry_run_verbose_shows_would_clean() {
    let temp_dir = setup_test_dir_with_locks();

    let mut cmd = Command::cargo_bin("mutx").unwrap();
    cmd.arg("housekeep")
        .arg("locks")
        .arg("--dry-run")
        .arg("--verbose")
        .arg("--older-than")
        .arg("0s")
        .arg(temp_dir.path());

    cmd.assert()
        .success()
        .stdout(predicate::str::contains("Would clean 1 lock file(s)"));
}

#[test]
fn test_all_command_dry_run_shows_would_clean() {
    let temp_dir = TempDir::new().unwrap();

    // Create both backup and lock files
    let backup_file = temp_dir.path().join("file.txt.mutx.backup");
    fs::write(&backup_file, "backup content").unwrap();
    let lock_file = temp_dir.path().join("file.txt.mutx.lock");
    fs::write(&lock_file, "lock content").unwrap();

    let mut cmd = Command::cargo_bin("mutx").unwrap();
    cmd.arg("housekeep")
        .arg("all")
        .arg("--dry-run")
        .arg("--older-than")
        .arg("0s")
        .arg(temp_dir.path());

    cmd.assert()
        .success()
        .stdout(predicate::str::contains("Would clean 1 lock file(s)"))
        .stdout(predicate::str::contains("Would clean 1 backup file(s)"));
}
