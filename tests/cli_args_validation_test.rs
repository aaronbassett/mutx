use assert_cmd::Command;

#[test]
fn test_timeout_with_no_wait_conflicts() {
    let temp = tempfile::TempDir::new().unwrap();
    let output = temp.path().join("output.txt");

    let mut cmd = Command::new(env!("CARGO_BIN_EXE_mutx"));
    cmd.arg("--no-wait")
        .arg("--timeout")
        .arg("5")
        .arg(&output)
        .write_stdin("test");

    cmd.assert()
        .failure()
        .stderr(predicates::str::contains("cannot be used with"));
}

#[test]
fn test_timeout_alone_works() {
    // This test verifies timeout implies wait mode
    let temp = tempfile::TempDir::new().unwrap();
    let output = temp.path().join("output.txt");

    let mut cmd = Command::new(env!("CARGO_BIN_EXE_mutx"));
    cmd.arg("--timeout")
        .arg("1")
        .arg(&output)
        .write_stdin("test content");

    cmd.assert().success();
}
