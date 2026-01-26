# Post-Release Fixes Implementation Plan (v0.3.0)

> **For Claude:** REQUIRED SUB-SKILL: Use superpowers:executing-plans to implement this plan task-by-task.

**Goal:** Fix three post-release issues: backup suffix regression, housekeep UX problems, and missing write subcommand. Roll version back to v0.3.0 to signal pre-release status.

**Architecture:** Five phases of incremental changes with frequent commits. Each phase is independently testable and builds on the previous. Use TDD where practical. Maintain backward compatibility in dispatch layer while restructuring CLI.

**Tech Stack:** Rust, Clap 4.x, proc macros for custom attributes, workspace for macro crate

---

## Phase 1: Fix Backup Suffix Regression

### Task 1: Add Test for Custom Backup Suffix

**Files:**
- Test: `tests/backup_suffix_test.rs`

**Step 1: Write test for custom suffix without timestamp**

Create `tests/backup_suffix_test.rs`:

```rust
use mutx::backup::{create_backup, BackupConfig};
use std::fs;
use tempfile::TempDir;

#[test]
fn test_custom_suffix_without_timestamp() {
    let dir = TempDir::new().unwrap();
    let target = dir.path().join("test.txt");
    fs::write(&target, "original content").unwrap();

    let config = BackupConfig {
        source: target.clone(),
        suffix: ".bak".to_string(),
        directory: None,
        timestamp: false,
    };

    let backup_path = create_backup(&config).unwrap();

    assert_eq!(
        backup_path.file_name().unwrap().to_str().unwrap(),
        "test.txt.bak"
    );
    assert!(backup_path.exists());
    assert_eq!(
        fs::read_to_string(&backup_path).unwrap(),
        "original content"
    );
}
```

**Step 2: Run test to verify it fails**

Run: `cargo test test_custom_suffix_without_timestamp`
Expected: FAIL - creates "test.txt.mutx.backup" instead of "test.txt.bak"

**Step 3: Add test for custom suffix with timestamp**

Add to `tests/backup_suffix_test.rs`:

```rust
#[test]
fn test_custom_suffix_with_timestamp() {
    let dir = TempDir::new().unwrap();
    let target = dir.path().join("test.txt");
    fs::write(&target, "original").unwrap();

    let config = BackupConfig {
        source: target.clone(),
        suffix: ".deploy.backup".to_string(),
        directory: None,
        timestamp: true,
    };

    let backup_path = create_backup(&config).unwrap();
    let filename = backup_path.file_name().unwrap().to_str().unwrap();

    // Should be: test.txt.YYYYMMDD_HHMMSS.deploy.backup
    assert!(filename.starts_with("test.txt."));
    assert!(filename.ends_with(".deploy.backup"));
    assert!(filename.contains('_')); // Timestamp separator
    assert_eq!(fs::read_to_string(&backup_path).unwrap(), "original");
}

#[test]
fn test_default_suffix_still_works() {
    let dir = TempDir::new().unwrap();
    let target = dir.path().join("test.txt");
    fs::write(&target, "content").unwrap();

    let config = BackupConfig {
        source: target.clone(),
        suffix: ".mutx.backup".to_string(),
        directory: None,
        timestamp: false,
    };

    let backup_path = create_backup(&config).unwrap();

    assert_eq!(
        backup_path.file_name().unwrap().to_str().unwrap(),
        "test.txt.mutx.backup"
    );
}
```

**Step 4: Run tests to verify they fail**

Run: `cargo test backup_suffix`
Expected: All tests FAIL - suffix hardcoded to ".mutx.backup"

**Step 5: Update generate_backup_path to use config.suffix**

Modify `src/backup.rs:75-80`:

```rust
let backup_name = if config.timestamp {
    let timestamp = Local::now().format("%Y%m%d_%H%M%S");
    format!("{}.{}{}", filename, timestamp, config.suffix)
} else {
    format!("{}{}", filename, config.suffix)
};
```

**Step 6: Run tests to verify they pass**

Run: `cargo test backup_suffix`
Expected: All tests PASS

**Step 7: Run full backup test suite**

Run: `cargo test backup`
Expected: All backup tests PASS

**Step 8: Commit backup suffix fix**

```bash
git add tests/backup_suffix_test.rs src/backup.rs
git commit -m "fix: make --backup-suffix functional

The --backup-suffix flag was being accepted but ignored. Backups always
used .mutx.backup regardless of the provided suffix.

Now config.suffix is used in backup name generation:
- Without timestamp: {filename}{suffix}
- With timestamp: {filename}.{timestamp}{suffix}

This allows users to specify custom suffixes like .bak, .deploy.backup, etc.

Co-Authored-By: Claude Opus 4.5 <noreply@anthropic.com>"
```

---

## Phase 2: Housekeep Custom Suffix Support

### Task 2: Add Suffix Field to CleanBackupConfig

**Files:**
- Modify: `src/housekeep.rs:17-23`

**Step 1: Add suffix field to CleanBackupConfig**

Modify `src/housekeep.rs:17-23`:

```rust
pub struct CleanBackupConfig {
    pub dir: PathBuf,
    pub recursive: bool,
    pub older_than: Option<Duration>,
    pub keep_newest: Option<usize>,
    pub dry_run: bool,
    pub suffix: String,  // NEW
}
```

**Step 2: Update clean_backups function signature**

The function already takes `&CleanBackupConfig`, so no signature change needed.
Just update the implementation to use `config.suffix`.

**Step 3: Update is_backup_file to accept suffix parameter**

Modify `src/housekeep.rs:176-181`:

```rust
fn is_backup_file(path: &Path, suffix: &str) -> bool {
    path.file_name()
        .and_then(|s| s.to_str())
        .map(|name| name.ends_with(suffix))
        .unwrap_or(false)
}
```

**Step 4: Update extract_base_filename to accept suffix parameter**

Modify `src/housekeep.rs:183-211`:

```rust
fn extract_base_filename(path: &Path, suffix: &str) -> String {
    let name = path
        .file_name()
        .and_then(|n| n.to_str())
        .unwrap_or("unknown");

    // Must end with the suffix
    let without_suffix = match name.strip_suffix(suffix) {
        Some(s) => s,
        None => return name.to_string(),
    };

    // Try to parse timestamp: filename.YYYYMMDD_HHMMSS
    let parts: Vec<&str> = without_suffix.rsplitn(2, '.').collect();
    if parts.len() == 2 {
        let timestamp = parts[0];
        if is_valid_timestamp(timestamp) {
            return parts[1].to_string();  // Base filename without timestamp
        }
    }

    // No timestamp found, return without suffix
    without_suffix.to_string()
}
```

