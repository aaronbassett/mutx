use mutx::lock::derive_lock_path;
use tempfile::TempDir;

#[test]
fn test_lock_path_format_basic() {
    let temp = TempDir::new().unwrap();
    let output = temp.path().join("data").join("files").join("output.txt");

    // Create parent directory so it exists for canonicalization
    std::fs::create_dir_all(output.parent().unwrap()).unwrap();

    let lock_path = derive_lock_path(&output, false).unwrap();

    // Should contain parent "files", filename "output.txt", and 8-char hash
    // Format: {initialism}files.output.txt.{hash}.lock
    // where initialism includes up to 3 ancestor directories (excluding parent)
    let name = lock_path.file_name().unwrap().to_str().unwrap();

    // Check it ends with expected pattern
    assert!(name.contains("files.output.txt."));
    assert!(name.ends_with(".lock"));

    // Extract hash part (should be 8 hex chars before .lock)
    let without_lock = name.strip_suffix(".lock").unwrap();
    let parts: Vec<&str> = without_lock.split('.').collect();

    // Should have at least: {parent}.{filename_parts...}.{hash}
    // Example: d.files.output.txt.{hash} or t.d.files.output.txt.{hash}
    assert!(parts.len() >= 5); // At least: initials..., files, output, txt, hash

    // Hash is the last part before .lock
    let hash = parts[parts.len() - 1];
    assert_eq!(hash.len(), 8);
    assert!(hash.chars().all(|c| c.is_ascii_hexdigit()));
}

#[test]
fn test_lock_path_same_for_same_file() {
    let temp = TempDir::new().unwrap();
    let output = temp.path().join("test.txt");

    let path1 = derive_lock_path(&output, false).unwrap();
    let path2 = derive_lock_path(&output, false).unwrap();

    assert_eq!(path1, path2);
}

#[test]
fn test_lock_path_different_for_different_files() {
    let temp = TempDir::new().unwrap();
    let output1 = temp.path().join("test1.txt");
    let output2 = temp.path().join("test2.txt");

    let path1 = derive_lock_path(&output1, false).unwrap();
    let path2 = derive_lock_path(&output2, false).unwrap();

    assert_ne!(path1, path2);
}

#[test]
fn test_lock_path_in_cache_directory() {
    let temp = TempDir::new().unwrap();
    let output = temp.path().join("test.txt");

    let lock_path = derive_lock_path(&output, false).unwrap();

    // Should be in platform cache directory
    let path_str = lock_path.to_str().unwrap();

    #[cfg(target_os = "linux")]
    assert!(path_str.contains("/.cache/mutx/locks/"));

    #[cfg(target_os = "macos")]
    assert!(path_str.contains("/Library/Caches/mutx/locks/"));

    #[cfg(target_os = "windows")]
    assert!(path_str.contains("\\Local\\mutx\\locks\\"));
}

#[test]
fn test_custom_lock_path_accepted() {
    let temp = TempDir::new().unwrap();
    let _output = temp.path().join("test.txt");
    let custom = temp.path().join("custom.lock");

    let lock_path = derive_lock_path(&custom, true).unwrap();

    // Custom path should be used as-is
    assert_eq!(lock_path, custom);
}
