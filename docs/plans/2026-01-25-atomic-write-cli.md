# Atomic-Write CLI Implementation Plan

> **PROJECT NOTE:** This project is named `mutx` (not `atomic-write`). Update all references to package name, binary name, and repository accordingly.

> **For Claude:** REQUIRED SUB-SKILL: Use superpowers:executing-plans to implement this plan task-by-task.

**Goal:** Build a production-ready CLI tool for atomic file writes with file locking, backup functionality, and housekeeping utilities.

**Architecture:** Core library handles atomic writes using `atomic-write-file` crate with `fs2` for advisory file locking. CLI layer built with `clap` provides two subcommands: main write operation and housekeep utilities. TDD approach with unit tests for library code and integration tests for CLI behavior.

**Tech Stack:** Rust 1.70+, clap 4.x, atomic-write-file 0.2+, fs2, anyhow, tempfile (dev)

---

## Phase 1: Project Setup

### Task 1.1: Initialize Rust Project

**Files:**
- Create: `Cargo.toml`
- Create: `src/main.rs`
- Create: `src/lib.rs`
- Create: `.gitignore`

**Step 1: Initialize cargo project**

```bash
cargo init --name atomic-write
```

**Step 2: Update Cargo.toml with dependencies**

Edit `Cargo.toml`:

```toml
[package]
name = "atomic-write"
version = "1.0.0"
edition = "2021"
rust-version = "1.70"
authors = ["Aaron Bassett"]
description = "Atomic file writes with process coordination through file locking"
license = "MIT OR Apache-2.0"
repository = "https://github.com/aaronbassett/atomic-write"
keywords = ["atomic", "file", "lock", "cli"]
categories = ["command-line-utilities", "filesystem"]

[[bin]]
name = "atomic-write"
path = "src/main.rs"

[dependencies]
clap = { version = "4.5", features = ["derive", "wrap_help"] }
atomic-write-file = "0.2"
fs2 = "0.4"
anyhow = "1.0"
libc = "0.2"

[dev-dependencies]
tempfile = "3.8"
assert_cmd = "2.0"
predicates = "3.0"
```

**Step 3: Create basic src/lib.rs**

```rust
//! Atomic file write library with file locking support

pub mod lock;
pub mod write;
pub mod backup;
pub mod housekeep;

pub use write::{AtomicWriter, WriteMode};
pub use lock::{FileLock, LockStrategy};
```

**Step 4: Create basic src/main.rs**

```rust
use clap::Parser;
use anyhow::Result;

mod cli;

fn main() -> Result<()> {
    let args = cli::Args::parse();
    cli::run(args)
}
```

**Step 5: Create .gitignore**

```
/target/
Cargo.lock
*.swp
*.swo
*~
.DS_Store
```

**Step 6: Verify build**

```bash
cargo check
```

Expected: Success (warnings ok, should compile)

**Step 7: Commit**

```bash
git add Cargo.toml src/ .gitignore
git commit -m "feat: initialize atomic-write project structure"
```

---

### Task 1.2: Setup CLI Argument Parsing

**Files:**
- Create: `src/cli/mod.rs`
- Create: `src/cli/args.rs`
- Modify: `src/main.rs`

**Step 1: Write test for CLI argument parsing**

Create `tests/cli_args_test.rs`:

```rust
use assert_cmd::Command;
use predicates::prelude::*;

#[test]
fn test_help_message_shows() {
    let mut cmd = Command::cargo_bin("atomic-write").unwrap();
    cmd.arg("--help")
        .assert()
        .success()
        .stdout(predicate::str::contains("Atomic file writes"));
}

#[test]
fn test_requires_output_file() {
    let mut cmd = Command::cargo_bin("atomic-write").unwrap();
    cmd.assert()
        .failure()
        .stderr(predicate::str::contains("required arguments"));
}

#[test]
fn test_version_flag() {
    let mut cmd = Command::cargo_bin("atomic-write").unwrap();
    cmd.arg("--version")
        .assert()
        .success()
        .stdout(predicate::str::contains("1.0.0"));
}
```

**Step 2: Run test to verify it fails**

```bash
cargo test test_help_message_shows
```

