# Housekeep Safety Fixes Implementation Plan

> **For Claude:** REQUIRED SUB-SKILL: Use superpowers:executing-plans to implement this plan task-by-task.

**Goal:** Fix three safety issues in housekeep: empty suffix validation, cache directory panics, and misleading dry-run output

**Architecture:** Add input validation before operations, make cache directory access fallible with proper error propagation, and clarify output messaging with conditional verbs

**Tech Stack:** Rust, Clap 4.x CLI, assert_cmd for integration tests

---

## Task 1: Add Suffix Validation Tests

**Files:**
- Create: `tests/housekeep_suffix_validation_test.rs`

**Step 1: Write the failing tests**

Create `tests/housekeep_suffix_validation_test.rs`:

```rust
use assert_cmd::Command;
use predicates::prelude::*;
use tempfile::TempDir;

#[test]
fn test_empty_suffix_rejected() {
    let dir = TempDir::new().unwrap();

    let mut cmd = Command::cargo_bin("mutx").unwrap();
    cmd.arg("housekeep")
        .arg("backups")
        .arg("--suffix")
        .arg("")
        .arg("--dry-run")
        .arg(dir.path())
        .assert()
        .failure()
        .stderr(predicate::str::contains("cannot be empty"));
}

#[test]
fn test_single_dot_suffix_rejected() {
    let dir = TempDir::new().unwrap();

    let mut cmd = Command::cargo_bin("mutx").unwrap();
    cmd.arg("housekeep")
        .arg("backups")
        .arg("--suffix")
        .arg(".")
        .arg("--dry-run")
        .arg(dir.path())
        .assert()
        .failure()
        .stderr(predicate::str::contains("cannot be a single dot"));
}

#[test]
fn test_valid_suffixes_accepted() {
    let dir = TempDir::new().unwrap();

    // Test .bak suffix
    let mut cmd = Command::cargo_bin("mutx").unwrap();
    cmd.arg("housekeep")
        .arg("backups")
        .arg("--suffix")
        .arg(".bak")
        .arg("--dry-run")
        .arg(dir.path())
        .assert()
        .success();

    // Test backup suffix (no leading dot)
    let mut cmd = Command::cargo_bin("mutx").unwrap();
    cmd.arg("housekeep")
        .arg("backups")
        .arg("--suffix")
        .arg("backup")
        .arg("--dry-run")
        .arg(dir.path())
        .assert()
        .success();

    // Test compound suffix
    let mut cmd = Command::cargo_bin("mutx").unwrap();
    cmd.arg("housekeep")
        .arg("backups")
        .arg("--suffix")
        .arg(".old.bak")
        .arg("--dry-run")
        .arg(dir.path())
        .assert()
        .success();
}

#[test]
fn test_empty_suffix_rejected_in_all_command() {
    let dir = TempDir::new().unwrap();

    let mut cmd = Command::cargo_bin("mutx").unwrap();
    cmd.arg("housekeep")
        .arg("all")
        .arg("--suffix")
        .arg("")
        .arg("--dry-run")
        .arg(dir.path())
        .assert()
        .failure()
        .stderr(predicate::str::contains("cannot be empty"));
}
```

**Step 2: Run tests to verify they fail**

Run: `cargo test housekeep_suffix_validation_test`

Expected: All 4 tests FAIL (validation not implemented yet)

**Step 3: Add validate_suffix function**

Add to `src/cli/housekeep_command.rs` before `execute_housekeep`:

```rust
fn validate_suffix(suffix: &str) -> Result<()> {
    if suffix.is_empty() {
        return Err(MutxError::Other(
            "Backup suffix cannot be empty".to_string(),
        ));
    }

    if suffix == "." {
        return Err(MutxError::Other(
            "Backup suffix cannot be a single dot".to_string(),
        ));
    }

    Ok(())
}
```

**Step 4: Call validation in Backups handler**

In `execute_housekeep`, at the start of the `Backups` match arm:

```rust
HousekeepOperation::Backups {
    dir,
    recursive,
    older_than,
    keep_newest,
    suffix,
    dry_run,
    verbose,
} => {
    validate_suffix(&suffix)?;  // Add this line

    // Smart default: use current directory
    let target_dir = dir.unwrap_or_else(|| PathBuf::from("."));
    // ... rest of existing logic
}
```

**Step 5: Call validation in All handler**

