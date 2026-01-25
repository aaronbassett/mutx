use assert_cmd::Command;
use predicates::prelude::*;

#[test]
fn test_help_message_shows() {
    let mut cmd = assert_cmd::cargo::cargo_bin!("mutx");
    cmd.arg("--help")
        .assert()
        .success()
        .stdout(predicate::str::contains("Atomic file writes"));
}

#[test]
fn test_requires_output_file() {
    let mut cmd = assert_cmd::cargo::cargo_bin!("mutx");
    cmd.assert()
        .failure()
        .stderr(predicate::str::contains("Output file required"));
}

#[test]
fn test_version_flag() {
    let mut cmd = assert_cmd::cargo::cargo_bin!("mutx");
    cmd.arg("--version")
        .assert()
        .success()
        .stdout(predicate::str::contains("1.0.0"));
}
