use assert_cmd::Command;
use predicates::prelude::*;
use std::fs;
use tempfile::TempDir;

#[test]
fn test_output_must_be_provided() {
    let mut cmd = Command::cargo_bin("mutx").unwrap();
    cmd.write_stdin("test");

    cmd.assert()
        .failure()
        .stderr(
            predicate::str::contains("Output file required")
                .or(predicate::str::contains("OUTPUT")),
        );
}

#[test]
fn test_input_file_must_exist() {
    let temp = TempDir::new().unwrap();
    let output = temp.path().join("output.txt");
    let input = temp.path().join("nonexistent.txt");

    let mut cmd = Command::cargo_bin("mutx").unwrap();
    cmd.arg("--input")
        .arg(&input)
        .arg(&output);

    cmd.assert()
        .failure()
        .stderr(
            predicate::str::contains("does not exist")
                .or(predicate::str::contains("not found"))
                .or(predicate::str::contains("PathNotFound")),
        );
}

#[test]
fn test_backup_dir_must_be_directory() {
    let temp = TempDir::new().unwrap();
    let output = temp.path().join("output.txt");
    let not_a_dir = temp.path().join("file.txt");
    fs::write(&not_a_dir, "").unwrap();

    let mut cmd = Command::cargo_bin("mutx").unwrap();
    cmd.arg("--backup")
        .arg("--backup-dir")
        .arg(&not_a_dir)
        .arg(&output)
        .write_stdin("test");

    cmd.assert()
        .failure()
        .stderr(
            predicate::str::contains("not a directory")
                .or(predicate::str::contains("NotADirectory")),
        );
}
