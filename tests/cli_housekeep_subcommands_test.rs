use assert_cmd::Command;
use std::fs;
use tempfile::TempDir;

#[test]
fn test_housekeep_locks_subcommand() {
    let mut cmd = Command::cargo_bin("mutx").unwrap();
    cmd.arg("housekeep")
        .arg("locks")
        .arg("--dry-run")
        .assert()
        .success();
}

#[test]
fn test_housekeep_backups_subcommand() {
    let dir = TempDir::new().unwrap();

    let mut cmd = Command::cargo_bin("mutx").unwrap();
    cmd.arg("housekeep")
        .arg("backups")
        .arg("--dry-run")
        .arg(dir.path())
        .assert()
        .success();
}

#[test]
fn test_housekeep_all_with_single_dir() {
    let dir = TempDir::new().unwrap();

    let mut cmd = Command::cargo_bin("mutx").unwrap();
    cmd.arg("housekeep")
        .arg("all")
        .arg("--dry-run")
        .arg(dir.path())
        .assert()
        .success();
}

#[test]
fn test_housekeep_all_with_separate_dirs() {
    let locks_dir = TempDir::new().unwrap();
    let backups_dir = TempDir::new().unwrap();

    let mut cmd = Command::cargo_bin("mutx").unwrap();
    cmd.arg("housekeep")
        .arg("all")
        .arg("--dry-run")
        .arg("--locks-dir")
        .arg(locks_dir.path())
        .arg("--backups-dir")
        .arg(backups_dir.path())
        .assert()
        .success();
}

#[test]
fn test_housekeep_all_requires_dir_or_both_flags() {
    let mut cmd = Command::cargo_bin("mutx").unwrap();
    cmd.arg("housekeep")
        .arg("all")
        .arg("--dry-run")
        .assert()
        .failure();
}

#[test]
fn test_housekeep_backups_custom_suffix() {
    let dir = TempDir::new().unwrap();

    // Create a .bak file
    fs::write(dir.path().join("test.txt.bak"), "backup").unwrap();

    let mut cmd = Command::cargo_bin("mutx").unwrap();
    cmd.arg("housekeep")
        .arg("backups")
        .arg("--suffix")
        .arg(".bak")
        .arg("--dry-run")
        .arg(dir.path())
        .assert()
        .success();
}