**Step 5: Update clean_backups to pass suffix to helper functions**

Modify `src/housekeep.rs:69-120` - update calls:

```rust
pub fn clean_backups(config: &CleanBackupConfig) -> Result<Vec<PathBuf>> {
    let mut all_backups: HashMap<String, Vec<BackupFile>> = HashMap::new();

    visit_directory(&config.dir, config.recursive, &mut |path| {
        if is_backup_file(path, &config.suffix) {  // Pass suffix
            match get_modification_time(path) {
                Ok(modified) => {
                    let base = extract_base_filename(path, &config.suffix);  // Pass suffix
                    all_backups
                        .entry(base)
                        .or_insert_with(Vec::new)
                        .push(BackupFile {
                            path: path.to_path_buf(),
                            modified,
                        });
                }
                Err(e) => {
                    debug!("Skipping {}: {:?}", path.display(), e);
                }
            }
        }
        Ok(())
    })?;

    // ... rest of function unchanged
}
```

**Step 6: Update housekeep_command.rs to pass suffix**

Modify `src/cli/housekeep_command.rs:68-74`:

```rust
let config = CleanBackupConfig {
    dir: target_dir.clone(),
    recursive,
    older_than: duration,
    keep_newest,
    dry_run,
    suffix: ".mutx.backup".to_string(),  // NEW: default suffix for now
};
```

**Step 7: Compile to check for errors**

Run: `cargo build`
Expected: Compilation succeeds

**Step 8: Run existing housekeep tests**

Run: `cargo test housekeep`
Expected: All tests PASS (using default .mutx.backup suffix)

**Step 9: Write test for custom suffix cleanup**

Add to `tests/housekeep_test.rs`:

```rust
#[test]
fn test_cleans_custom_suffix_backups() {
    let dir = TempDir::new().unwrap();

    // Create backups with custom suffix
    fs::write(dir.path().join("file.txt.bak"), "backup1").unwrap();
    fs::write(
        dir.path().join("file.txt.20260126_120000.bak"),
        "backup2",
    )
    .unwrap();

    // Should not touch .mutx.backup files
    fs::write(dir.path().join("other.txt.mutx.backup"), "keep").unwrap();

    let config = CleanBackupConfig {
        dir: dir.path().to_path_buf(),
        recursive: false,
        older_than: None,
        keep_newest: Some(1),
        dry_run: false,
        suffix: ".bak".to_string(),
    };

    let cleaned = clean_backups(&config).unwrap();

    // Should clean one .bak file (keeping newest)
    assert_eq!(cleaned.len(), 1);

    // .mutx.backup file should still exist
    assert!(dir.path().join("other.txt.mutx.backup").exists());
}
```

**Step 10: Run test to verify it passes**

Run: `cargo test test_cleans_custom_suffix_backups`
Expected: PASS

**Step 11: Commit housekeep suffix support**

```bash
git add src/housekeep.rs src/cli/housekeep_command.rs tests/housekeep_test.rs
git commit -m "feat(housekeep): support custom backup suffixes

Add suffix parameter to CleanBackupConfig to support cleaning backups
with custom suffixes.

Changes:
- Add suffix field to CleanBackupConfig
- Update is_backup_file() to accept suffix parameter
- Update extract_base_filename() for generic suffix stripping
- Default to .mutx.backup for backward compatibility

This enables future CLI flag: --suffix for housekeep backups command.

Co-Authored-By: Claude Opus 4.5 <noreply@anthropic.com>"
```

---

## Phase 3: Proc Macro and Write Subcommand

### Task 3: Create mutx-macros Crate

**Files:**
- Create: `mutx-macros/Cargo.toml`
- Create: `mutx-macros/src/lib.rs`
- Modify: `Cargo.toml` (workspace config)

**Step 1: Create workspace configuration**

Modify root `Cargo.toml` - add before `[package]`:

```toml
[workspace]
members = [".", "mutx-macros"]
```

**Step 2: Create mutx-macros directory**

Run: `mkdir mutx-macros`

**Step 3: Create mutx-macros Cargo.toml**

Create `mutx-macros/Cargo.toml`:

```toml
[package]
name = "mutx-macros"
version = "0.1.0"
edition = "2021"

[lib]
proc-macro = true

[dependencies]
proc-macro2 = "1.0"
quote = "1.0"
syn = { version = "2.0", features = ["full"] }
```

**Step 4: Create mutx-macros src directory**

Run: `mkdir mutx-macros/src`

**Step 5: Create simple marker macro**

Create `mutx-macros/src/lib.rs`:

```rust
use proc_macro::TokenStream;

/// Marks a subcommand as the implicit default when no subcommand is specified.
///
/// This is a marker attribute - the actual dispatch logic is handled in
/// the CLI module's run() function.
#[proc_macro_attribute]
pub fn implicit_command(_attr: TokenStream, item: TokenStream) -> TokenStream {
    // Pass through unchanged - this is just a semantic marker
    item
}
```

**Step 6: Add mutx-macros dependency to main Cargo.toml**

Modify root `Cargo.toml` dependencies:

```toml
mutx-macros = { path = "mutx-macros" }
```

**Step 7: Build workspace to verify**

Run: `cargo build`
Expected: Builds successfully, including mutx-macros crate

**Step 8: Commit macro crate**

```bash
git add Cargo.toml mutx-macros/
git commit -m "feat: add mutx-macros crate for custom attributes

Create proc macro crate for #[implicit_command] attribute.
This is a marker attribute used to designate the default subcommand
when none is specified.

The macro itself is a simple passthrough - dispatch logic is in cli/mod.rs.

Co-Authored-By: Claude Opus 4.5 <noreply@anthropic.com>"
```

### Task 4: Add Write Subcommand to CLI

**Files:**
- Modify: `src/cli/args.rs:78-118`
- Modify: `src/cli/mod.rs:8-13`
- Modify: `src/cli/write_command.rs`

**Step 1: Import implicit_command macro**

Add to `src/cli/args.rs` at top:

