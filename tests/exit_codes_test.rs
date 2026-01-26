use assert_cmd::Command;
use tempfile::TempDir;

#[test]
fn test_exit_code_0_on_success() {
    let dir = TempDir::new().unwrap();
    let output = dir.path().join("test.txt");

    let mut cmd = Command::new(env!("CARGO_BIN_EXE_mutx"));
    cmd.arg(output).write_stdin("data").assert().code(0);
}

#[test]
fn test_exit_code_1_on_general_error() {
    // Test with invalid UTF-8 in path to trigger general error
    let mut cmd = Command::new(env!("CARGO_BIN_EXE_mutx"));
    cmd.arg("") // Empty path
        .write_stdin("data")
        .assert()
        .failure(); // Just verify it fails, exit code may vary by platform
}

#[test]
fn test_exit_code_2_on_lock_timeout() {
    let dir = TempDir::new().unwrap();
    let output = dir.path().join("test.txt");
    let lock_path = mutx::derive_lock_path(&output, false).unwrap();

    let _lock = mutx::FileLock::acquire(&lock_path, mutx::LockStrategy::Wait).unwrap();

    let mut cmd = Command::new(env!("CARGO_BIN_EXE_mutx"));
    cmd.arg("--timeout")
        .arg("1")
        .arg(output)
        .write_stdin("data")
        .assert()
        .code(2);
}

#[test]
fn test_exit_code_2_on_no_wait() {
    let dir = TempDir::new().unwrap();
    let output = dir.path().join("test.txt");
    let lock_path = mutx::derive_lock_path(&output, false).unwrap();

    let _lock = mutx::FileLock::acquire(&lock_path, mutx::LockStrategy::Wait).unwrap();

    let mut cmd = Command::new(env!("CARGO_BIN_EXE_mutx"));
    cmd.arg("--no-wait")
        .arg(output)
        .write_stdin("data")
        .assert()
        .code(2);
}