In `execute_housekeep`, at the start of the `All` match arm:

```rust
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
} => {
    validate_suffix(&suffix)?;  // Add this line

    // Validation: require either dir OR both locks_dir and backups_dir
    let (locks_path, backups_path) = match (dir, locks_dir, backups_dir) {
        // ... rest of existing logic
    };
    // ... rest of existing logic
}
```

**Step 6: Run tests to verify they pass**

Run: `cargo test housekeep_suffix_validation_test`

Expected: All 4 tests PASS

**Step 7: Run full test suite**

Run: `cargo test`

Expected: All tests pass (no regressions)

**Step 8: Commit**

```bash
git add tests/housekeep_suffix_validation_test.rs src/cli/housekeep_command.rs
git commit -m "fix(housekeep): validate backup suffix to prevent accidental deletion

Reject empty string and single dot suffixes that would match all files.
This prevents catastrophic deletion from CLI typos like --suffix \"\".

Valid suffixes like .bak, backup, and .old.bak continue to work.

Co-Authored-By: Claude Opus 4.5 <noreply@anthropic.com>"
```

---

## Task 2: Make get_lock_cache_dir Fallible

**Files:**
- Modify: `src/lock/path.rs:94-100` (function signature and implementation)
- Modify: `src/cli/housekeep_command.rs:24` (caller)

**Step 1: Write test for cache directory error handling**

This is tricky to test automatically (requires permission manipulation). Document manual test:

Add comment in `src/lock/path.rs` above `get_lock_cache_dir`:

```rust
/// Get the platform-specific cache directory for lock files.
///
/// Returns an error if the cache directory cannot be determined
/// (e.g., on systems without a home directory or with permission issues).
///
/// Users can work around this by providing an explicit directory
/// to housekeep commands.
///
/// # Manual Testing
///
/// ```bash
/// # Restrict cache directory permissions
/// chmod 000 ~/.cache
/// mutx housekeep locks
/// # Should show error, not panic
/// chmod 755 ~/.cache  # Restore
/// ```
```

**Step 2: Update get_lock_cache_dir signature**

In `src/lock/path.rs`, change function signature and implementation:

```rust
// Before
pub fn get_lock_cache_dir() -> PathBuf {
    directories::ProjectDirs::from("", "", "mutx")
        .map(|proj_dirs| proj_dirs.cache_dir().join("locks"))
        .expect("Failed to determine cache directory")
}

// After
pub fn get_lock_cache_dir() -> Result<PathBuf> {
    directories::ProjectDirs::from("", "", "mutx")
        .map(|proj_dirs| proj_dirs.cache_dir().join("locks"))
        .ok_or_else(|| {
            MutxError::Other(
                "Failed to determine lock cache directory. \
                 Try specifying an explicit directory with the DIR argument."
                    .to_string(),
            )
        })
}
```

**Step 3: Check for other callers**

Run: `grep -r "get_lock_cache_dir" src/`

Expected: Only found in `src/cli/housekeep_command.rs` and `src/lock/path.rs`

**Step 4: Update caller in housekeep_command.rs**

In `execute_housekeep`, in the `Locks` match arm:

```rust
// Before
let target_dir = dir.unwrap_or_else(|| get_lock_cache_dir().unwrap());

// After
let target_dir = match dir {
    Some(d) => d,
    None => get_lock_cache_dir()?,
};
```

**Step 5: Build to verify**

Run: `cargo build`

Expected: Clean build

**Step 6: Run tests**

Run: `cargo test`

Expected: All tests pass

**Step 7: Manual verification**

Test that error message is clear:

```bash
# This should work normally
cargo run -- housekeep locks --dry-run

# Manual test with permission issues (if possible on your system)
# Verify error is clear, not a panic
```

**Step 8: Commit**

```bash
git add src/lock/path.rs src/cli/housekeep_command.rs
git commit -m "fix(housekeep): propagate cache directory errors instead of panicking

Make get_lock_cache_dir() return Result<PathBuf> instead of panicking
on permission errors or filesystem issues. Provides clear error message
with workaround suggestion.

Users can work around by specifying explicit directory:
  mutx housekeep locks /path/to/dir