```rust
use mutx_macros::implicit_command;
```

**Step 2: Add Write variant to Command enum**

Modify `src/cli/args.rs:78-118` - add Write variant BEFORE Housekeep:

```rust
#[derive(Subcommand, Debug)]
pub enum Command {
    /// Write to file atomically (default when no subcommand specified)
    #[implicit_command]
    Write {
        /// Target file path
        #[arg(value_name = "OUTPUT")]
        output: PathBuf,

        /// Read from file instead of stdin
        #[arg(short, long, value_name = "FILE")]
        input: Option<PathBuf>,

        /// Use streaming mode (constant memory)
        #[arg(long)]
        stream: bool,

        /// Fail immediately if locked (default: wait)
        #[arg(long)]
        no_wait: bool,

        /// Wait timeout in milliseconds (implies wait mode)
        #[arg(short = 't', long, value_name = "MILLISECONDS", conflicts_with = "no_wait")]
        timeout: Option<u64>,

        /// Maximum polling interval in milliseconds (default: 1000)
        #[arg(long, value_name = "MILLISECONDS", requires = "timeout")]
        max_poll_interval: Option<u64>,

        /// Custom lock file location
        #[arg(long, value_name = "PATH")]
        lock_file: Option<PathBuf>,

        /// Follow symbolic links for output files
        #[arg(long)]
        follow_symlinks: bool,

        /// Follow symbolic links even for lock files (implies --follow-symlinks)
        #[arg(long)]
        follow_lock_symlinks: bool,

        /// Create backup before overwrite
        #[arg(short = 'b', long)]
        backup: bool,

        /// Backup filename suffix
        #[arg(long, value_name = "SUFFIX", default_value = ".mutx.backup", requires = "backup")]
        backup_suffix: String,

        /// Store backups in directory
        #[arg(long, value_name = "DIR", requires = "backup")]
        backup_dir: Option<PathBuf>,

        /// Add timestamp to backup filename
        #[arg(long, requires = "backup")]
        backup_timestamp: bool,

        /// Verbose output
        #[arg(short = 'v', action = clap::ArgAction::Count)]
        verbose: u8,
    },

    /// Housekeeping operations
    Housekeep {
        // ... existing housekeep fields ...
    },
}
```

**Step 3: Update write_command to handle both forms**

Add new function to `src/cli/write_command.rs` at the end:

```rust
/// Execute write command from explicit Write subcommand
pub fn execute_write_from_command(
    output: PathBuf,
    input: Option<PathBuf>,
    stream: bool,
    no_wait: bool,
    timeout: Option<u64>,
    max_poll_interval: Option<u64>,
    lock_file: Option<PathBuf>,
    follow_symlinks: bool,
    follow_lock_symlinks: bool,
    backup: bool,
    backup_suffix: String,
    backup_dir: Option<PathBuf>,
    backup_timestamp: bool,
    verbose: u8,
) -> Result<()> {
    // Convert to Args structure for compatibility
    let args = crate::cli::Args {
        command: None,
        output: Some(output),
        input,
        stream,
        no_wait,
        timeout,
        max_poll_interval,
        lock_file,
        follow_symlinks,
        follow_lock_symlinks,
        backup,
        backup_suffix,
        backup_dir,
        backup_timestamp,
        verbose,
    };

    execute_write(args)
}
```

**Step 4: Update cli/mod.rs dispatch**

Modify `src/cli/mod.rs:8-13`:

```rust
pub fn run(args: Args) -> Result<()> {
    match args.command {
        Some(Command::Write {
            output,
            input,
            stream,
            no_wait,
            timeout,
            max_poll_interval,
            lock_file,
            follow_symlinks,
            follow_lock_symlinks,
            backup,
            backup_suffix,
            backup_dir,
            backup_timestamp,
            verbose,
        }) => write_command::execute_write_from_command(
            output,
            input,
            stream,
            no_wait,
            timeout,
            max_poll_interval,
            lock_file,
            follow_symlinks,
            follow_lock_symlinks,
            backup,
            backup_suffix,
            backup_dir,
            backup_timestamp,
            verbose,
        ),
        Some(cmd @ Command::Housekeep { .. }) => housekeep_command::execute_housekeep(cmd),
        None => write_command::execute_write(args),
    }
}
```

**Step 5: Test implicit write (no subcommand)**

Run: `echo "test" | cargo run -- /tmp/test-implicit.txt`
Expected: File written successfully

**Step 6: Test explicit write subcommand**

Run: `echo "test" | cargo run -- write /tmp/test-explicit.txt`
Expected: File written successfully

**Step 7: Add CLI test for both forms**

Create `tests/write_subcommand_test.rs`:

```rust
use assert_cmd::Command;
use std::fs;
use tempfile::TempDir;

#[test]
fn test_implicit_write_creates_file() {
    let dir = TempDir::new().unwrap();
    let output = dir.path().join("output.txt");

    let mut cmd = Command::cargo_bin("mutx").unwrap();
    cmd.arg(output.to_str().unwrap())
        .write_stdin("implicit content")
        .assert()
        .success();

    assert_eq!(fs::read_to_string(&output).unwrap(), "implicit content");
}

#[test]
fn test_explicit_write_creates_file() {
    let dir = TempDir::new().unwrap();
    let output = dir.path().join("output.txt");

    let mut cmd = Command::cargo_bin("mutx").unwrap();
    cmd.arg("write")
        .arg(output.to_str().unwrap())
        .write_stdin("explicit content")
        .assert()
        .success();

    assert_eq!(fs::read_to_string(&output).unwrap(), "explicit content");
}

#[test]
fn test_both_forms_produce_same_result() {
    let dir = TempDir::new().unwrap();
    let implicit = dir.path().join("implicit.txt");
    let explicit = dir.path().join("explicit.txt");

    // Implicit
    Command::cargo_bin("mutx")
        .unwrap()
        .arg(implicit.to_str().unwrap())
        .write_stdin("same content")
        .assert()
        .success();

    // Explicit
    Command::cargo_bin("mutx")
        .unwrap()
        .arg("write")
        .arg(explicit.to_str().unwrap())
        .write_stdin("same content")
        .assert()
        .success();

    assert_eq!(
        fs::read_to_string(&implicit).unwrap(),
        fs::read_to_string(&explicit).unwrap()
    );
}
```

**Step 8: Run new tests**

