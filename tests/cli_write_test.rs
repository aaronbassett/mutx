use assert_cmd::Command;
use predicates::prelude::*;
use std::fs;
use tempfile::TempDir;

#[test]
fn test_write_from_stdin() {
    let dir = TempDir::new().unwrap();
    let output = dir.path().join("output.txt");

    let mut cmd = Command::cargo_bin("mutx").unwrap();
    cmd.arg(output.to_str().unwrap())
        .write_stdin("hello world")
        .assert()
        .success();

    assert_eq!(fs::read_to_string(&output).unwrap(), "hello world");
}

#[test]
fn test_write_from_file() {
    let dir = TempDir::new().unwrap();
    let input = dir.path().join("input.txt");
    let output = dir.path().join("output.txt");

    fs::write(&input, "file content").unwrap();

    let mut cmd = Command::cargo_bin("mutx").unwrap();
    cmd.arg("--input").arg(input.to_str().unwrap())
        .arg(output.to_str().unwrap())
        .assert()
        .success();

    assert_eq!(fs::read_to_string(&output).unwrap(), "file content");
}

#[test]
fn test_streaming_mode() {
    let dir = TempDir::new().unwrap();
    let output = dir.path().join("output.txt");

    let mut cmd = Command::cargo_bin("mutx").unwrap();
    cmd.arg("--stream")
        .arg(output.to_str().unwrap())
        .write_stdin("streamed content")
        .assert()
        .success();

    assert_eq!(fs::read_to_string(&output).unwrap(), "streamed content");
}

#[test]
fn test_empty_input_creates_empty_file() {
    let dir = TempDir::new().unwrap();
    let output = dir.path().join("empty.txt");

    let mut cmd = Command::cargo_bin("mutx").unwrap();
    cmd.arg(output.to_str().unwrap())
        .write_stdin("")
        .assert()
        .success();

    assert!(output.exists());
    assert_eq!(fs::read_to_string(&output).unwrap(), "");
}

#[test]
fn test_backup_creation() {
    let dir = TempDir::new().unwrap();
    let output = dir.path().join("file.txt");

    fs::write(&output, "original").unwrap();

    let mut cmd = Command::cargo_bin("mutx").unwrap();
    cmd.arg("--backup")
        .arg(output.to_str().unwrap())
        .write_stdin("updated")
        .assert()
        .success();

    let backup = output.with_extension("txt.backup");
    assert!(backup.exists());
    assert_eq!(fs::read_to_string(&backup).unwrap(), "original");
    assert_eq!(fs::read_to_string(&output).unwrap(), "updated");
}

#[test]
fn test_lock_no_wait_fails_when_locked() {
    use std::sync::Arc;
    use std::thread;
    use std::time::Duration;

    let dir = TempDir::new().unwrap();
    let output = Arc::new(dir.path().join("locked.txt"));
    let lock_path = output.with_extension("lock");

    let output_clone = output.clone();
    let handle = thread::spawn(move || {
        let _lock = mutx::FileLock::acquire(
            &lock_path,
            mutx::LockStrategy::Wait
        ).unwrap();
        thread::sleep(Duration::from_secs(2));
    });

    thread::sleep(Duration::from_millis(100));

    let mut cmd = Command::cargo_bin("mutx").unwrap();
    cmd.arg("--no-wait")
        .arg(output.to_str().unwrap())
        .write_stdin("should fail")
        .assert()
        .failure()
        .code(2)
        .stderr(predicate::str::contains("locked"));

    handle.join().unwrap();
}