Co-Authored-By: Claude Opus 4.5 <noreply@anthropic.com>"
```

---

## Task 3: Clarify Dry-Run Output

**Files:**
- Modify: `src/cli/housekeep_command.rs:120-129` (report_cleaning_results function)
- Modify: `src/cli/housekeep_command.rs` (all call sites)
- Create: `tests/housekeep_dry_run_output_test.rs`

**Step 1: Write tests for dry-run output**

Create `tests/housekeep_dry_run_output_test.rs`:

```rust
use assert_cmd::Command;
use predicates::prelude::*;
use std::fs;
use tempfile::TempDir;

#[test]
fn test_dry_run_shows_would_clean() {
    let dir = TempDir::new().unwrap();

    // Create backup files
    fs::write(dir.path().join("file.txt.mutx.backup"), "backup").unwrap();

    let mut cmd = Command::cargo_bin("mutx").unwrap();
    cmd.arg("housekeep")
        .arg("backups")
        .arg("--dry-run")
        .arg("--older-than")
        .arg("0s")
        .arg(dir.path())
        .assert()
        .success()
        .stdout(predicate::str::contains("Would clean"));

    // Verify file still exists
    assert!(dir.path().join("file.txt.mutx.backup").exists());
}

#[test]
fn test_normal_run_shows_cleaned() {
    let dir = TempDir::new().unwrap();

    // Create backup files
    fs::write(dir.path().join("file.txt.mutx.backup"), "backup").unwrap();

    let mut cmd = Command::cargo_bin("mutx").unwrap();
    cmd.arg("housekeep")
        .arg("backups")
        .arg("--older-than")
        .arg("0s")
        .arg(dir.path())
        .assert()
        .success()
        .stdout(predicate::str::contains("Cleaned"));

    // Verify file was deleted
    assert!(!dir.path().join("file.txt.mutx.backup").exists());
}

#[test]
fn test_dry_run_verbose_shows_would_clean() {
    let dir = TempDir::new().unwrap();

    // Create backup files
    fs::write(dir.path().join("file.txt.mutx.backup"), "backup").unwrap();

    let mut cmd = Command::cargo_bin("mutx").unwrap();
    cmd.arg("housekeep")
        .arg("backups")
        .arg("--dry-run")
        .arg("--verbose")
        .arg("--older-than")
        .arg("0s")
        .arg(dir.path())
        .assert()
        .success()
        .stdout(predicate::str::contains("Would clean"))
        .stdout(predicate::str::contains("file.txt.mutx.backup"));
}

#[test]
fn test_all_command_dry_run_shows_would_clean() {
    let dir = TempDir::new().unwrap();

    // Create backup files
    fs::write(dir.path().join("file.txt.mutx.backup"), "backup").unwrap();

    let mut cmd = Command::cargo_bin("mutx").unwrap();
    cmd.arg("housekeep")
        .arg("all")
        .arg("--dry-run")
        .arg("--older-than")
        .arg("0s")
        .arg(dir.path())
        .assert()
        .success()
        .stdout(predicate::str::contains("Would clean"));
}
```

**Step 2: Run tests to verify they fail**

Run: `cargo test housekeep_dry_run_output_test`

Expected: Tests with "Would clean" FAIL (not implemented yet)

**Step 3: Update report_cleaning_results signature**

In `src/cli/housekeep_command.rs`, update the function:

```rust
// Before
fn report_cleaning_results(item_type: &str, cleaned: &[PathBuf], verbose: bool) {
    if cleaned.is_empty() {
        println!("No {} files to clean", item_type);
    } else {
        println!("Cleaned {} {} file(s)", cleaned.len(), item_type);
        if verbose {
            for path in cleaned {
                println!("  - {}", path.display());
            }
        }
    }
}

// After
fn report_cleaning_results(
    item_type: &str,
    cleaned: &[PathBuf],
    verbose: bool,
    dry_run: bool,
) {
    let verb = if dry_run { "Would clean" } else { "Cleaned" };

    if cleaned.is_empty() {
        println!("No {} files to clean", item_type);
    } else {
        println!("{} {} {} file(s)", verb, cleaned.len(), item_type);
        if verbose {
            for path in cleaned {
                println!("  - {}", path.display());
            }
        }
    }
}
```

**Step 4: Update call site in Locks handler**

In `execute_housekeep`, in the `Locks` match arm:

```rust
// Before
report_cleaning_results("lock", &cleaned, verbose);