Run: `cargo test write_subcommand`
Expected: All tests PASS

**Step 9: Run full test suite**

Run: `cargo test`
Expected: All tests PASS

**Step 10: Commit write subcommand**

```bash
git add src/cli/args.rs src/cli/mod.rs src/cli/write_command.rs tests/write_subcommand_test.rs
git commit -m "feat: add explicit write subcommand with implicit fallback

Add Write subcommand marked with #[implicit_command] to support both:
- mutx output.txt (implicit, backward compatible)
- mutx write output.txt (explicit, clearer intent)

The macro is a semantic marker. Dispatch logic in cli/mod.rs handles
both forms by converting explicit Write arguments to Args structure.

Both forms produce identical behavior.

Co-Authored-By: Claude Opus 4.5 <noreply@anthropic.com>"
```

---

## Phase 4: Restructure Housekeep as Nested Subcommands

### Task 5: Create HousekeepOperation Enum

**Files:**
- Modify: `src/cli/args.rs:78-118`
- Create: `tests/housekeep_subcommand_test.rs`

**Step 1: Create HousekeepOperation enum**

Add to `src/cli/args.rs` after Command enum:

```rust
#[derive(Subcommand, Debug)]
pub enum HousekeepOperation {
    /// Clean orphaned lock files from cache directory
    Locks {
        /// Directory to clean (default: platform lock cache directory)
        #[arg(value_name = "DIR")]
        dir: Option<PathBuf>,

        #[arg(short = 'r', long)]
        recursive: bool,

        #[arg(long, value_name = "DURATION")]
        older_than: Option<String>,

        #[arg(short = 'n', long)]
        dry_run: bool,

        #[arg(short = 'v', long)]
        verbose: bool,
    },

    /// Clean old backup files
    Backups {
        /// Directory to clean (default: current directory)
        #[arg(value_name = "DIR")]
        dir: Option<PathBuf>,

        #[arg(short = 'r', long)]
        recursive: bool,

        #[arg(long, value_name = "DURATION")]
        older_than: Option<String>,

        #[arg(long, value_name = "N")]
        keep_newest: Option<usize>,

        /// Backup suffix to match (default: .mutx.backup)
        #[arg(long, value_name = "SUFFIX", default_value = ".mutx.backup")]
        suffix: String,

        #[arg(short = 'n', long)]
        dry_run: bool,

        #[arg(short = 'v', long)]
        verbose: bool,
    },

    /// Clean both locks and backups
    All {
        /// Directory to clean (used for both locks and backups)
        #[arg(value_name = "DIR", conflicts_with_all = ["locks_dir", "backups_dir"])]
        dir: Option<PathBuf>,

        /// Directory for lock files (requires --backups-dir)
        #[arg(long, value_name = "DIR", requires = "backups_dir")]
        locks_dir: Option<PathBuf>,

        /// Directory for backup files (requires --locks-dir)
        #[arg(long, value_name = "DIR", requires = "locks_dir")]
        backups_dir: Option<PathBuf>,

        #[arg(short = 'r', long)]
        recursive: bool,

        #[arg(long, value_name = "DURATION")]
        older_than: Option<String>,

        #[arg(long, value_name = "N")]
        keep_newest: Option<usize>,

        /// Backup suffix to match (default: .mutx.backup)
        #[arg(long, value_name = "SUFFIX", default_value = ".mutx.backup")]
        suffix: String,

        #[arg(short = 'n', long)]
        dry_run: bool,

        #[arg(short = 'v', long)]
        verbose: bool,
    },
}
```

**Step 2: Update Command::Housekeep to nest operations**

Replace the Housekeep variant in Command enum:

```rust
/// Housekeeping operations
#[command(subcommand_required = true)]
Housekeep {
    #[command(subcommand)]
    operation: HousekeepOperation,
},
```

**Step 3: Export HousekeepOperation**

Modify `src/cli/mod.rs:5`:

```rust
pub use args::{Args, Command, HousekeepOperation};
```

**Step 4: Compile to check for errors**

Run: `cargo build`
Expected: Build fails - housekeep_command.rs needs updating

### Task 6: Update Housekeep Command Handler

**Files:**
- Modify: `src/cli/housekeep_command.rs`
- Modify: `src/cli/mod.rs:8-13`

**Step 1: Update execute_housekeep signature**

Modify `src/cli/housekeep_command.rs:7`:

```rust
pub fn execute_housekeep(operation: crate::cli::HousekeepOperation) -> Result<()> {
    use crate::cli::HousekeepOperation;

    match operation {
        HousekeepOperation::Locks {
            dir,
            recursive,
            older_than,
            dry_run,
            verbose,
        } => execute_clean_locks(dir, recursive, older_than, dry_run, verbose),

        HousekeepOperation::Backups {
            dir,
            recursive,
            older_than,
            keep_newest,
            suffix,
            dry_run,
            verbose,
        } => execute_clean_backups(dir, recursive, older_than, keep_newest, suffix, dry_run, verbose),

        HousekeepOperation::All {
            dir,
            locks_dir,
            backups_dir,
            recursive,
            older_than,
            keep_newest,
            suffix,
            dry_run,
            verbose,
        } => execute_clean_all(
            dir,
            locks_dir,
            backups_dir,
            recursive,
            older_than,
            keep_newest,
            suffix,
            dry_run,
            verbose,
        ),
    }
}
```

**Step 2: Implement execute_clean_locks with smart default**

Add to `src/cli/housekeep_command.rs`:

```rust
use mutx::lock::get_lock_cache_dir;

fn execute_clean_locks(
    dir: Option<PathBuf>,
    recursive: bool,
    older_than: Option<String>,
    dry_run: bool,
    verbose: bool,
) -> Result<()> {
    // Default to lock cache directory
    let target_dir = if let Some(d) = dir {
        d
    } else {
        get_lock_cache_dir()?
    };

    let duration = match older_than {
        Some(s) => Some(parse_duration(&s)?),
        None => None,
    };

    let config = CleanLockConfig {
        dir: target_dir,
        recursive,
        older_than: duration,
        dry_run,
    };

    let cleaned = clean_locks(&config)?;

    for path in &cleaned {
        println!(
            "{}{}",
            if dry_run { "[DRY RUN] Would delete: " } else { "Deleted: " },
            path.display()
        );
    }

    if verbose || dry_run {
        eprintln!("Cleaned {} lock file(s)", cleaned.len());
    }

    Ok(())
}
```

