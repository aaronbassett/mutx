use assert_cmd::Command;
use tempfile::TempDir;

#[test]
fn test_implicit_write_command() {
    let dir = TempDir::new().unwrap();
    let output = dir.path().join("test.txt");

    let mut cmd = Command::cargo_bin("mutx").unwrap();
    cmd.arg(output.to_str().unwrap())
        .write_stdin("test content")
        .assert()
        .success();

    let content = std::fs::read_to_string(&output).unwrap();
    assert_eq!(content, "test content");
}

#[test]
fn test_explicit_write_command() {
    let dir = TempDir::new().unwrap();
    let output = dir.path().join("test.txt");

    let mut cmd = Command::cargo_bin("mutx").unwrap();
    cmd.arg("write")
        .arg(output.to_str().unwrap())
        .write_stdin("test content")
        .assert()
        .success();

    let content = std::fs::read_to_string(&output).unwrap();
    assert_eq!(content, "test content");
}

#[test]
fn test_both_forms_produce_identical_results() {
    let dir = TempDir::new().unwrap();

    // Implicit form
    let output1 = dir.path().join("test1.txt");
    Command::cargo_bin("mutx").unwrap()
        .arg(output1.to_str().unwrap())
        .write_stdin("content")
        .assert()
        .success();

    // Explicit form
    let output2 = dir.path().join("test2.txt");
    Command::cargo_bin("mutx").unwrap()
        .arg("write")
        .arg(output2.to_str().unwrap())
        .write_stdin("content")
        .assert()
        .success();

    let content1 = std::fs::read_to_string(&output1).unwrap();
    let content2 = std::fs::read_to_string(&output2).unwrap();
    assert_eq!(content1, content2);
}