// After
report_cleaning_results("lock", &cleaned, verbose, dry_run);
```

**Step 5: Update call site in Backups handler**

In `execute_housekeep`, in the `Backups` match arm:

```rust
// Before
report_cleaning_results("backup", &cleaned, verbose);

// After
report_cleaning_results("backup", &cleaned, verbose, dry_run);
```

**Step 6: Update call sites in All handler**

In `execute_housekeep`, in the `All` match arm (two calls):

```rust
// Before
report_cleaning_results("lock", &cleaned_locks, verbose);
report_cleaning_results("backup", &cleaned_backups, verbose);

// After
report_cleaning_results("lock", &cleaned_locks, verbose, dry_run);
report_cleaning_results("backup", &cleaned_backups, verbose, dry_run);
```

**Step 7: Run tests to verify they pass**

Run: `cargo test housekeep_dry_run_output_test`

Expected: All 4 tests PASS

**Step 8: Run full test suite**

Run: `cargo test`

Expected: All tests pass

**Step 9: Manual verification**

```bash
# Create test directory with backup
mkdir -p /tmp/test-dry-run
echo "backup" > /tmp/test-dry-run/file.txt.mutx.backup

# Test dry-run output
cargo run -- housekeep backups --dry-run --older-than 0s /tmp/test-dry-run
# Should show "Would clean"

# Test normal output
cargo run -- housekeep backups --older-than 0s /tmp/test-dry-run
# Should show "Cleaned"

# Cleanup
rm -rf /tmp/test-dry-run
```

**Step 10: Commit**

```bash
git add src/cli/housekeep_command.rs tests/housekeep_dry_run_output_test.rs
git commit -m "fix(housekeep): clarify dry-run output with conditional verb

Add dry_run parameter to report_cleaning_results() and use conditional
verb: 'Cleaned' for actual deletions, 'Would clean' for dry-run.

This prevents misleading output during audits where users might think
files were actually deleted during dry-run mode.

Matches convention of other CLI tools like rsync and rm.

Co-Authored-By: Claude Opus 4.5 <noreply@anthropic.com>"
```

---

## Task 4: Final Verification

**Files:**
- None (verification only)

**Step 1: Run complete test suite**

Run: `cargo test --workspace`

Expected: All tests pass (including new safety tests)

**Step 2: Run clippy**

Run: `cargo clippy --workspace -- -D warnings`

Expected: No warnings

**Step 3: Manual integration test - Suffix validation**

```bash
# Should fail with clear error
cargo run -- housekeep backups --suffix ""
cargo run -- housekeep backups --suffix .

# Should work
cargo run -- housekeep backups --suffix .bak --dry-run
cargo run -- housekeep backups --suffix backup --dry-run
```

**Step 4: Manual integration test - Dry-run output**

```bash
# Create test backups
mkdir -p /tmp/test-safety
echo "backup" > /tmp/test-safety/file.txt.mutx.backup

# Check dry-run says "Would clean"
cargo run -- housekeep backups --dry-run --older-than 0s /tmp/test-safety

# Check normal says "Cleaned"
cargo run -- housekeep backups --older-than 0s /tmp/test-safety

# Cleanup
rm -rf /tmp/test-safety
```

**Step 5: Verify git log**

Run: `git log --oneline -5`

Expected: 3 new commits for safety fixes

**Step 6: Verify no uncommitted changes**

Run: `git status`

Expected: Clean working tree

**Step 7: Document completion**

All three safety issues fixed:
- ✅ Empty suffix validation prevents catastrophic deletion
- ✅ Cache directory errors propagated instead of panicking
- ✅ Dry-run output clarified with conditional verb

All tests passing, no regressions, ready for merge.

---

## Success Criteria

- [ ] All 4 suffix validation tests pass
- [ ] All 4 dry-run output tests pass
- [ ] Full test suite passes (no regressions)
- [ ] Clippy reports no warnings
- [ ] Manual tests confirm expected behavior
- [ ] Three commits with clear messages
- [ ] Clean git working tree

---

## Notes

**Test Count:**
- New tests: 8 (4 suffix validation + 4 dry-run output)
- Existing tests: ~80
- Total: ~88 tests

**Manual Testing:**
- Cache directory error handling requires permission manipulation
- Document manual test procedure in code comments

**No Breaking Changes:**
- All fixes are additive or fix bugs
- Invalid inputs now error instead of causing problems
- Output improvements maintain structure