**Step 3: Implement execute_clean_backups with suffix support**

Add to `src/cli/housekeep_command.rs`:

```rust
fn execute_clean_backups(
    dir: Option<PathBuf>,
    recursive: bool,
    older_than: Option<String>,
    keep_newest: Option<usize>,
    suffix: String,
    dry_run: bool,
    verbose: bool,
) -> Result<()> {
    // Default to current directory
    let target_dir = dir.unwrap_or_else(|| PathBuf::from("."));

    let duration = match older_than {
        Some(s) => Some(parse_duration(&s)?),
        None => None,
    };

    let config = CleanBackupConfig {
        dir: target_dir,
        recursive,
        older_than: duration,
        keep_newest,
        dry_run,
        suffix,
    };

    let cleaned = clean_backups(&config)?;

    for path in &cleaned {
        println!(
            "{}{}",
            if dry_run { "[DRY RUN] Would delete: " } else { "Deleted: " },
            path.display()
        );
    }

    if verbose || dry_run {
        eprintln!("Cleaned {} backup file(s)", cleaned.len());
    }

    Ok(())
}
```

**Step 4: Implement execute_clean_all with validation**

Add to `src/cli/housekeep_command.rs`:

```rust
#[allow(clippy::too_many_arguments)]
fn execute_clean_all(
    dir: Option<PathBuf>,
    locks_dir: Option<PathBuf>,
    backups_dir: Option<PathBuf>,
    recursive: bool,
    older_than: Option<String>,
    keep_newest: Option<usize>,
    suffix: String,
    dry_run: bool,
    verbose: bool,
) -> Result<()> {
    // Validate: either dir OR both locks_dir and backups_dir
    let (lock_target, backup_target) = match (dir, locks_dir, backups_dir) {
        (Some(d), None, None) => (d.clone(), d),
        (None, Some(ld), Some(bd)) => (ld, bd),
        _ => {
            return Err(MutxError::Other(
                "Specify either [DIR] or both --locks-dir and --backups-dir".to_string(),
            ))
        }
    };

    let mut total_cleaned = 0;

    // Clean locks
    let lock_result = execute_clean_locks(Some(lock_target), recursive, older_than.clone(), dry_run, false);
    match lock_result {
        Ok(()) => {
            if verbose {
                eprintln!("Locks cleaned");
            }
        }
        Err(e) => eprintln!("Warning: lock cleanup failed: {}", e),
    }

    // Clean backups
    let backup_result = execute_clean_backups(
        Some(backup_target),
        recursive,
        older_than,
        keep_newest,
        suffix,
        dry_run,
        false,
    );
    match backup_result {
        Ok(()) => {
            if verbose {
                eprintln!("Backups cleaned");
            }
        }
        Err(e) => eprintln!("Warning: backup cleanup failed: {}", e),
    }

    if verbose {
        eprintln!("All housekeeping complete");
    }

    Ok(())
}
```

**Step 5: Update cli/mod.rs dispatch**

Modify `src/cli/mod.rs`:

```rust
pub fn run(args: Args) -> Result<()> {
    match args.command {
        Some(Command::Write { /* ... */ }) => { /* ... */ },
        Some(Command::Housekeep { operation }) => {
            housekeep_command::execute_housekeep(operation)
        }
        None => write_command::execute_write(args),
    }
}
```

**Step 6: Compile to verify**

Run: `cargo build`
Expected: Build succeeds

**Step 7: Test locks subcommand with default directory**

Run: `cargo run -- housekeep locks --dry-run -v`
Expected: Shows lock cache directory path, lists lock files

**Step 8: Test backups subcommand with custom suffix**

```bash
# Create test backups
mkdir /tmp/test-backups
echo "test" > /tmp/test-backups/file.txt.bak
echo "test" > /tmp/test-backups/file.txt.20260126_120000.bak

# Test cleanup
cargo run -- housekeep backups --suffix .bak --dry-run /tmp/test-backups
```

Expected: Lists .bak files for cleanup

**Step 9: Test all subcommand with single directory**

Run: `cargo run -- housekeep all --dry-run /tmp/test-dir`
Expected: Cleans both locks and backups from /tmp/test-dir

**Step 10: Test all subcommand with separate directories**

Run: `cargo run -- housekeep all --locks-dir ~/.cache/mutx/locks --backups-dir /tmp --dry-run`
Expected: Cleans locks from cache, backups from /tmp

**Step 11: Test validation error**

Run: `cargo run -- housekeep all`
Expected: Error: "Specify either [DIR] or both --locks-dir and --backups-dir"

### Task 7: Update Housekeep Tests

**Files:**
- Modify: `tests/cli_housekeep_test.rs`
- Create: `tests/housekeep_subcommand_test.rs`

**Step 1: Update existing CLI housekeep tests**

Modify `tests/cli_housekeep_test.rs` - update all command syntax:

```rust
// OLD: mutx housekeep --clean-locks
// NEW: mutx housekeep locks

#[test]
fn test_housekeep_clean_locks() {
    let dir = TempDir::new().unwrap();
    let lock = dir.path().join("file.lock");
    File::create(&lock).unwrap();

    let mut cmd = Command::cargo_bin("mutx").unwrap();
    cmd.arg("housekeep")
        .arg("locks")  // Changed from --clean-locks
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

    let mut cmd = Command::cargo_bin("mutx").unwrap();
    cmd.arg("housekeep")
        .arg("locks")  // Changed
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

    fs::write(dir.path().join("file.txt.mutx.backup"), "backup1").unwrap();
    fs::write(
        dir.path().join("file.txt.20260125_120000.mutx.backup"),
        "backup2",
    )
    .unwrap();

    let mut cmd = Command::cargo_bin("mutx").unwrap();
    cmd.arg("housekeep")
        .arg("backups")  // Changed from --clean-backups
        .arg("--keep-newest")
        .arg("1")
        .arg(dir.path())
        .assert()
        .success();

    let backups: Vec<_> = fs::read_dir(dir.path())
        .unwrap()
        .filter_map(|e| e.ok())
        .filter(|e| e.path().to_str().unwrap().contains(".mutx.backup"))
        .collect();

    assert_eq!(backups.len(), 1);
}

#[test]
fn test_housekeep_requires_subcommand() {
    let dir = TempDir::new().unwrap();

    let mut cmd = Command::cargo_bin("mutx").unwrap();
    cmd.arg("housekeep")
        .arg(dir.path())
        .assert()
        .failure()
        .stderr(predicate::str::contains("required"));
}
```