Expected: FAIL (cli module doesn't exist)

**Step 3: Create CLI argument structure**

Create `src/cli/args.rs`:

```rust
use clap::{Parser, Subcommand};
use std::path::PathBuf;

#[derive(Parser, Debug)]
#[command(
    name = "atomic-write",
    version,
    about = "Atomic file writes with process coordination through file locking",
    long_about = None
)]
pub struct Args {
    #[command(subcommand)]
    pub command: Option<Command>,

    /// Target file path (required if no subcommand)
    #[arg(value_name = "OUTPUT", required_unless_present = "command")]
    pub output: Option<PathBuf>,

    /// Read from file instead of stdin
    #[arg(short, long, value_name = "FILE")]
    pub input: Option<PathBuf>,

    /// Use streaming mode (constant memory)
    #[arg(long)]
    pub stream: bool,

    /// Wait for lock (default)
    #[arg(long, conflicts_with = "no_wait")]
    pub wait: bool,

    /// Fail immediately if locked
    #[arg(long, conflicts_with = "wait")]
    pub no_wait: bool,

    /// Wait timeout in seconds (requires --wait)
    #[arg(short = 't', long, value_name = "SECONDS", requires = "wait")]
    pub timeout: Option<u64>,

    /// Custom lock file location
    #[arg(long, value_name = "PATH")]
    pub lock_file: Option<PathBuf>,

    /// Create backup before overwrite
    #[arg(short = 'b', long)]
    pub backup: bool,

    /// Backup filename suffix
    #[arg(long, value_name = "SUFFIX", default_value = ".backup", requires = "backup")]
    pub backup_suffix: String,

    /// Store backups in directory
    #[arg(long, value_name = "DIR", requires = "backup")]
    pub backup_dir: Option<PathBuf>,

    /// Add timestamp to backup filename
    #[arg(long, requires = "backup")]
    pub backup_timestamp: bool,

    /// Set file permissions (octal, e.g., 0644)
    #[arg(short = 'm', long, value_name = "OCTAL")]
    pub mode: Option<String>,

    /// Use umask default permissions instead of preserving
    #[arg(long)]
    pub no_preserve_mode: bool,

    /// Preserve owner/group (requires privileges)
    #[arg(long)]
    pub preserve_owner: bool,

    /// Preserve owner, ignore EPERM errors
    #[arg(long, conflicts_with = "preserve_owner")]
    pub try_preserve_owner: bool,

    /// Verbose output
    #[arg(short = 'v', action = clap::ArgAction::Count)]
    pub verbose: u8,

    /// Suppress non-error output
    #[arg(short = 'q', long, conflicts_with = "verbose")]
    pub quiet: bool,

    /// Structured JSON output
    #[arg(long)]
    pub json: bool,
}

#[derive(Subcommand, Debug)]
pub enum Command {
    /// Housekeeping operations
    Housekeep {
        /// Directory to clean (default: current directory)
        #[arg(value_name = "DIR")]
        dir: Option<PathBuf>,

        /// Clean orphaned lock files
        #[arg(long)]
        clean_locks: bool,

        /// Clean old backup files
        #[arg(long)]
        clean_backups: bool,

        /// Clean both locks and backups
        #[arg(long)]
        all: bool,

        /// Scan subdirectories
        #[arg(short = 'r', long)]
        recursive: bool,

        /// Age threshold (e.g., "2h" for locks, "7d" for backups)
        #[arg(long, value_name = "DURATION")]
        older_than: Option<String>,

        /// Keep N newest backups per file (backups only)
        #[arg(long, value_name = "N")]
        keep_newest: Option<usize>,

        /// Show what would be deleted without deleting
        #[arg(short = 'n', long)]
        dry_run: bool,

        /// Verbose output
        #[arg(short = 'v', long)]
        verbose: bool,

        /// Structured JSON output
        #[arg(long)]
        json: bool,
    },
}
```

**Step 4: Create CLI module**

Create `src/cli/mod.rs`:

```rust
mod args;

pub use args::{Args, Command};
use anyhow::Result;

pub fn run(args: Args) -> Result<()> {
    // TODO: Implement
    println!("Args: {:?}", args);
    Ok(())
}
```

**Step 5: Update main.rs**

```rust
use clap::Parser;
use anyhow::Result;

mod cli;

fn main() -> Result<()> {
    let args = cli::Args::parse();
    cli::run(args)
}
```

**Step 6: Run tests**

```bash
cargo test
```

Expected: All CLI argument tests PASS

**Step 7: Commit**

```bash
git add src/cli/ tests/ src/main.rs
git commit -m "feat: add CLI argument parsing with clap"
```

---

## Phase 2: Core Library - File Locking

### Task 2.1: File Lock Module - Basic Structure

**Files:**
- Create: `src/lock.rs`
- Create: `tests/lock_test.rs`

**Step 1: Write test for lock acquisition**

Create `tests/lock_test.rs`:

```rust
use atomic_write::lock::{FileLock, LockStrategy};
use tempfile::NamedTempFile;
use std::time::Duration;

#[test]
fn test_lock_acquire_and_release() {
    let temp = NamedTempFile::new().unwrap();
    let lock_path = temp.path().with_extension("lock");

    let lock = FileLock::acquire(&lock_path, LockStrategy::Wait).unwrap();
    assert!(lock_path.exists());

    drop(lock);
    assert!(!lock_path.exists(), "Lock file should be cleaned up");
}

#[test]
fn test_lock_no_wait_fails_when_locked() {
    let temp = NamedTempFile::new().unwrap();
    let lock_path = temp.path().with_extension("lock");

    let _lock1 = FileLock::acquire(&lock_path, LockStrategy::Wait).unwrap();

    let result = FileLock::acquire(&lock_path, LockStrategy::NoWait);
    assert!(result.is_err());
    assert!(result.unwrap_err().to_string().contains("locked"));
}

#[test]
fn test_lock_timeout() {
    let temp = NamedTempFile::new().unwrap();
    let lock_path = temp.path().with_extension("lock");

    let _lock1 = FileLock::acquire(&lock_path, LockStrategy::Wait).unwrap();

    let start = std::time::Instant::now();
    let result = FileLock::acquire(
        &lock_path,
        LockStrategy::Timeout(Duration::from_secs(1))
    );
    let elapsed = start.elapsed();

    assert!(result.is_err());
    assert!(elapsed >= Duration::from_secs(1));
    assert!(elapsed < Duration::from_secs(2));
}
```

**Step 2: Run test to verify it fails**

```bash
cargo test test_lock_acquire_and_release
```

Expected: FAIL (lock module doesn't exist)

**Step 3: Implement FileLock structure**

Create `src/lock.rs`:

```rust
use anyhow::{Context, Result, bail};
use std::fs::{File, OpenOptions};
use std::io;
use std::path::{Path, PathBuf};
use std::time::{Duration, Instant};
use fs2::FileExt;

#[derive(Debug, Clone)]
pub enum LockStrategy {
    Wait,
    NoWait,
    Timeout(Duration),
}

pub struct FileLock {
    file: File,
    path: PathBuf,
}

impl FileLock {
    /// Acquire an exclusive lock on the specified file
    pub fn acquire(lock_path: &Path, strategy: LockStrategy) -> Result<Self> {
        // Create lock file
        let file = OpenOptions::new()
            .create(true)
            .write(true)
            .open(lock_path)
            .with_context(|| format!("Failed to create lock file: {}", lock_path.display()))?;

        // Acquire lock based on strategy
        match strategy {
            LockStrategy::Wait => {
                file.lock_exclusive()
                    .with_context(|| format!("Failed to acquire lock: {}", lock_path.display()))?;
            }
            LockStrategy::NoWait => {
                file.try_lock_exclusive()
                    .map_err(|e| match e.kind() {
                        io::ErrorKind::WouldBlock => {
                            anyhow::anyhow!("File locked by another process")
                        }
                        _ => anyhow::Error::new(e)
                    })
                    .with_context(|| format!("Failed to acquire lock: {}", lock_path.display()))?;
            }
            LockStrategy::Timeout(duration) => {
                let start = Instant::now();
                loop {
                    match file.try_lock_exclusive() {
                        Ok(_) => break,
                        Err(e) if e.kind() == io::ErrorKind::WouldBlock => {
                            if start.elapsed() >= duration {
                                bail!(
                                    "Lock acquisition timeout after {}s",
                                    duration.as_secs()
                                );
                            }
                            std::thread::sleep(Duration::from_millis(100));
                        }
                        Err(e) => {
                            return Err(e).with_context(|| {
                                format!("Failed to acquire lock: {}", lock_path.display())
                            });
                        }
                    }
                }
            }
        }

        Ok(FileLock {
            file,
            path: lock_path.to_path_buf(),
        })
    }

    /// Get the lock file path
    pub fn path(&self) -> &Path {
        &self.path
    }
}

impl Drop for FileLock {
    fn drop(&mut self) {
        // Unlock is automatic when file handle is dropped
        // Try to delete lock file (best effort)
        let _ = std::fs::remove_file(&self.path);
    }
}
```

**Step 4: Update lib.rs exports**

Modify `src/lib.rs`:

```rust
//! Atomic file write library with file locking support

pub mod lock;

pub use lock::{FileLock, LockStrategy};
```

**Step 5: Run tests**

```bash
cargo test lock_test
```

Expected: All lock tests PASS

**Step 6: Commit**

```bash
git add src/lock.rs src/lib.rs tests/lock_test.rs
git commit -m "feat: implement file locking with Drop cleanup"
```

---

## Phase 3: Core Library - Atomic Write

### Task 3.1: Atomic Writer - Simple Mode

**Files:**
- Create: `src/write.rs`
- Create: `tests/write_test.rs`

**Step 1: Write test for simple mode write**

Create `tests/write_test.rs`:

```rust
use atomic_write::{AtomicWriter, WriteMode};
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
```

**Step 2: Run test to verify it fails**

```bash
cargo test test_simple_write_creates_file
```

Expected: FAIL (write module doesn't exist)

**Step 3: Implement AtomicWriter for simple mode**

Create `src/write.rs`:

```rust
use anyhow::{Context, Result};
use std::fs::{self, File};
use std::io::{self, Write};
use std::path::{Path, PathBuf};

#[derive(Debug, Clone, Copy)]
pub enum WriteMode {
    Simple,
    Streaming,
}

pub struct AtomicWriter {
    mode: WriteMode,
    target: PathBuf,
    buffer: Vec<u8>,
    temp_file: Option<atomic_write_file::AtomicWriteFile>,
}

impl AtomicWriter {
    /// Create a new atomic writer for the target file
    pub fn new(target: &Path, mode: WriteMode) -> Result<Self> {
        Ok(AtomicWriter {
            mode,
            target: target.to_path_buf(),
            buffer: Vec::new(),
            temp_file: None,
        })
    }

    /// Write data (buffered in simple mode)
    pub fn write_all(&mut self, buf: &[u8]) -> Result<()> {
        match self.mode {
            WriteMode::Simple => {
                self.buffer.extend_from_slice(buf);
                Ok(())
            }
            WriteMode::Streaming => {
                // Initialize temp file on first write
                if self.temp_file.is_none() {
                    self.temp_file = Some(
                        atomic_write_file::AtomicWriteFile::open(&self.target)
                            .with_context(|| {
                                format!("Failed to create temp file for: {}", self.target.display())
                            })?
                    );
                }

                self.temp_file.as_mut().unwrap().write_all(buf)
                    .with_context(|| "Failed to write to temp file")?;
                Ok(())
            }
        }
    }

    /// Commit the write (atomic rename)
    pub fn commit(mut self) -> Result<()> {
        match self.mode {
            WriteMode::Simple => {
                let mut temp = atomic_write_file::AtomicWriteFile::open(&self.target)
                    .with_context(|| {
                        format!("Failed to create temp file for: {}", self.target.display())
                    })?;

                temp.write_all(&self.buffer)
                    .with_context(|| "Failed to write to temp file")?;

                temp.commit()
                    .with_context(|| {
                        format!("Failed to commit write to: {}", self.target.display())
                    })?;
            }
            WriteMode::Streaming => {
                if let Some(temp) = self.temp_file.take() {
                    temp.commit()
                        .with_context(|| {
                            format!("Failed to commit write to: {}", self.target.display())
                        })?;
                } else {
                    // No writes happened, create empty file
                    let temp = atomic_write_file::AtomicWriteFile::open(&self.target)
                        .with_context(|| {
                            format!("Failed to create temp file for: {}", self.target.display())
                        })?;
                    temp.commit()
                        .with_context(|| {
                            format!("Failed to commit write to: {}", self.target.display())
                        })?;
                }
            }
        }
        Ok(())
    }
}
```

**Step 4: Update lib.rs exports**

Modify `src/lib.rs`:

```rust
//! Atomic file write library with file locking support

pub mod lock;
pub mod write;

pub use lock::{FileLock, LockStrategy};
pub use write::{AtomicWriter, WriteMode};
```

**Step 5: Run tests**

```bash
cargo test write_test
```

Expected: All write tests PASS

**Step 6: Commit**

```bash
git add src/write.rs src/lib.rs tests/write_test.rs
git commit -m "feat: implement atomic writer with simple and streaming modes"
```

---

### Task 3.2: Integrate Lock + Write

**Files:**
- Create: `tests/integration_lock_write_test.rs`
- Modify: `src/write.rs`

**Step 1: Write integration test**

Create `tests/integration_lock_write_test.rs`:

```rust
use atomic_write::{AtomicWriter, FileLock, LockStrategy, WriteMode};
use std::fs;
use std::io::Write;
use tempfile::TempDir;

#[test]
fn test_lock_and_write_integration() {
    let dir = TempDir::new().unwrap();
    let target = dir.path().join("test.txt");
    let lock_path = target.with_extension("lock");

    let _lock = FileLock::acquire(&lock_path, LockStrategy::Wait).unwrap();

    let mut writer = AtomicWriter::new(&target, WriteMode::Simple).unwrap();
    writer.write_all(b"locked write").unwrap();
    writer.commit().unwrap();

    assert_eq!(fs::read_to_string(&target).unwrap(), "locked write");

    drop(_lock);
    assert!(!lock_path.exists());
}

#[test]
fn test_concurrent_write_blocks() {
    use std::sync::Arc;
    use std::sync::atomic::{AtomicBool, Ordering};
    use std::thread;
    use std::time::Duration;

    let dir = TempDir::new().unwrap();
    let target = Arc::new(dir.path().join("test.txt"));
    let lock_path = target.with_extension("lock");

    let blocked = Arc::new(AtomicBool::new(false));
    let blocked_clone = blocked.clone();
    let target_clone = target.clone();

    // Thread 1: Hold lock for 1 second
    let t1 = thread::spawn(move || {
        let _lock = FileLock::acquire(&lock_path, LockStrategy::Wait).unwrap();
        thread::sleep(Duration::from_millis(500));
    });

    // Give thread 1 time to acquire lock
    thread::sleep(Duration::from_millis(100));

    // Thread 2: Try to acquire with no-wait
    let t2 = thread::spawn(move || {
        let lock_path2 = target_clone.with_extension("lock");
        let result = FileLock::acquire(&lock_path2, LockStrategy::NoWait);
        if result.is_err() {
            blocked_clone.store(true, Ordering::SeqCst);
        }
    });

    t1.join().unwrap();
    t2.join().unwrap();

    assert!(blocked.load(Ordering::SeqCst), "Second lock should have been blocked");
}
```

**Step 2: Run test**

```bash
cargo test test_lock_and_write_integration
```

Expected: PASS

**Step 3: Run test for concurrent blocking**

```bash
cargo test test_concurrent_write_blocks
```

Expected: PASS

**Step 4: Commit**

```bash
git add tests/integration_lock_write_test.rs
git commit -m "test: add integration tests for lock + write"
```

---

## Phase 4: Backup Functionality

### Task 4.1: Backup Module

**Files:**
- Create: `src/backup.rs`
- Create: `tests/backup_test.rs`

**Step 1: Write test for backup creation**

Create `tests/backup_test.rs`:

```rust
use atomic_write::backup::{BackupConfig, create_backup};
use std::fs;
use tempfile::TempDir;

#[test]
fn test_simple_backup_creation() {
    let dir = TempDir::new().unwrap();
    let target = dir.path().join("test.txt");
    fs::write(&target, "original content").unwrap();

    let config = BackupConfig {
        suffix: ".backup".to_string(),
        timestamp: false,
        backup_dir: None,
    };

    create_backup(&target, &config).unwrap();

    let backup_path = target.with_extension("txt.backup");
    assert!(backup_path.exists());
    assert_eq!(fs::read_to_string(&backup_path).unwrap(), "original content");
}

#[test]
fn test_backup_with_timestamp() {
    let dir = TempDir::new().unwrap();
    let target = dir.path().join("test.txt");
    fs::write(&target, "original").unwrap();

    let config = BackupConfig {
        suffix: ".backup".to_string(),
        timestamp: true,
        backup_dir: None,
    };

    let backup_path = create_backup(&target, &config).unwrap();

    assert!(backup_path.exists());
    assert!(backup_path.file_name().unwrap().to_str().unwrap().contains("test.txt."));
    assert!(backup_path.file_name().unwrap().to_str().unwrap().contains(".backup"));
    assert_eq!(fs::read_to_string(&backup_path).unwrap(), "original");
}

#[test]
fn test_backup_to_directory() {
    let dir = TempDir::new().unwrap();
    let backup_dir = dir.path().join("backups");
    fs::create_dir(&backup_dir).unwrap();

    let target = dir.path().join("test.txt");
    fs::write(&target, "original").unwrap();

    let config = BackupConfig {
        suffix: ".backup".to_string(),
        timestamp: false,
        backup_dir: Some(backup_dir.clone()),
    };

    create_backup(&target, &config).unwrap();

    let backup_path = backup_dir.join("test.txt.backup");
    assert!(backup_path.exists());
    assert_eq!(fs::read_to_string(&backup_path).unwrap(), "original");
}

#[test]
fn test_backup_nonexistent_file_skips() {
    let dir = TempDir::new().unwrap();
    let target = dir.path().join("nonexistent.txt");

    let config = BackupConfig {
        suffix: ".backup".to_string(),
        timestamp: false,
        backup_dir: None,
    };

    let result = create_backup(&target, &config);
    assert!(result.is_ok());

    let backup_path = target.with_extension("txt.backup");
    assert!(!backup_path.exists());
}
```

**Step 2: Run test to verify it fails**

```bash
cargo test test_simple_backup_creation
```

Expected: FAIL (backup module doesn't exist)

**Step 3: Implement backup module**

Create `src/backup.rs`:

```rust
use anyhow::{Context, Result};
use std::fs;
use std::path::{Path, PathBuf};

#[derive(Debug, Clone)]
pub struct BackupConfig {
    pub suffix: String,
    pub timestamp: bool,
    pub backup_dir: Option<PathBuf>,
}

/// Create a backup of the target file
pub fn create_backup(target: &Path, config: &BackupConfig) -> Result<PathBuf> {
    // Skip if target doesn't exist
    if !target.exists() {
        return Ok(PathBuf::new());
    }

    // Generate backup filename
    let filename = target.file_name()
        .context("Invalid target filename")?
        .to_str()
        .context("Filename is not valid UTF-8")?;

    let backup_name = if config.timestamp {
        let timestamp = chrono::Local::now().format("%Y%m%d-%H%M%S");
        format!("{}.{}{}", filename, timestamp, config.suffix)
    } else {
        format!("{}{}", filename, config.suffix)
    };

    // Determine backup location
    let backup_path = if let Some(ref backup_dir) = config.backup_dir {
        if !backup_dir.exists() {
            anyhow::bail!(
                "Backup directory does not exist: {}\nHint: Create it first: mkdir -p {}",
                backup_dir.display(),
                backup_dir.display()
            );
        }
        backup_dir.join(backup_name)
    } else {
        target.with_file_name(backup_name)
    };

    // Create backup using atomic rename
    fs::copy(target, &backup_path)
        .with_context(|| format!("Failed to create backup: {}", backup_path.display()))?;

    Ok(backup_path)
}
```

**Step 4: Add chrono dependency**

Modify `Cargo.toml`:

```toml
[dependencies]
clap = { version = "4.5", features = ["derive", "wrap_help"] }
atomic-write-file = "0.2"
fs2 = "0.4"
anyhow = "1.0"
libc = "0.2"
chrono = "0.4"
```

**Step 5: Update lib.rs exports**

Modify `src/lib.rs`:

```rust
//! Atomic file write library with file locking support

pub mod lock;
pub mod write;
pub mod backup;

pub use lock::{FileLock, LockStrategy};
pub use write::{AtomicWriter, WriteMode};
pub use backup::{BackupConfig, create_backup};
```

**Step 6: Run tests**

```bash
cargo test backup_test
```

Expected: All backup tests PASS

**Step 7: Commit**

```bash
git add src/backup.rs src/lib.rs Cargo.toml tests/backup_test.rs
git commit -m "feat: implement backup functionality with timestamp support"
```

---

## Phase 5: Housekeeping Module

### Task 5.1: Lock File Cleanup

**Files:**
- Create: `src/housekeep.rs`
- Create: `tests/housekeep_test.rs`

**Step 1: Write test for lock cleanup**

Create `tests/housekeep_test.rs`:

```rust
use atomic_write::housekeep::{clean_locks, CleanLockConfig};
use std::fs::{self, File};
use std::path::PathBuf;
use tempfile::TempDir;
use std::time::{Duration, SystemTime};

#[test]
fn test_clean_orphaned_locks() {
    let dir = TempDir::new().unwrap();

    // Create orphaned lock file
    let lock1 = dir.path().join("file1.lock");
    File::create(&lock1).unwrap();

    let config = CleanLockConfig {
        dir: dir.path().to_path_buf(),
        recursive: false,
        older_than: None,
        dry_run: false,
    };

    let cleaned = clean_locks(&config).unwrap();

    assert_eq!(cleaned.len(), 1);
    assert_eq!(cleaned[0], lock1);
    assert!(!lock1.exists());
}

#[test]
fn test_skip_active_locks() {
    let dir = TempDir::new().unwrap();

    let lock_path = dir.path().join("active.lock");
    let _active_lock = atomic_write::FileLock::acquire(
        &lock_path,
        atomic_write::LockStrategy::Wait
    ).unwrap();

    let config = CleanLockConfig {
        dir: dir.path().to_path_buf(),
        recursive: false,
        older_than: None,
        dry_run: false,
    };

    let cleaned = clean_locks(&config).unwrap();

    assert_eq!(cleaned.len(), 0);
    assert!(lock_path.exists());
}

#[test]
fn test_dry_run_doesnt_delete() {
    let dir = TempDir::new().unwrap();
    let lock1 = dir.path().join("file1.lock");
    File::create(&lock1).unwrap();

    let config = CleanLockConfig {
        dir: dir.path().to_path_buf(),
        recursive: false,
        older_than: None,
        dry_run: true,
    };

    let would_clean = clean_locks(&config).unwrap();

    assert_eq!(would_clean.len(), 1);
    assert!(lock1.exists(), "Dry run should not delete");
}

#[test]
fn test_older_than_filter() {
    let dir = TempDir::new().unwrap();

    // Old lock (2 hours ago)
    let old_lock = dir.path().join("old.lock");
    File::create(&old_lock).unwrap();
    let two_hours_ago = SystemTime::now() - Duration::from_secs(2 * 3600);
    filetime::set_file_mtime(&old_lock, filetime::FileTime::from_system_time(two_hours_ago)).unwrap();

    // Recent lock (30 minutes ago)
    let recent_lock = dir.path().join("recent.lock");
    File::create(&recent_lock).unwrap();

    let config = CleanLockConfig {
        dir: dir.path().to_path_buf(),
        recursive: false,
        older_than: Some(Duration::from_secs(3600)), // 1 hour
        dry_run: false,
    };

    let cleaned = clean_locks(&config).unwrap();

    assert_eq!(cleaned.len(), 1);
    assert_eq!(cleaned[0], old_lock);
    assert!(recent_lock.exists(), "Recent lock should not be cleaned");
}
```

**Step 2: Add filetime dev dependency**

Modify `Cargo.toml`:

```toml
[dev-dependencies]
tempfile = "3.8"
assert_cmd = "2.0"
predicates = "3.0"
filetime = "0.2"
```

**Step 3: Run test to verify it fails**

```bash
cargo test test_clean_orphaned_locks
```

Expected: FAIL (housekeep module doesn't exist)

**Step 4: Implement lock cleanup**

Create `src/housekeep.rs`:

```rust
use anyhow::{Context, Result};
use std::fs::{self, File};
use std::path::{Path, PathBuf};
use std::time::{Duration, SystemTime};
use fs2::FileExt;

#[derive(Debug, Clone)]
pub struct CleanLockConfig {
    pub dir: PathBuf,
    pub recursive: bool,
    pub older_than: Option<Duration>,
    pub dry_run: bool,
}

#[derive(Debug, Clone)]
pub struct CleanBackupConfig {
    pub dir: PathBuf,
    pub recursive: bool,
    pub older_than: Option<Duration>,
    pub keep_newest: Option<usize>,
    pub dry_run: bool,
}

/// Clean orphaned lock files
pub fn clean_locks(config: &CleanLockConfig) -> Result<Vec<PathBuf>> {
    let mut cleaned = Vec::new();

    visit_directory(&config.dir, config.recursive, |path| {
        if is_lock_file(path) && is_orphaned(path, config.older_than)? {
            if config.dry_run {
                cleaned.push(path.to_path_buf());
            } else {
                fs::remove_file(path)
                    .with_context(|| format!("Failed to remove lock file: {}", path.display()))?;
                cleaned.push(path.to_path_buf());
            }
        }
        Ok(())
    })?;

    Ok(cleaned)
}

/// Clean old backup files
pub fn clean_backups(config: &CleanBackupConfig) -> Result<Vec<PathBuf>> {
    use std::collections::HashMap;

    let mut backups: HashMap<String, Vec<(PathBuf, SystemTime)>> = HashMap::new();

    // Collect all backups grouped by base filename
    visit_directory(&config.dir, config.recursive, |path| {
        if is_backup_file(path) {
            let base = extract_base_filename(path);
            let mtime = fs::metadata(path)?.modified()?;
            backups.entry(base).or_insert_with(Vec::new).push((path.to_path_buf(), mtime));
        }
        Ok(())
    })?;

    let mut cleaned = Vec::new();

    // Process each group of backups
    for (_, mut group) in backups {
        // Sort by modification time (newest first)
        group.sort_by(|a, b| b.1.cmp(&a.1));

        for (idx, (path, mtime)) in group.iter().enumerate() {
            let mut should_delete = false;

            // Check keep_newest
            if let Some(keep) = config.keep_newest {
                if idx >= keep {
                    should_delete = true;
                }
            }

            // Check older_than
            if let Some(max_age) = config.older_than {
                if let Ok(elapsed) = SystemTime::now().duration_since(*mtime) {
                    if elapsed > max_age {
                        should_delete = true;
                    }
                }
            }

            if should_delete {
                if config.dry_run {
                    cleaned.push(path.clone());
                } else {
                    fs::remove_file(path)
                        .with_context(|| format!("Failed to remove backup: {}", path.display()))?;
                    cleaned.push(path.clone());
                }
            }
        }
    }

    Ok(cleaned)
}

fn visit_directory<F>(dir: &Path, recursive: bool, mut visitor: F) -> Result<()>
where
    F: FnMut(&Path) -> Result<()>,
{
    for entry in fs::read_dir(dir)
        .with_context(|| format!("Failed to read directory: {}", dir.display()))?
    {
        let entry = entry?;
        let path = entry.path();

        if path.is_dir() && recursive {
            visit_directory(&path, recursive, &mut visitor)?;
        } else if path.is_file() {
            visitor(&path)?;
        }
    }
    Ok(())
}

fn is_lock_file(path: &Path) -> bool {
    path.extension().and_then(|s| s.to_str()) == Some("lock")
}

fn is_backup_file(path: &Path) -> bool {
    if let Some(name) = path.file_name().and_then(|s| s.to_str()) {
        name.contains(".backup") || name.contains(".bak")
    } else {
        false
    }
}

fn extract_base_filename(path: &Path) -> String {
    let name = path.file_name().unwrap().to_str().unwrap();
    // Extract base by removing timestamp and backup suffix
    // e.g., "config.json.20260125-143022.backup" -> "config.json"
    if let Some(pos) = name.find(".20") {
        name[..pos].to_string()
    } else if let Some(pos) = name.rfind(".backup") {
        name[..pos].to_string()
    } else if let Some(pos) = name.rfind(".bak") {
        name[..pos].to_string()
    } else {
        name.to_string()
    }
}

fn is_orphaned(lock_path: &Path, older_than: Option<Duration>) -> Result<bool> {
    // Check age filter first
    if let Some(max_age) = older_than {
        let metadata = fs::metadata(lock_path)?;
        let mtime = metadata.modified()?;
        if let Ok(elapsed) = SystemTime::now().duration_since(mtime) {
            if elapsed < max_age {
                return Ok(false);
            }
        }
    }

    // Try to acquire lock - if successful, it's orphaned
    let file = File::open(lock_path)?;
    match file.try_lock_exclusive() {
        Ok(_) => {
            // Successfully locked = orphaned
            let _ = file.unlock();
            Ok(true)
        }
        Err(e) if e.kind() == std::io::ErrorKind::WouldBlock => {
            // Lock held by another process = not orphaned
            Ok(false)
        }
        Err(e) => Err(e.into()),
    }
}
```

**Step 5: Update lib.rs exports**

Modify `src/lib.rs`:

```rust
//! Atomic file write library with file locking support

pub mod lock;
pub mod write;
pub mod backup;
pub mod housekeep;

pub use lock::{FileLock, LockStrategy};
pub use write::{AtomicWriter, WriteMode};
pub use backup::{BackupConfig, create_backup};
pub use housekeep::{clean_locks, clean_backups, CleanLockConfig, CleanBackupConfig};
```

**Step 6: Run tests**

```bash
cargo test housekeep_test
```

Expected: All housekeep tests PASS

**Step 7: Commit**

```bash
git add src/housekeep.rs src/lib.rs Cargo.toml tests/housekeep_test.rs
git commit -m "feat: implement housekeeping with lock and backup cleanup"
```

---

## Phase 6: CLI Implementation

### Task 6.1: Main Write Command Handler

**Files:**
- Create: `src/cli/write_command.rs`
- Modify: `src/cli/mod.rs`

**Step 1: Write integration test for write command**

Create `tests/cli_write_test.rs`:

```rust
use assert_cmd::Command;
use predicates::prelude::*;
use std::fs;
use tempfile::TempDir;

#[test]
fn test_write_from_stdin() {
    let dir = TempDir::new().unwrap();
    let output = dir.path().join("output.txt");

    let mut cmd = Command::cargo_bin("atomic-write").unwrap();
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

    let mut cmd = Command::cargo_bin("atomic-write").unwrap();
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

    let mut cmd = Command::cargo_bin("atomic-write").unwrap();
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

    let mut cmd = Command::cargo_bin("atomic-write").unwrap();
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

    let mut cmd = Command::cargo_bin("atomic-write").unwrap();
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
        let _lock = atomic_write::FileLock::acquire(
            &lock_path,
            atomic_write::LockStrategy::Wait
        ).unwrap();
        thread::sleep(Duration::from_secs(2));
    });

    thread::sleep(Duration::from_millis(100));

    let mut cmd = Command::cargo_bin("atomic-write").unwrap();
    cmd.arg("--no-wait")
        .arg(output.to_str().unwrap())
        .write_stdin("should fail")
        .assert()
        .failure()
        .code(2)
        .stderr(predicate::str::contains("locked"));

    handle.join().unwrap();
}
```

**Step 2: Run test to verify it fails**

```bash
cargo test test_write_from_stdin
```

Expected: FAIL (command handler not implemented)

**Step 3: Implement write command handler**

Create `src/cli/write_command.rs`:

```rust
use crate::cli::Args;
use crate::{AtomicWriter, BackupConfig, FileLock, LockStrategy, WriteMode, create_backup};
use anyhow::{Context, Result};
use std::fs::File;
use std::io::{self, Read, Write};
use std::path::PathBuf;
use std::time::Duration;

pub fn execute_write(args: Args) -> Result<()> {
    let output = args.output.context("Output file required")?;

    // Determine lock strategy
    let lock_strategy = if args.no_wait {
        LockStrategy::NoWait
    } else if let Some(timeout) = args.timeout {
        LockStrategy::Timeout(Duration::from_secs(timeout))
    } else {
        LockStrategy::Wait
    };

    // Determine lock file path
    let lock_path = args.lock_file.unwrap_or_else(|| {
        output.with_extension(
            format!("{}.lock", output.extension().unwrap_or_default().to_str().unwrap())
        )
    });

    // Acquire lock
    let _lock = FileLock::acquire(&lock_path, lock_strategy)
        .context("Failed to acquire lock")?;

    if args.verbose > 0 {
        eprintln!("Lock acquired: {}", lock_path.display());
    }

    // Create backup if requested
    if args.backup {
        let backup_config = BackupConfig {
            suffix: args.backup_suffix,
            timestamp: args.backup_timestamp,
            backup_dir: args.backup_dir,
        };

        let backup_path = create_backup(&output, &backup_config)?;
        if args.verbose > 0 && backup_path.exists() {
            eprintln!("Backup created: {}", backup_path.display());
        }
    }

    // Determine write mode
    let mode = if args.stream {
        WriteMode::Streaming
    } else {
        WriteMode::Simple
    };

    // Create writer
    let mut writer = AtomicWriter::new(&output, mode)?;

    // Read input
    let mut input: Box<dyn Read> = if let Some(input_file) = args.input {
        Box::new(File::open(&input_file)
            .with_context(|| format!("Failed to open input file: {}", input_file.display()))?)
    } else {
        Box::new(io::stdin())
    };

    // Copy data
    let mut buffer = [0u8; 8192];
    loop {
        let n = input.read(&mut buffer)?;
        if n == 0 {
            break;
        }
        writer.write_all(&buffer[..n])?;
    }

    // Commit write
    writer.commit()?;

    if args.verbose > 0 {
        eprintln!("Write completed: {}", output.display());
    }

    Ok(())
}
```

**Step 4: Update CLI mod to use write command**

Modify `src/cli/mod.rs`:

```rust
mod args;
mod write_command;

pub use args::{Args, Command};
use anyhow::Result;

pub fn run(args: Args) -> Result<()> {
    match args.command {
        Some(Command::Housekeep { .. }) => {
            // TODO: Implement housekeep
            eprintln!("Housekeep not yet implemented");
            Ok(())
        }
        None => {
            write_command::execute_write(args)
        }
    }
}
```

**Step 5: Run tests**

```bash
cargo test cli_write_test
```

Expected: All CLI write tests PASS

**Step 6: Commit**

```bash
git add src/cli/write_command.rs src/cli/mod.rs tests/cli_write_test.rs
git commit -m "feat: implement main write command handler"
```

---

### Task 6.2: Housekeep Command Handler

**Files:**
- Create: `src/cli/housekeep_command.rs`
- Modify: `src/cli/mod.rs`
- Create: `tests/cli_housekeep_test.rs`

**Step 1: Write integration test**

Create `tests/cli_housekeep_test.rs`:

```rust
use assert_cmd::Command;
use predicates::prelude::*;
use std::fs::{self, File};
use tempfile::TempDir;

#[test]
fn test_housekeep_clean_locks() {
    let dir = TempDir::new().unwrap();

    // Create orphaned lock
    let lock = dir.path().join("file.lock");
    File::create(&lock).unwrap();

    let mut cmd = Command::cargo_bin("atomic-write").unwrap();
    cmd.arg("housekeep")
        .arg("--clean-locks")
        .arg(dir.path())
        .assert()
        .success()
        .stdout(predicate::str::contains("file.lock"));

    assert!(!lock.exists());
}

#[test]
fn test_housekeep_dry_run() {
    let dir = TempDir::new().unwrap();
    let lock = dir.path().join("file.lock");
    File::create(&lock).unwrap();

    let mut cmd = Command::cargo_bin("atomic-write").unwrap();
    cmd.arg("housekeep")
        .arg("--clean-locks")
        .arg("--dry-run")
        .arg(dir.path())
        .assert()
        .success()
        .stdout(predicate::str::contains("file.lock"));

    assert!(lock.exists(), "Dry run should not delete");
}

#[test]
fn test_housekeep_clean_backups() {
    let dir = TempDir::new().unwrap();

    // Create backup files
    fs::write(dir.path().join("file.txt.backup"), "backup1").unwrap();
    fs::write(dir.path().join("file.txt.20260125-120000.backup"), "backup2").unwrap();

    let mut cmd = Command::cargo_bin("atomic-write").unwrap();
    cmd.arg("housekeep")
        .arg("--clean-backups")
        .arg("--keep-newest")
        .arg("1")
        .arg(dir.path())
        .assert()
        .success();

    // Should keep one backup
    let backups: Vec<_> = fs::read_dir(dir.path())
        .unwrap()
        .filter_map(|e| e.ok())
        .filter(|e| e.path().to_str().unwrap().contains("backup"))
        .collect();

    assert_eq!(backups.len(), 1);
}

#[test]
fn test_housekeep_requires_operation() {
    let dir = TempDir::new().unwrap();

    let mut cmd = Command::cargo_bin("atomic-write").unwrap();
    cmd.arg("housekeep")
        .arg(dir.path())
        .assert()
        .failure()
        .stderr(predicate::str::contains("at least one operation"));
}
```

**Step 2: Run test to verify it fails**

```bash
cargo test test_housekeep_clean_locks
```

Expected: FAIL (housekeep command not implemented)

**Step 3: Implement housekeep command**

Create `src/cli/housekeep_command.rs`:

```rust
use crate::cli::Command;
use crate::housekeep::{clean_backups, clean_locks, CleanBackupConfig, CleanLockConfig};
use anyhow::{bail, Result};
use std::path::PathBuf;
use std::time::Duration;

pub fn execute_housekeep(cmd: Command) -> Result<()> {
    if let Command::Housekeep {
        dir,
        clean_locks: do_clean_locks,
        clean_backups: do_clean_backups,
        all,
        recursive,
        older_than,
        keep_newest,
        dry_run,
        verbose,
        json,
    } = cmd
    {
        let target_dir = dir.unwrap_or_else(|| PathBuf::from("."));

        if !do_clean_locks && !do_clean_backups && !all {
            bail!("Error: Specify at least one operation: --clean-locks, --clean-backups, or --all");
        }

        let mut total_cleaned = 0;

        // Clean locks
        if do_clean_locks || all {
            let duration = parse_duration_hours(&older_than)?;
            let config = CleanLockConfig {
                dir: target_dir.clone(),
                recursive,
                older_than: duration,
                dry_run,
            };

            let cleaned = clean_locks(&config)?;

            if json {
                println!("{{\"operation\": \"clean_locks\", \"count\": {}, \"files\": {:?}}}",
                    cleaned.len(), cleaned);
            } else {
                for path in &cleaned {
                    println!("{}{}",
                        if dry_run { "[DRY RUN] Would delete: " } else { "Deleted: " },
                        path.display()
                    );
                }
                if verbose || dry_run {
                    eprintln!("Cleaned {} lock file(s)", cleaned.len());
                }
            }

            total_cleaned += cleaned.len();
        }

        // Clean backups
        if do_clean_backups || all {
            let duration = parse_duration_days(&older_than)?;
            let config = CleanBackupConfig {
                dir: target_dir.clone(),
                recursive,
                older_than: duration,
                keep_newest,
                dry_run,
            };

            let cleaned = clean_backups(&config)?;

            if json {
                println!("{{\"operation\": \"clean_backups\", \"count\": {}, \"files\": {:?}}}",
                    cleaned.len(), cleaned);
            } else {
                for path in &cleaned {
                    println!("{}{}",
                        if dry_run { "[DRY RUN] Would delete: " } else { "Deleted: " },
                        path.display()
                    );
                }
                if verbose || dry_run {
                    eprintln!("Cleaned {} backup file(s)", cleaned.len());
                }
            }

            total_cleaned += cleaned.len();
        }

        if !json && verbose {
            eprintln!("Total: {} file(s) cleaned", total_cleaned);
        }

        Ok(())
    } else {
        unreachable!()
    }
}

fn parse_duration_hours(s: &Option<String>) -> Result<Option<Duration>> {
    match s {
        Some(s) => {
            if s.ends_with('h') {
                let hours: u64 = s[..s.len()-1].parse()?;
                Ok(Some(Duration::from_secs(hours * 3600)))
            } else {
                let hours: u64 = s.parse()?;
                Ok(Some(Duration::from_secs(hours * 3600)))
            }
        }
        None => Ok(None),
    }
}

fn parse_duration_days(s: &Option<String>) -> Result<Option<Duration>> {
    match s {
        Some(s) => {
            if s.ends_with('d') {
                let days: u64 = s[..s.len()-1].parse()?;
                Ok(Some(Duration::from_secs(days * 86400)))
            } else {
                let days: u64 = s.parse()?;
                Ok(Some(Duration::from_secs(days * 86400)))
            }
        }
        None => Ok(None),
    }
}
```

**Step 4: Update CLI mod**

Modify `src/cli/mod.rs`:

```rust
mod args;
mod write_command;
mod housekeep_command;

pub use args::{Args, Command};
use anyhow::Result;

pub fn run(args: Args) -> Result<()> {
    match args.command {
        Some(cmd @ Command::Housekeep { .. }) => {
            housekeep_command::execute_housekeep(cmd)
        }
        None => {
            write_command::execute_write(args)
        }
    }
}
```

**Step 5: Run tests**

```bash
cargo test cli_housekeep_test
```

Expected: All housekeep CLI tests PASS

**Step 6: Commit**

```bash
git add src/cli/housekeep_command.rs src/cli/mod.rs tests/cli_housekeep_test.rs
git commit -m "feat: implement housekeep command handler"
```

---

## Phase 7: Error Handling & Exit Codes

### Task 7.1: Structured Error Types

**Files:**
- Create: `src/error.rs`
- Modify: `src/lib.rs`
- Modify: `src/main.rs`

**Step 1: Write test for exit codes**

Create `tests/exit_codes_test.rs`:

```rust
use assert_cmd::Command;
use std::fs::File;
use std::thread;
use std::time::Duration;
use tempfile::TempDir;

#[test]
fn test_exit_code_0_on_success() {
    let dir = TempDir::new().unwrap();
    let output = dir.path().join("test.txt");

    let mut cmd = Command::cargo_bin("atomic-write").unwrap();
    cmd.arg(output)
        .write_stdin("data")
        .assert()
        .code(0);
}

#[test]
fn test_exit_code_1_on_permission_error() {
    let mut cmd = Command::cargo_bin("atomic-write").unwrap();
    cmd.arg("/root/forbidden.txt")
        .write_stdin("data")
        .assert()
        .code(1);
}

#[test]
fn test_exit_code_2_on_lock_timeout() {
    let dir = TempDir::new().unwrap();
    let output = dir.path().join("test.txt");
    let lock_path = output.with_extension("lock");

    let _lock = atomic_write::FileLock::acquire(
        &lock_path,
        atomic_write::LockStrategy::Wait
    ).unwrap();

    let mut cmd = Command::cargo_bin("atomic-write").unwrap();
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
    let lock_path = output.with_extension("lock");

    let _lock = atomic_write::FileLock::acquire(
        &lock_path,
        atomic_write::LockStrategy::Wait
    ).unwrap();

    let mut cmd = Command::cargo_bin("atomic-write").unwrap();
    cmd.arg("--no-wait")
        .arg(output)
        .write_stdin("data")
        .assert()
        .code(2);
}
```

**Step 2: Run test**

```bash
cargo test test_exit_code_0_on_success
```

Expected: PASS (basic success case should work)

**Step 3: Run lock timeout test**

```bash
cargo test test_exit_code_2_on_lock_timeout
```

Expected: May FAIL if exit code not properly mapped

**Step 4: Create error module**

Create `src/error.rs`:

```rust
use anyhow::Error;
use std::fmt;

#[derive(Debug)]
pub enum ErrorKind {
    LockFailed,
    Timeout,
    PermissionDenied,
    Interrupted,
    General,
}

pub struct AppError {
    kind: ErrorKind,
    source: Error,
}

impl AppError {
    pub fn new(kind: ErrorKind, source: Error) -> Self {
        AppError { kind, source }
    }

    pub fn exit_code(&self) -> i32 {
        match self.kind {
            ErrorKind::LockFailed | ErrorKind::Timeout => 2,
            ErrorKind::Interrupted => 3,
            ErrorKind::PermissionDenied | ErrorKind::General => 1,
        }
    }

    pub fn from_anyhow(err: Error) -> Self {
        let msg = err.to_string().to_lowercase();

        let kind = if msg.contains("lock") && (msg.contains("timeout") || msg.contains("acquisition")) {
            ErrorKind::Timeout
        } else if msg.contains("locked") || msg.contains("would block") {
            ErrorKind::LockFailed
        } else if msg.contains("permission denied") {
            ErrorKind::PermissionDenied
        } else if msg.contains("interrupt") {
            ErrorKind::Interrupted
        } else {
            ErrorKind::General
        };

        AppError { kind, source: err }
    }
}

impl fmt::Display for AppError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "Error: {}", self.source)
    }
}

impl fmt::Debug for AppError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{}", self.source)
    }
}
```

**Step 5: Update main.rs for proper exit codes**

Modify `src/main.rs`:

```rust
use clap::Parser;
use std::process;

mod cli;

fn main() {
    let args = cli::Args::parse();

    if let Err(err) = cli::run(args) {
        let app_err = atomic_write::error::AppError::from_anyhow(err);
        eprintln!("{}", app_err);
        process::exit(app_err.exit_code());
    }
}
```

**Step 6: Update lib.rs**

Modify `src/lib.rs`:

```rust
//! Atomic file write library with file locking support

pub mod lock;
pub mod write;
pub mod backup;
pub mod housekeep;
pub mod error;

pub use lock::{FileLock, LockStrategy};
pub use write::{AtomicWriter, WriteMode};
pub use backup::{BackupConfig, create_backup};
pub use housekeep::{clean_locks, clean_backups, CleanBackupConfig, CleanLockConfig};
```

**Step 7: Run all exit code tests**

```bash
cargo test exit_codes_test
```

Expected: All exit code tests PASS

**Step 8: Commit**

```bash
git add src/error.rs src/main.rs src/lib.rs tests/exit_codes_test.rs
git commit -m "feat: add structured error handling with proper exit codes"
```

---

## Phase 8: Documentation

### Task 8.1: README and Usage Examples

**Files:**
- Create: `README.md`
- Create: `CHANGELOG.md`

**Step 1: Create README**

```bash
cat > README.md << 'EOF'
# atomic-write

A command-line tool for atomic file writes with process coordination through file locking.

## Features

- **Atomic writes**: All-or-nothing file updates using atomic rename
- **File locking**: Advisory locks prevent concurrent write conflicts
- **Backup support**: Optional backups with timestamps
- **Streaming mode**: Process large files with constant memory usage
- **Housekeeping**: Clean up orphaned locks and old backups

## Installation

### From source

```bash
cargo install --path .
```

### From crates.io

```bash
cargo install atomic-write
```

## Quick Start

```bash
# Basic usage
echo "new content" | atomic-write config.json

# With backup
echo "new content" | atomic-write --backup config.json

# Large file streaming
cat large_file.csv | atomic-write --stream output.csv

# Wait for lock with timeout
generate_config.sh | atomic-write --timeout 30 config.json

# Fail fast if locked
atomic-write --no-wait config.json < data.txt
```

## Usage

### Write Command

```
atomic-write [OPTIONS] <OUTPUT>
```

**Options:**
- `-i, --input <FILE>`: Read from file instead of stdin
- `--stream`: Use streaming mode for large files
- `--wait`: Wait for lock (default)
- `--no-wait`: Fail immediately if locked
- `-t, --timeout <SECONDS>`: Lock acquisition timeout
- `-b, --backup`: Create backup before overwrite
- `--backup-suffix <SUFFIX>`: Custom backup suffix (default: .backup)
- `--backup-timestamp`: Add timestamp to backup
- `-v`: Verbose output (-vv for debug)

### Housekeep Command

```
atomic-write housekeep [OPTIONS] [DIR]
```

**Options:**
- `--clean-locks`: Clean orphaned lock files
- `--clean-backups`: Clean old backup files
- `--all`: Clean both locks and backups
- `-r, --recursive`: Scan subdirectories
- `--older-than <DURATION>`: Age threshold (e.g., "2h", "7d")
- `--keep-newest <N>`: Keep N newest backups per file
- `-n, --dry-run`: Show what would be deleted

## Examples

### Configuration File Updates

```bash
# Update JSON config atomically
jq '.database.max_connections = 100' config.json | atomic-write config.json

# With backup for safety
jq '.setting = "new"' app.json | atomic-write --backup app.json
```

### Concurrent Cron Jobs

```bash
# Multiple cron jobs writing to same file
* * * * * process_logs.sh | atomic-write --wait /var/log/summary.log
* * * * * analyze_metrics.sh | atomic-write --wait /var/log/summary.log
```

### Large File Processing

```bash
# Stream large CSV without loading into memory
transform_data.py < input.csv | atomic-write --stream output.csv
```

### Lock Cleanup

```bash
# Clean locks older than 1 hour
atomic-write housekeep --clean-locks --older-than 1h /var/lib/app

# Keep only 3 newest backups
atomic-write housekeep --clean-backups --keep-newest 3 /data
```

## Exit Codes

- `0`: Success
- `1`: General error (I/O, permission denied, invalid arguments)
- `2`: Lock acquisition failed (timeout or no-wait)
- `3`: Interrupted (SIGINT, SIGTERM)

## Limitations

- **Advisory locks only**: Non-cooperating processes can still write
- **Unix/Linux/macOS only**: Windows support planned for v2.0
- **Lock files orphaned on SIGKILL**: Use housekeep command for cleanup
- **Single backup generation**: Use `--backup-timestamp` for multiple versions

## License

MIT OR Apache-2.0

## Contributing

Contributions welcome! Please open an issue before major changes.
EOF
```

**Step 2: Create CHANGELOG**

```bash
cat > CHANGELOG.md << 'EOF'
# Changelog

All notable changes to this project will be documented in this file.

The format is based on [Keep a Changelog](https://keepachangelog.com/en/1.0.0/),
and this project adheres to [Semantic Versioning](https://semver.org/spec/v2.0.0.html).

## [1.0.0] - 2026-01-25

### Added
- Atomic file writes using atomic rename
- File locking with advisory locks
- Simple and streaming write modes
- Backup creation with optional timestamps
- Housekeeping utilities for lock and backup cleanup
- Configurable lock acquisition strategies (wait, no-wait, timeout)
- Proper exit codes for error handling
- Comprehensive test suite

### Security
- Orphaned lock file cleanup via housekeep command
- Atomic operations prevent partial writes
- Permission preservation options
EOF
```

**Step 3: Verify documentation**

```bash
cargo doc --no-deps --open
```

Expected: Documentation builds and opens in browser

**Step 4: Commit**

```bash
git add README.md CHANGELOG.md
git commit -m "docs: add README and CHANGELOG"
```

---

## Phase 9: Final Testing & Polish

### Task 9.1: Comprehensive Integration Tests

**Files:**
- Create: `tests/end_to_end_test.rs`

**Step 1: Create end-to-end test suite**

Create `tests/end_to_end_test.rs`:

```rust
use assert_cmd::Command;
use predicates::prelude::*;
use std::fs;
use std::thread;
use std::time::Duration;
use tempfile::TempDir;

#[test]
fn test_full_workflow_with_backup_and_lock() {
    let dir = TempDir::new().unwrap();
    let target = dir.path().join("config.json");

    // Initial write
    fs::write(&target, r#"{"version": 1}"#).unwrap();

    // Update with backup
    let mut cmd = Command::cargo_bin("atomic-write").unwrap();
    cmd.arg("--backup")
        .arg("--backup-timestamp")
        .arg("-v")
        .arg(&target)
        .write_stdin(r#"{"version": 2}"#)
        .assert()
        .success()
        .stderr(predicate::str::contains("Lock acquired"))
        .stderr(predicate::str::contains("Backup created"));

    // Verify content updated
    assert_eq!(fs::read_to_string(&target).unwrap(), r#"{"version": 2}"#);

    // Verify backup exists
    let backups: Vec<_> = fs::read_dir(dir.path())
        .unwrap()
        .filter_map(|e| e.ok())
        .filter(|e| e.path().to_str().unwrap().contains("backup"))
        .collect();
    assert_eq!(backups.len(), 1);
}

#[test]
fn test_concurrent_writers_with_locking() {
    let dir = TempDir::new().unwrap();
    let target = dir.path().join("counter.txt");
    fs::write(&target, "0").unwrap();

    let handles: Vec<_> = (0..5).map(|i| {
        let target = target.clone();
        thread::spawn(move || {
            thread::sleep(Duration::from_millis(i * 100));

            let mut cmd = Command::cargo_bin("atomic-write").unwrap();
            cmd.arg("--wait")
                .arg(&target)
                .write_stdin(format!("writer-{}", i))
                .assert()
                .success();
        })
    }).collect();

    for h in handles {
        h.join().unwrap();
    }

    // File should have one writer's content (not corrupted)
    let content = fs::read_to_string(&target).unwrap();
    assert!(content.starts_with("writer-"));
}

#[test]
fn test_streaming_large_file() {
    let dir = TempDir::new().unwrap();
    let output = dir.path().join("large.txt");

    // Generate 1MB of data
    let data = "x".repeat(1024 * 1024);

    let mut cmd = Command::cargo_bin("atomic-write").unwrap();
    cmd.arg("--stream")
        .arg(&output)
        .write_stdin(data.clone())
        .assert()
        .success();

    assert_eq!(fs::read_to_string(&output).unwrap(), data);
}

#[test]
fn test_housekeep_full_workflow() {
    let dir = TempDir::new().unwrap();

    // Create various files
    fs::write(dir.path().join("file1.txt"), "data").unwrap();
    fs::write(dir.path().join("file1.txt.backup"), "old").unwrap();
    fs::write(dir.path().join("file2.lock"), "").unwrap();

    // Dry run first
    let mut cmd = Command::cargo_bin("atomic-write").unwrap();
    cmd.arg("housekeep")
        .arg("--all")
        .arg("--dry-run")
        .arg(dir.path())
        .assert()
        .success()
        .stdout(predicate::str::contains("file2.lock"));

    // Verify nothing deleted
    assert!(dir.path().join("file2.lock").exists());

    // Real cleanup
    let mut cmd = Command::cargo_bin("atomic-write").unwrap();
    cmd.arg("housekeep")
        .arg("--all")
        .arg(dir.path())
        .assert()
        .success();

    // Verify lock cleaned
    assert!(!dir.path().join("file2.lock").exists());
}
```

**Step 2: Run all tests**

```bash
cargo test
```

Expected: All tests PASS

**Step 3: Run with --release to verify performance**

```bash
cargo test --release
```

Expected: All tests PASS

**Step 4: Commit**

```bash
git add tests/end_to_end_test.rs
git commit -m "test: add comprehensive end-to-end integration tests"
```

---

### Task 9.2: Build and Package

**Files:**
- Modify: `Cargo.toml`
- Create: `.cargo/config.toml` (optional)

**Step 1: Verify release build**

```bash
cargo build --release
```

Expected: Clean build with no warnings

**Step 2: Test release binary**

```bash
./target/release/atomic-write --version
./target/release/atomic-write --help
```

Expected: Version and help output display correctly

**Step 3: Run clippy**

```bash
cargo clippy -- -D warnings
```

Expected: No warnings or errors

**Step 4: Run fmt check**

```bash
cargo fmt -- --check
```

Expected: Code is properly formatted

**Step 5: Fix any formatting issues**

```bash
cargo fmt
```

**Step 6: Final test run**

```bash
cargo test --all-features
```

Expected: All tests PASS

**Step 7: Commit any fixes**

```bash
git add -A
git commit -m "chore: final polish and formatting"
```

---

## Execution Summary

This implementation plan provides:

1. **Phase 1**: Project scaffolding with proper Rust structure
2. **Phase 2**: Core file locking with Drop-based cleanup
3. **Phase 3**: Atomic write operations (simple + streaming modes)
4. **Phase 4**: Backup functionality with timestamps
5. **Phase 5**: Housekeeping utilities for cleanup
6. **Phase 6**: Complete CLI implementation
7. **Phase 7**: Error handling with proper exit codes
8. **Phase 8**: Documentation (README, CHANGELOG)
9. **Phase 9**: Integration tests and release preparation

**Total estimated steps**: ~70 steps across 9 phases

**Key architectural decisions**:
- TDD approach throughout
- Separate library (`src/lib.rs`) from CLI (`src/cli/`)
- Drop trait for automatic lock cleanup
- atomic-write-file crate for platform-specific atomicity
- Frequent, granular commits

**Testing strategy**:
- Unit tests for each module
- Integration tests for cross-module behavior
- CLI tests using assert_cmd
- End-to-end workflow tests
- Exit code validation

**Next steps after plan execution**:
1. Review generated code for security issues
2. Performance benchmarking
3. Manual testing on target platforms
4. Prepare crates.io release
5. Setup CI/CD pipeline
