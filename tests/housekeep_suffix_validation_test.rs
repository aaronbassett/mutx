use assert_cmd::Command;
use predicates::prelude::*;
use std::fs;
use tempfile::TempDir;

#[test]
fn test_empty_suffix_rejected() {
    let dir = TempDir::new().unwrap();
    fs::write(dir.path().join("file.txt.bak"), "backup").unwrap();

    let mut cmd = Command::new(env!("CARGO_BIN_EXE_mutx"));
    cmd.arg("housekeep")
        .arg("backups")
        .arg("--suffix")
        .arg("")
        .arg(dir.path())
        .assert()
        .failure()
        .stderr(predicate::str::contains("Backup suffix cannot be empty"));
}

#[test]
fn test_single_dot_suffix_rejected() {
    let dir = TempDir::new().unwrap();
    fs::write(dir.path().join("file.txt.bak"), "backup").unwrap();

    let mut cmd = Command::new(env!("CARGO_BIN_EXE_mutx"));
    cmd.arg("housekeep")
        .arg("backups")
        .arg("--suffix")
        .arg(".")
        .arg(dir.path())
        .assert()
        .failure()
        .stderr(predicate::str::contains(
            "Backup suffix cannot be a single dot",
        ));
}

#[test]
fn test_valid_suffixes_accepted() {
    let dir = TempDir::new().unwrap();

    // Test .bak suffix
    fs::write(dir.path().join("file1.txt.bak"), "backup1").unwrap();
    let mut cmd = Command::new(env!("CARGO_BIN_EXE_mutx"));
    cmd.arg("housekeep")
        .arg("backups")
        .arg("--suffix")
        .arg(".bak")
        .arg(dir.path())
        .assert()
        .success();

    // Test backup suffix
    fs::write(dir.path().join("file2.txt.backup"), "backup2").unwrap();
    let mut cmd = Command::new(env!("CARGO_BIN_EXE_mutx"));
    cmd.arg("housekeep")
        .arg("backups")
        .arg("--suffix")
        .arg(".backup")
        .arg(dir.path())
        .assert()
        .success();

    // Test .old.bak suffix
    fs::write(dir.path().join("file3.txt.old.bak"), "backup3").unwrap();
    let mut cmd = Command::new(env!("CARGO_BIN_EXE_mutx"));
    cmd.arg("housekeep")
        .arg("backups")
        .arg("--suffix")
        .arg(".old.bak")
        .arg(dir.path())
        .assert()
        .success();
}

#[test]
fn test_empty_suffix_rejected_in_all_command() {
    let dir = TempDir::new().unwrap();

    let mut cmd = Command::new(env!("CARGO_BIN_EXE_mutx"));
    cmd.arg("housekeep")
        .arg("all")
        .arg("--suffix")
        .arg("")
        .arg(dir.path())
        .assert()
        .failure()
        .stderr(predicate::str::contains("Backup suffix cannot be empty"));
}