**Step 2: Create comprehensive subcommand tests**

Create `tests/housekeep_subcommand_test.rs`:

```rust
use assert_cmd::Command;
use std::fs::{self, File};
use tempfile::TempDir;
use predicates::prelude::*;

#[test]
fn test_locks_defaults_to_cache_directory() {
    // This test verifies the default but doesn't actually clean anything
    let mut cmd = Command::cargo_bin("mutx").unwrap();
    cmd.arg("housekeep")
        .arg("locks")
        .arg("--dry-run")
        .arg("-v")
        .assert()
        .success();

    // Output should mention cache directory
    // Actual path varies by platform, so just verify it runs
}

#[test]
fn test_backups_defaults_to_current_directory() {
    let mut cmd = Command::cargo_bin("mutx").unwrap();
    cmd.arg("housekeep")
        .arg("backups")
        .arg("--dry-run")
        .arg("-v")
        .assert()
        .success();
}

#[test]
fn test_backups_custom_suffix() {
    let dir = TempDir::new().unwrap();

    fs::write(dir.path().join("file.txt.bak"), "backup").unwrap();
    fs::write(dir.path().join("file.txt.mutx.backup"), "keep").unwrap();

    let mut cmd = Command::cargo_bin("mutx").unwrap();
    cmd.arg("housekeep")
        .arg("backups")
        .arg("--suffix")
        .arg(".bak")
        .arg(dir.path())
        .assert()
        .success();

    // .bak should be deleted
    assert!(!dir.path().join("file.txt.bak").exists());
    // .mutx.backup should remain
    assert!(dir.path().join("file.txt.mutx.backup").exists());
}

#[test]
fn test_all_with_single_directory() {
    let dir = TempDir::new().unwrap();

    fs::write(dir.path().join("test.lock"), "").unwrap();
    fs::write(dir.path().join("file.txt.mutx.backup"), "backup").unwrap();

    let mut cmd = Command::cargo_bin("mutx").unwrap();
    cmd.arg("housekeep")
        .arg("all")
        .arg(dir.path())
        .assert()
        .success();

    assert!(!dir.path().join("test.lock").exists());
    assert!(!dir.path().join("file.txt.mutx.backup").exists());
}

#[test]
fn test_all_with_separate_directories() {
    let locks_dir = TempDir::new().unwrap();
    let backups_dir = TempDir::new().unwrap();

    fs::write(locks_dir.path().join("test.lock"), "").unwrap();
    fs::write(backups_dir.path().join("file.txt.mutx.backup"), "backup").unwrap();

    let mut cmd = Command::cargo_bin("mutx").unwrap();
    cmd.arg("housekeep")
        .arg("all")
        .arg("--locks-dir")
        .arg(locks_dir.path())
        .arg("--backups-dir")
        .arg(backups_dir.path())
        .assert()
        .success();

    assert!(!locks_dir.path().join("test.lock").exists());
    assert!(!backups_dir.path().join("file.txt.mutx.backup").exists());
}

#[test]
fn test_all_validation_error_no_args() {
    let mut cmd = Command::cargo_bin("mutx").unwrap();
    cmd.arg("housekeep")
        .arg("all")
        .assert()
        .failure()
        .stderr(predicate::str::contains("Specify either"));
}

#[test]
fn test_all_validation_error_mixed_args() {
    let dir = TempDir::new().unwrap();

    let mut cmd = Command::cargo_bin("mutx").unwrap();
    cmd.arg("housekeep")
        .arg("all")
        .arg(dir.path())
        .arg("--locks-dir")
        .arg(dir.path())
        .assert()
        .failure();
}
```

**Step 3: Run all housekeep tests**

Run: `cargo test housekeep`
Expected: All tests PASS

**Step 4: Run full test suite**

Run: `cargo test`
Expected: All tests PASS

**Step 5: Commit housekeep restructure**

```bash
git add src/cli/args.rs src/cli/mod.rs src/cli/housekeep_command.rs tests/cli_housekeep_test.rs tests/housekeep_subcommand_test.rs
git commit -m "feat!: restructure housekeep as nested subcommands

BREAKING CHANGE: Housekeep now uses subcommands instead of flags

Old syntax:
- mutx housekeep --clean-locks [DIR]
- mutx housekeep --clean-backups [DIR]
- mutx housekeep --all [DIR]

New syntax:
- mutx housekeep locks [DIR]       (defaults to cache directory)
- mutx housekeep backups [DIR]     (defaults to current directory)
- mutx housekeep all [DIR]         (or --locks-dir + --backups-dir)

Features:
- Smart defaults: locks → cache, backups → current directory
- Custom suffix support: --suffix flag for backups command
- Separate directory control for 'all' command

Co-Authored-By: Claude Opus 4.5 <noreply@anthropic.com>"
```

---

## Phase 5: Version Rollback and Documentation

### Task 8: Roll Version Back to 0.3.0

**Files:**
- Modify: `Cargo.toml`
- Modify: `mutx-macros/Cargo.toml`
- Modify: `CHANGELOG.md`
- Modify: `tests/cli_args_test.rs`

**Step 1: Update version in Cargo.toml**

Modify `Cargo.toml`:

```toml
[package]
version = "0.3.0"
```

**Step 2: Update macro crate version**

Modify `mutx-macros/Cargo.toml`:

```toml
[package]
version = "0.1.0"  # Keep at 0.1.0 (internal crate)
```

**Step 3: Rewrite CHANGELOG.md**

Replace entire file with:

```markdown
# Changelog

All notable changes to this project will be documented in this file.

The format is based on [Keep a Changelog](https://keepachangelog.com/en/1.0.0/),
and this project adheres to [Semantic Versioning](https://semver.org/spec/v2.0.0.html).

## [Unreleased]

## [0.3.0] - 2026-01-26

**Note:** Version numbers rolled back from v1.1.0 to v0.3.0 to better signal
pre-release status. Version 1.0.0 is reserved for the first stable public release.

### Added
- Explicit `mutx write` subcommand (implicit `mutx output.txt` still works)
- Housekeep nested subcommands: `locks`, `backups`, `all`
- `--suffix` flag for `housekeep backups` to clean custom backup patterns
- `--locks-dir` and `--backups-dir` flags for `housekeep all`
- Smart defaults: `housekeep locks` defaults to cache directory
- Proc macro crate `mutx-macros` with `#[implicit_command]` attribute

