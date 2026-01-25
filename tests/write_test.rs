use mutx::{AtomicWriter, WriteMode};
use std::fs;
use std::io::Write;
use tempfile::TempDir;

#[test]
fn test_simple_write_creates_file() {
    let dir = TempDir::new().unwrap();
    let target = dir.path().join("test.txt");

    let mut writer = AtomicWriter::new(&target, WriteMode::Simple).unwrap();
    writer.write_all(b"hello world").unwrap();
    writer.commit().unwrap();

    let content = fs::read_to_string(&target).unwrap();
    assert_eq!(content, "hello world");
}

#[test]
fn test_simple_write_atomic_on_error() {
    let dir = TempDir::new().unwrap();
    let target = dir.path().join("test.txt");

    // Write initial content
    fs::write(&target, "original").unwrap();

    // Start write but don't commit
    {
        let mut writer = AtomicWriter::new(&target, WriteMode::Simple).unwrap();
        writer.write_all(b"new content").unwrap();
        // Drop without commit
    }

    // Original content should be preserved
    let content = fs::read_to_string(&target).unwrap();
    assert_eq!(content, "original");
}

#[test]
fn test_empty_input_creates_empty_file() {
    let dir = TempDir::new().unwrap();
    let target = dir.path().join("empty.txt");

    let writer = AtomicWriter::new(&target, WriteMode::Simple).unwrap();
    writer.commit().unwrap();

    assert!(target.exists());
    assert_eq!(fs::read_to_string(&target).unwrap(), "");
}
