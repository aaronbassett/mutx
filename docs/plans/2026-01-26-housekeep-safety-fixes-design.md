# Housekeep Safety Fixes Design

**Date:** 2026-01-26
**Status:** Design Complete, Ready for Implementation

## Overview

Three safety issues discovered in housekeep implementation after v0.3.0:

1. **Empty suffix catastrophe**: `--suffix ""` matches all files, could delete non-backups
2. **Cache directory panic**: `get_lock_cache_dir().unwrap()` panics on permission errors
3. **Misleading dry-run output**: "Cleaned N files" shown even when no files deleted

All three issues have clear, minimal fixes that improve safety without restricting legitimate use cases.

---

## Issue 1: Empty Suffix Validation

### Problem

**Location:** `src/cli/args.rs:119-155` + `src/housekeep.rs:177-182`

The `--suffix` flag accepts any string, including empty strings. Since `is_backup_file()` uses `ends_with(suffix)`, an empty suffix matches every filename:

```rust
fn is_backup_file(path: &Path, suffix: &str) -> bool {
    path.file_name()
        .and_then(|s| s.to_str())
        .map(|name| name.ends_with(suffix))  // "" matches everything!
        .unwrap_or(false)
}
```

**Catastrophic scenario:**
```bash
mutx housekeep backups --suffix "" /important/data
# Would delete ALL files in /important/data
```

### Solution

**Validation function in `src/cli/housekeep_command.rs`:**

```rust
fn validate_suffix(suffix: &str) -> Result<()> {
    if suffix.is_empty() {
        return Err(MutxError::Other(
            "Backup suffix cannot be empty".to_string()
        ));
    }

    if suffix == "." {
        return Err(MutxError::Other(
            "Backup suffix cannot be a single dot".to_string()
        ));
    }

    Ok(())
}
```

**Usage in `execute_housekeep`:**

```rust
HousekeepOperation::Backups {
    suffix,
    // ... other fields
} => {
    validate_suffix(&suffix)?;  // Fail fast before any operations

    let target_dir = dir.unwrap_or_else(|| PathBuf::from("."));
    // ... rest of logic
}

HousekeepOperation::All {
    suffix,
    // ... other fields
} => {
    validate_suffix(&suffix)?;  // Fail fast before any operations

    // ... rest of logic
}
```

### Why This Works

- **Prevents catastrophe**: Rejects empty string and single dot
- **Fails fast**: Validation before any filesystem operations
- **Clear errors**: User knows exactly what went wrong
- **Flexible**: Allows `.bak`, `backup`, `.deploy.bak`, etc.
- **No breaking changes**: Default `.mutx.backup` still works

### Examples

```bash
# Error cases
$ mutx housekeep backups --suffix ""
Error: Backup suffix cannot be empty

$ mutx housekeep backups --suffix .
Error: Backup suffix cannot be a single dot

# Valid cases
$ mutx housekeep backups --suffix .bak       # Works
$ mutx housekeep backups --suffix backup     # Works (no dot required)
$ mutx housekeep backups --suffix .old.bak   # Works
```

---

## Issue 2: Cache Directory Error Handling

### Problem

**Location:** `src/cli/housekeep_command.rs:24`

The `get_lock_cache_dir()` function can fail (permission errors, missing directories), but calling code uses `.unwrap()`:

```rust
let target_dir = dir.unwrap_or_else(|| get_lock_cache_dir().unwrap());
```

**Panic scenarios:**
- Cache directory permissions denied
- Filesystem errors
- Missing home directory (rare but possible)

### Solution

**Make `get_lock_cache_dir()` fallible in `src/lock/path.rs`:**

```rust
// Before
pub fn get_lock_cache_dir() -> PathBuf {
    // ... implementation
}

// After
pub fn get_lock_cache_dir() -> Result<PathBuf> {
    directories::ProjectDirs::from("", "", "mutx")
        .map(|proj_dirs| proj_dirs.cache_dir().join("locks"))
        .ok_or_else(|| MutxError::Other(
            "Failed to determine lock cache directory".to_string()
        ))
}
```

**Update callers in `src/cli/housekeep_command.rs`:**

```rust
// Before
let target_dir = dir.unwrap_or_else(|| get_lock_cache_dir().unwrap());

// After
let target_dir = match dir {
    Some(d) => d,
    None => get_lock_cache_dir()?,
};
```

### Why This Works

- **No panics**: All errors structured and propagated
- **Clear messages**: User knows cache directory failed
- **Workaround available**: User can specify explicit `--dir` flag
- **Consistent**: Matches Rust error handling patterns

### Error Message Example

```bash
$ mutx housekeep locks
Error: Failed to determine lock cache directory

# Workaround
$ mutx housekeep locks /tmp/locks
```

### Other Call Sites

**Check if `get_lock_cache_dir()` is called elsewhere:**

```bash
grep -r "get_lock_cache_dir" src/
```

Update all callers to handle `Result<PathBuf>`.

---

## Issue 3: Dry-Run Output Clarity

### Problem

**Location:** `src/cli/housekeep_command.rs:120-128`

The `report_cleaning_results()` function prints "Cleaned N files" even during `--dry-run`:

```rust
fn report_cleaning_results(item_type: &str, cleaned: &[PathBuf], verbose: bool) {
    if cleaned.is_empty() {
        println!("No {} files to clean", item_type);
    } else {
        println!("Cleaned {} {} file(s)", cleaned.len(), item_type);
        // ...
    }
}
```

**Misleading output:**
```bash
$ mutx housekeep backups --dry-run
Cleaned 5 backup file(s)  # Files NOT actually deleted!
```