### Fixed
- **CRITICAL:** `--backup-suffix` now functional (was silently ignored)
- Housekeep locks defaults to cache directory instead of current directory
- README examples corrected to match actual CLI syntax

### Changed
- **BREAKING:** Housekeep now uses subcommands instead of flags:
  - `mutx housekeep --clean-locks` → `mutx housekeep locks`
  - `mutx housekeep --clean-backups` → `mutx housekeep backups`
  - `mutx housekeep --all` → `mutx housekeep all [DIR]`
- Version numbering strategy: v0.x for pre-release, v1.0.0+ for stable

### Security (from previous v1.1.0 release)
- Lock files stored in platform cache directories to prevent collision attacks
- Symlink rejection by default with opt-in `--follow-symlinks` flags
- Backup format uses strict validation to prevent user file deletion
- Exponential backoff with jitter to prevent timing attacks in lock acquisition

### Features (from previous v1.1.0 release)
- **Lock Storage**: Platform-specific cache directories
  - Linux: `~/.cache/mutx/locks/`
  - macOS: `~/Library/Caches/mutx/locks/`
  - Windows: `%LOCALAPPDATA%\mutx\locks\`
- **Lock Naming**: Collision-resistant format with SHA256 hash
- **Timeout Backoff**: Exponential backoff (1.5x multiplier) with random jitter (0-100ms)
- **Symlink Protection**: Reject symlinks by default for output and lock files
- **Backup Format**: Collision-resistant timestamped format `{file}.{YYYYMMDD_HHMMSS}.mutx.backup`

### Technical Details (from previous v1.1.0 release)
- Added dependencies: `directories`, `rand`, `sha2`
- Lock files persist after release (not deleted on Drop) for proper mutual exclusion
- Housekeep cleanup handles symlinks safely without following them
- Custom lock paths can be specified with `--lock-file` flag
- Maximum poll interval configurable with `--max-poll-interval` (default: 1000ms)

## [0.2.0] - 2026-01-24

### Added
- Initial atomic file write implementation
- File locking with advisory locks
- Backup support with optional timestamps
- Streaming mode for large files
- Housekeeping command for cleanup

### Features
- Atomic writes using write-to-temp + rename strategy
- Advisory file locking to prevent concurrent writes
- Optional backup creation before overwrite
- Streaming mode with constant memory usage
- Lock timeout and no-wait modes

[Unreleased]: https://github.com/aaronbassett/mutx/compare/v0.3.0...HEAD
[0.3.0]: https://github.com/aaronbassett/mutx/compare/v0.2.0...v0.3.0
[0.2.0]: https://github.com/aaronbassett/mutx/releases/tag/v0.2.0
```

**Step 4: Update version test**

Modify `tests/cli_args_test.rs:22-28`:

```rust
#[test]
fn test_version_flag() {
    let mut cmd = Command::cargo_bin("mutx").unwrap();
    cmd.arg("--version")
        .assert()
        .success()
        .stdout(predicate::str::contains("0.3.0"));
}
```

**Step 5: Run version test**

Run: `cargo test test_version_flag`
Expected: PASS

**Step 6: Commit version rollback**

```bash
git add Cargo.toml mutx-macros/Cargo.toml CHANGELOG.md tests/cli_args_test.rs
git commit -m "chore: roll back to v0.3.0 for pre-release clarity

Version 1.1.0 incorrectly suggested stable/production-ready status.
Rolling back to v0.3.0 to signal ongoing pre-release development.

Version 1.0.0 will be reserved for the first stable public release
when the CLI interface is finalized and thoroughly tested.

Updated:
- Cargo.toml: version = \"0.3.0\"
- CHANGELOG.md: Comprehensive rewrite with version rationale
- Tests: Version assertions updated to 0.3.0

Co-Authored-By: Claude Opus 4.5 <noreply@anthropic.com>"
```

### Task 9: Update README Documentation

**Files:**
- Modify: `README.md`

**Step 1: Add pre-release notice at top**

Add after the description line in `README.md`:

```markdown
# mutx

A command-line tool for atomic file writes with process coordination through file locking.

> **Pre-release Software:** mutx is currently in active development (v0.3.x). The CLI interface
> and behavior may change between releases. Version 1.0.0 will mark the first stable release.

## Features
```

**Step 2: Fix all mutx write commands**

Find and replace in README.md:

```bash
# OLD (incorrect):
mutx write output.txt

# NEW (both correct):
mutx output.txt
mutx write output.txt
```

Update these sections:
- Lock File Behavior examples (lines ~80-101)
- Security Considerations examples (lines ~86-101)

Correct examples:

```markdown
## Lock File Behavior

mutx uses file locks to coordinate between processes. Lock files are automatically
created in your platform's cache directory (not alongside your output files).

### Lock Persistence

Lock files persist after your command completes. This is intentional and prevents
race conditions where one process might wait on a deleted lock file while another
creates a new one.

To clean up orphaned lock files:

```bash
mutx housekeep locks
```

### Custom Lock Locations

You can specify a custom lock file location:

```bash
mutx output.txt --lock-file /tmp/my-custom.lock
```

Note: Custom lock files are not automatically cleaned by housekeep.

## Security Considerations

### Symlink Handling

By default, mutx rejects symbolic links for security:

```bash
# This will fail if output.txt is a symlink
mutx output.txt < input.txt

# Allow symlinks for output files
mutx output.txt --follow-symlinks < input.txt

# Allow symlinks even for lock files (not recommended)
mutx output.txt --follow-lock-symlinks < input.txt
```
```

**Step 3: Update housekeep examples**

Replace housekeep examples section (lines ~119-127):

```markdown
### Lock Cleanup

```bash
# Clean locks from cache directory (default location)
mutx housekeep locks

# Clean locks older than 1 hour from specific directory
mutx housekeep locks --older-than 1h /var/lib/app

# Clean backups, keep 3 newest per file
mutx housekeep backups --keep-newest 3 /data

# Clean custom backup suffix
mutx housekeep backups --suffix .bak

