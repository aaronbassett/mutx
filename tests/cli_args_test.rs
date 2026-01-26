use assert_cmd::Command;
use predicates::prelude::*;

#[test]
fn test_help_message_shows() {
    let mut cmd = Command::new(env!("CARGO_BIN_EXE_mutx"));
    cmd.arg("--help")
        .assert()
        .success()
        .stdout(predicate::str::contains("Atomic file writes"));
}

#[test]
fn test_requires_output_file() {
    let mut cmd = Command::new(env!("CARGO_BIN_EXE_mutx"));
    cmd.assert()
        .failure()
        .stderr(predicate::str::contains("OUTPUT argument required"));
}

#[test]
fn test_version_flag() {
    let mut cmd = Command::new(env!("CARGO_BIN_EXE_mutx"));
    cmd.arg("--version")
        .assert()
        .success()
        .stdout(predicate::str::contains("0.3.0"));
}