### Solution

**Add `dry_run` parameter and conditional verb:**

```rust
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

**Update all call sites in `execute_housekeep`:**

```rust
// Locks operation
report_cleaning_results("lock", &cleaned, verbose, dry_run);

// Backups operation
report_cleaning_results("backup", &cleaned, verbose, dry_run);

// All operation (both calls)
report_cleaning_results("lock", &cleaned_locks, verbose, dry_run);
report_cleaning_results("backup", &cleaned_backups, verbose, dry_run);
```

### Why This Works

- **Clear distinction**: "Cleaned" vs "Would clean"
- **Conventional**: Matches rsync, rm, and other CLI tools
- **Minimal change**: Just conditional verb, no format restructuring
- **No breaking changes**: Output structure unchanged

### Output Examples

```bash
# Normal mode
$ mutx housekeep backups
Cleaned 3 backup file(s)

# Dry-run mode
$ mutx housekeep backups --dry-run
Would clean 3 backup file(s)

# Verbose dry-run
$ mutx housekeep backups --dry-run --verbose
Would clean 2 backup file(s)
  - /data/file.txt.mutx.backup
  - /data/other.txt.mutx.backup
```

---

## Implementation Order

### Phase 1: Suffix Validation
1. Add `validate_suffix()` function to `housekeep_command.rs`
2. Call validation in `Backups` and `All` handlers
3. Add tests for empty string and single dot
4. Commit: "fix(housekeep): validate backup suffix to prevent accidental deletion"

### Phase 2: Cache Directory Error Handling
1. Update `get_lock_cache_dir()` signature to return `Result<PathBuf>`
2. Update implementation with proper error handling
3. Update all callers in `housekeep_command.rs`
4. Search and update any other callers
5. Test error case (manually, with permission issues)
6. Commit: "fix(housekeep): propagate cache directory errors instead of panicking"

### Phase 3: Dry-Run Output
1. Add `dry_run` parameter to `report_cleaning_results()`
2. Add conditional verb logic
3. Update all 4 call sites
4. Test both dry-run and normal modes
5. Commit: "fix(housekeep): clarify dry-run output with conditional verb"

---

## Testing Requirements

### New Tests Needed

**1. Suffix validation tests:**
```rust
#[test]
fn test_empty_suffix_rejected() {
    let mut cmd = Command::cargo_bin("mutx").unwrap();
    cmd.arg("housekeep")
        .arg("backups")
        .arg("--suffix")
        .arg("")
        .arg("/tmp")
        .assert()
        .failure()
        .stderr(predicate::str::contains("cannot be empty"));
}

#[test]
fn test_single_dot_suffix_rejected() {
    let mut cmd = Command::cargo_bin("mutx").unwrap();
    cmd.arg("housekeep")
        .arg("backups")
        .arg("--suffix")
        .arg(".")
        .arg("/tmp")
        .assert()
        .failure()
        .stderr(predicate::str::contains("cannot be a single dot"));
}

#[test]
fn test_valid_suffixes_accepted() {
    // Test .bak, backup, .old.bak, etc.
}
```

**2. Dry-run output tests:**
```rust
#[test]
fn test_dry_run_shows_would_clean() {
    // Create backup files
    // Run with --dry-run
    // Assert output contains "Would clean"
    // Assert files still exist
}

#[test]
fn test_normal_run_shows_cleaned() {
    // Create backup files
    // Run without --dry-run
    // Assert output contains "Cleaned"
    // Assert files deleted
}
```

**3. Cache directory error handling:**
- Manual test: restrict cache dir permissions, verify error (not panic)
- Unit test: mock `get_lock_cache_dir()` to return error

---

## Breaking Changes

**None.** All changes are additive or fix existing bugs:

- Suffix validation: Rejects invalid inputs that would cause bugs
- Cache directory: Replaces panic with proper error
- Dry-run output: Improves clarity without changing structure

---

## Architecture Notes

### Why Minimal Validation?

**Rejected alternatives:**
- **Require leading dot**: Too restrictive (users might want `backup`, `.old.bak`)
- **Whitelist patterns**: Inflexible, doesn't cover all use cases
- **Minimum length**: Arbitrary, doesn't add safety

**Chosen approach:**
- Only reject patterns that cause bugs (empty, single dot)
- Allow flexibility for user workflows
- Clear error messages guide correct usage

### Why Make get_lock_cache_dir() Fallible?

**Rejected alternatives:**
- **Fallback to temp dir**: Surprising behavior, locks in wrong place
- **Fallback to current dir**: Could clutter working directory

**Chosen approach:**
- Explicit errors let user know what failed
- User can provide explicit `--dir` as workaround
- Consistent with Rust error handling philosophy

### Why Conditional Verb for Dry-Run?

**Rejected alternatives:**
- **Prefix format** (`[DRY-RUN] Cleaned`): Visual noise
- **Different structure** (`Found N files that would be cleaned`): Breaking change

**Chosen approach:**
- Minimal, conventional approach
- Matches other CLI tools (rsync, rm)
- No breaking changes

---

## Future Considerations

### Additional Safety Features

**For v0.4.0 or later:**
- Interactive confirmation for large deletions
- Backup before cleanup (housekeep the housekeep)
- Undo mechanism (restore from trash)
- Progress bars for long operations

### Additional Validations

**Could add (if needed):**
- Suffix length limits (prevent abuse)
- Character restrictions (no spaces, special chars)
- Warning for suspicious patterns

**Not recommended now:**
- These add complexity without clear benefit
- Current validation prevents catastrophic failures
- Additional restrictions can come later if needed