# Clean both from same directory
mutx housekeep all /var/lib/app

# Clean both from different directories
mutx housekeep all --locks-dir ~/.cache/mutx/locks --backups-dir /data

# Dry run to see what would be deleted
mutx housekeep locks --dry-run -v
```
```

**Step 4: Update housekeep command documentation**

Replace Housekeep Command section:

```markdown
### Housekeep Command

```
mutx housekeep <OPERATION> [OPTIONS] [DIR]
```

**Operations:**
- `locks`: Clean orphaned lock files (defaults to cache directory)
- `backups`: Clean old backup files (defaults to current directory)
- `all`: Clean both locks and backups

**Common Options:**
- `-r, --recursive`: Scan subdirectories
- `--older-than <DURATION>`: Age threshold (e.g., "2h", "7d")
- `-n, --dry-run`: Show what would be deleted
- `-v, --verbose`: Verbose output

**Locks Options:**
- `[DIR]`: Directory to clean (default: platform cache directory)

**Backups Options:**
- `[DIR]`: Directory to clean (default: current directory)
- `--keep-newest <N>`: Keep N newest backups per file
- `--suffix <SUFFIX>`: Backup suffix to match (default: .mutx.backup)

**All Options:**
- `[DIR]`: Clean both from same directory
- `--locks-dir <DIR>`: Lock files directory (requires --backups-dir)
- `--backups-dir <DIR>`: Backup files directory (requires --locks-dir)
```

**Step 5: Add explicit write subcommand note**

Add to Usage section after Write Command:

```markdown
### Write Command

Write is the default command and can be invoked implicitly or explicitly:

```bash
# Implicit (recommended for brevity)
mutx output.txt < input.txt

# Explicit (clearer intent)
mutx write output.txt < input.txt
```

Both forms are identical in behavior.

```
mutx [write] [OPTIONS] <OUTPUT>
```
```

**Step 6: Verify README formatting**

Run: `grep -n "mutx write" README.md`
Expected: Only appears in documentation explaining both forms

**Step 7: Commit README updates**

```bash
git add README.md
git commit -m "docs: update README for v0.3.0 changes

- Add pre-release notice at top
- Document both implicit and explicit write forms
- Update all housekeep examples to use new subcommand syntax
- Fix incorrect 'mutx write' usage (now documented as optional)
- Document housekeep smart defaults
- Add --suffix flag documentation for housekeep backups

All command examples now match actual v0.3.0 CLI.

Co-Authored-By: Claude Opus 4.5 <noreply@anthropic.com>"
```

---

## Final Verification

### Task 10: Full Test Suite and Manual Testing

**Step 1: Run complete test suite**

Run: `cargo test`
Expected: All tests PASS

**Step 2: Test implicit write**

```bash
echo "implicit test" | cargo run -- /tmp/test-implicit.txt
cat /tmp/test-implicit.txt
```

Expected: File contains "implicit test"

**Step 3: Test explicit write**

```bash
echo "explicit test" | cargo run -- write /tmp/test-explicit.txt
cat /tmp/test-explicit.txt
```

Expected: File contains "explicit test"

**Step 4: Test custom backup suffix**

```bash
echo "original" > /tmp/test-backup.txt
echo "updated" | cargo run -- --backup --backup-suffix .bak /tmp/test-backup.txt
ls /tmp/test-backup.txt*
```

Expected: Both test-backup.txt and test-backup.txt.bak exist

**Step 5: Test housekeep locks (cache directory)**

```bash
cargo run -- housekeep locks --dry-run -v
```

Expected: Shows cache directory path, lists lock files

**Step 6: Test housekeep backups with custom suffix**

```bash
mkdir -p /tmp/test-housekeep
echo "test" > /tmp/test-housekeep/file.txt.bak
echo "test" > /tmp/test-housekeep/file.txt.custom
cargo run -- housekeep backups --suffix .bak /tmp/test-housekeep
ls /tmp/test-housekeep/
```

Expected: .bak file deleted, .custom file remains

**Step 7: Test housekeep all with validation**

```bash
cargo run -- housekeep all
```

Expected: Error message about required arguments

**Step 8: Check version**

Run: `cargo run -- --version`
Expected: Output contains "0.3.0"

**Step 9: Check help**

```bash
cargo run -- --help
cargo run -- write --help
cargo run -- housekeep --help
cargo run -- housekeep locks --help
```

Expected: All help messages display correctly

**Step 10: Final commit**

```bash
git log --oneline -10
```

Expected: Shows all commits from this implementation

---

## Summary

### Commits Created

1. `fix: make --backup-suffix functional`
2. `feat(housekeep): support custom backup suffixes`
3. `feat: add mutx-macros crate for custom attributes`
4. `feat: add explicit write subcommand with implicit fallback`
5. `feat!: restructure housekeep as nested subcommands`
6. `chore: roll back to v0.3.0 for pre-release clarity`
7. `docs: update README for v0.3.0 changes`

### Breaking Changes

- Housekeep commands: flags → subcommands
- Version: v1.1.0 → v0.3.0 (signaling pre-release)

### New Features

- `--backup-suffix` now functional
- `mutx write` explicit subcommand
- Housekeep smart defaults
- Custom suffix cleanup support

### Files Modified

**Core:**
- `Cargo.toml` (workspace, version, dependencies)
- `src/backup.rs` (use config.suffix)
- `src/housekeep.rs` (suffix parameter)
- `src/cli/args.rs` (Write + HousekeepOperation)
- `src/cli/mod.rs` (dispatch logic)
- `src/cli/write_command.rs` (explicit handler)
- `src/cli/housekeep_command.rs` (operation handlers)

**New:**
- `mutx-macros/` (entire crate)
- `tests/backup_suffix_test.rs`
- `tests/write_subcommand_test.rs`
- `tests/housekeep_subcommand_test.rs`

**Updated:**
- `CHANGELOG.md` (comprehensive rewrite)
- `README.md` (examples, pre-release notice)
- `tests/cli_args_test.rs` (version)
- `tests/cli_housekeep_test.rs` (syntax)
- `tests/housekeep_test.rs` (custom suffix)

### Reference Skills

- @superpowers:executing-plans - Use this to implement task-by-task
- @superpowers:requesting-code-review - Use after each phase
- @superpowers:verification-before-completion - Use before final commit
