use assert_cmd::Command;

#[test]
fn test_timeout_without_wait_should_fail() {
    let mut cmd = Command::cargo_bin("mutx").unwrap();
    cmd.arg("--timeout").arg("5")
        .arg("output.txt")
        .write_stdin("test");

    cmd.assert()
        .failure()
        .stderr(predicates::str::contains("timeout"));
}

#[test]
fn test_timeout_with_wait_should_work() {
    // This test verifies the fix works
    let temp = tempfile::TempDir::new().unwrap();
    let output = temp.path().join("output.txt");

    let mut cmd = Command::cargo_bin("mutx").unwrap();
    cmd.arg("--wait")
        .arg("--timeout").arg("1")
        .arg(&output)
        .write_stdin("test content");

    cmd.assert().success();
}
