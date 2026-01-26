# Post-Release Fixes Design (v0.3.0)

**Date:** 2026-01-26
**Status:** Design Complete, Ready for Implementation

## Overview

Three interconnected issues discovered after v1.1.0 release:
1. `--backup-suffix` flag silently ignored (behavioral regression)
2. Housekeep UX issues (flag-based with poor defaults)
3. README shows non-existent `mutx write` subcommand

**Decision:** Roll version back to **v0.3.0** to signal pre-release status. Reserve v1.0.0 for first stable public release.

---

## Issue 1: Backup Suffix Regression

### Problem

- CLI accepts `--backup-suffix` flag (default: `.mutx.backup`)
- `BackupConfig.suffix` is populated but never used
- `generate_backup_path()` always hardcodes `.mutx.backup`

### Solution

Use `config.suffix` in backup name generation.

**File: `src/backup.rs:75-80`**

```rust
let backup_name = if config.timestamp {
    let timestamp = Local::now().format("%Y%m%d_%H%M%S");
    format!("{}.{}{}", filename, timestamp, config.suffix)
} else {
    format!("{}{}", filename, config.suffix)
};
```

### Examples

```bash
# Default behavior (unchanged)
mutx --backup output.txt < input.txt
# Creates: output.txt.mutx.backup

# Custom suffix
mutx --backup --backup-suffix .bak output.txt < input.txt
# Creates: output.txt.bak

# Grouped by process
mutx --backup --backup-suffix .deploy.backup output.txt < input.txt
# Creates: output.txt.deploy.backup

# With timestamp
mutx --backup --backup-timestamp --backup-suffix .bak output.txt < input.txt
# Creates: output.txt.20260126_143022.bak
```

---

## Issue 2: Housekeep UX Problems

### Current Problems

```bash
mutx housekeep --clean-locks [DIR]    # Defaults to "." but locks in cache
mutx housekeep --clean-backups [DIR]  # Defaults to "."
mutx housekeep --all [DIR]            # Can't have two defaults
```

### New Design: Subcommand-Based

```bash
mutx housekeep locks [DIR]            # Defaults to cache directory
mutx housekeep backups [DIR]          # Defaults to current directory
mutx housekeep all [DIR]              # Either [DIR] or both --locks-dir + --backups-dir
mutx housekeep                        # Shows help (required subcommand)
```

### CLI Structure

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

### Smart Defaults

- `mutx housekeep locks` → defaults to `get_lock_cache_dir()`
- `mutx housekeep backups` → defaults to current directory (`.`)
- `mutx housekeep all` → requires either `[DIR]` OR both `--locks-dir` and `--backups-dir`

### Validation Logic for `all`

```rust
match (dir, locks_dir, backups_dir) {
    (Some(d), None, None) => (d.clone(), d.clone()),
    (None, Some(ld), Some(bd)) => (ld.clone(), bd.clone()),
    _ => return Err(MutxError::Other(
        "Specify either [DIR] or both --locks-dir and --backups-dir".to_string()
    )),
}
```

---

## Issue 3: Explicit Write Subcommand

### Goal

Support both implicit and explicit write command:

```bash
mutx output.txt < input.txt              # Implicit write (current)
mutx write output.txt < input.txt        # Explicit write (new)
```

### Implementation: `#[implicit_command]` Macro

**Create new crate: `mutx-macros/`**

Simple attribute macro that marks a subcommand variant as the default when no subcommand is specified.

**File: `mutx-macros/src/lib.rs`**

```rust
use proc_macro::TokenStream;

#[proc_macro_attribute]
pub fn implicit_command(_attr: TokenStream, item: TokenStream) -> TokenStream {
    // This is a marker attribute - the actual logic is in Args::parse handling
    // Just pass through the item unchanged
    item
}
```

**File: `Cargo.toml`** - Add workspace member

```toml
[workspace]
members = [".", "mutx-macros"]

[dependencies]
mutx-macros = { path = "mutx-macros" }
```

**File: `mutx-macros/Cargo.toml`**

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
syn = "2.0"
```

**Usage in `src/cli/args.rs`:**

```rust
use mutx_macros::implicit_command;

#[derive(Subcommand, Debug)]
pub enum Command {
    #[implicit_command]
    Write {
        /// Target file path
        #[arg(value_name = "OUTPUT")]
        output: PathBuf,

        // All write options here (input, stream, backup, etc.)
    },

    Housekeep {
        #[command(subcommand)]
        operation: HousekeepOperation,
    },
}
```

**File: `src/cli/mod.rs`** - Handle dispatch

```rust
pub fn run(args: Args) -> Result<()> {
    match args.command {
        Some(Command::Write { .. }) => {
            // Explicit: mutx write output.txt
            write_command::execute_write_from_command(cmd)
        }
        Some(Command::Housekeep { operation }) => {
            housekeep_command::execute_housekeep(operation)
        }
        None => {
            // Implicit: mutx output.txt
            // Use top-level args for backward compatibility
            write_command::execute_write_from_args(args)
        }
    }
}
```

---

## Issue 4: Housekeep Custom Suffix Support

### Changes to `src/housekeep.rs`

**Update `CleanBackupConfig`:**

```rust
pub struct CleanBackupConfig {
    pub dir: PathBuf,
    pub recursive: bool,
    pub older_than: Option<Duration>,
    pub keep_newest: Option<usize>,
    pub dry_run: bool,
    pub suffix: String,  // NEW: default ".mutx.backup"
}
```

**Update `is_backup_file`:**

```rust
fn is_backup_file(path: &Path, suffix: &str) -> bool {
    path.file_name()
        .and_then(|s| s.to_str())
        .map(|name| name.ends_with(suffix))
        .unwrap_or(false)
}
```

**Update `extract_base_filename`:**

```rust
fn extract_base_filename(path: &Path, suffix: &str) -> String {
    let name = path
        .file_name()
        .and_then(|n| n.to_str())
        .unwrap_or("unknown");

    let without_suffix = match name.strip_suffix(suffix) {
        Some(s) => s,
        None => return name.to_string(),
    };

    // Try to parse timestamp: filename.YYYYMMDD_HHMMSS
    let parts: Vec<&str> = without_suffix.rsplitn(2, '.').collect();
    if parts.len() == 2 {
        let timestamp = parts[0];
        if is_valid_timestamp(timestamp) {
            return parts[1].to_string();  // Base filename
        }
    }

    without_suffix.to_string()
}
```

**Update `clean_backups`:**

```rust
pub fn clean_backups(config: &CleanBackupConfig) -> Result<Vec<PathBuf>> {
    let mut all_backups: HashMap<String, Vec<BackupFile>> = HashMap::new();

    visit_directory(&config.dir, config.recursive, &mut |path| {
        if is_backup_file(path, &config.suffix) {  // Pass suffix
            // ... rest of logic
            let base = extract_base_filename(path, &config.suffix);  // Pass suffix
            // ...
        }
        Ok(())
    })?;

    // ... rest of function
}
```

---

## Issue 5: Version Rollback to v0.3.0

### Rationale

Signal pre-release maturity. Reserve v1.0.0 for stable public release.

### Changes

**File: `Cargo.toml`**

```toml
[package]
version = "0.3.0"
```

**File: `CHANGELOG.md`** - Rename and add note

```markdown
# Changelog

## [0.3.0] - 2026-01-26

**Note:** Version numbers rolled back from v1.1.0 to v0.3.0 to better signal
pre-release status. Version 1.0.0 is reserved for the first stable public release.

### Added
- Explicit `mutx write` subcommand (implicit `mutx` still works)
- Housekeep subcommands: `locks`, `backups`, `all`
- `--suffix` flag for `housekeep backups` to clean custom backup patterns
- `--locks-dir` and `--backups-dir` flags for `housekeep all`

### Fixed
- **CRITICAL:** `--backup-suffix` now functional (was silently ignored in v1.1.0)
- Housekeep locks defaults to cache directory instead of current directory
- README examples corrected to match actual CLI syntax

### Changed
- **BREAKING:** Housekeep now uses subcommands instead of flags:
  - `mutx housekeep --clean-locks` → `mutx housekeep locks`
  - `mutx housekeep --clean-backups` → `mutx housekeep backups`
  - `mutx housekeep --all` → `mutx housekeep all [DIR]`
- Version numbering strategy: v0.x for pre-release, v1.0.0+ for stable

### Security (from v1.1.0)
- Lock files moved to platform cache directories to prevent collision attacks
- Symlink rejection by default with opt-in flags
- Backup format uses strict validation to prevent user file deletion
- Exponential backoff with jitter to prevent timing attacks

[... rest of v1.1.0 changes ...]
```

**File: `tests/cli_args_test.rs`**

```rust
cmd.arg("--version")
    .assert()
    .success()
    .stdout(predicate::str::contains("0.3.0"));  // Update version
```

---

## Documentation Updates

### README.md Fixes

**Remove non-existent `write` subcommand from all examples:**

```bash
# OLD (incorrect):
mutx write output.txt < input.txt

# NEW (correct):
mutx output.txt < input.txt

# Also correct:
mutx write output.txt < input.txt
```

**Update housekeep examples:**

```bash
# Clean locks from cache directory
mutx housekeep locks

# Clean locks older than 1 hour
mutx housekeep locks --older-than 1h

# Clean backups, keep 3 newest
mutx housekeep backups --keep-newest 3 /data

# Clean custom backup suffix
mutx housekeep backups --suffix .bak

# Clean both from same directory
mutx housekeep all /var/lib/app

# Clean both from different directories
mutx housekeep all --locks-dir ~/.cache/mutx/locks --backups-dir /data
```

---

## Implementation Order

### Phase 1: Backup Suffix Fix
1. Update `generate_backup_path()` to use `config.suffix`
2. Update tests for custom suffixes
3. Commit: "fix: make --backup-suffix functional"

### Phase 2: Housekeep Suffix Support
1. Add `suffix` field to `CleanBackupConfig`
2. Update `is_backup_file()` to accept suffix parameter
3. Update `extract_base_filename()` for generic suffixes
4. Update all `clean_backups()` calls to pass suffix
5. Add tests for custom suffix cleanup
6. Commit: "feat(housekeep): support custom backup suffixes"

### Phase 3: Macro and Write Subcommand
1. Create `mutx-macros/` crate with `#[implicit_command]`
2. Add workspace configuration
3. Add `Write` variant to `Command` enum with all options
4. Update `run()` dispatch to handle both implicit and explicit
5. Test both invocation styles
6. Commit: "feat: add explicit write subcommand with implicit fallback"

### Phase 4: Housekeep Restructure
1. Create `HousekeepOperation` enum with subcommands
2. Update `Command::Housekeep` to nest operations
3. Implement smart defaults (locks → cache, backups → current)
4. Implement `all` validation logic
5. Update housekeep_command.rs dispatch
6. Update all housekeep tests
7. Commit: "feat!: restructure housekeep as nested subcommands"

### Phase 5: Version Rollback and Documentation
1. Update Cargo.toml to 0.3.0
2. Rewrite CHANGELOG with version rationale
3. Update version assertions in tests
4. Fix all README examples (`mutx write` → `mutx`)
5. Update housekeep documentation
6. Add pre-release notice to README
7. Commit: "chore: roll back to v0.3.0 for pre-release clarity"

---

## Breaking Changes Summary

**For users upgrading from v1.1.0 to v0.3.0:**

1. **Housekeep commands changed:**
   - `mutx housekeep --clean-locks` → `mutx housekeep locks`
   - `mutx housekeep --clean-backups` → `mutx housekeep backups`
   - `mutx housekeep --all` → `mutx housekeep all [DIR]`

2. **Version numbering:**
   - v1.1.0 was incorrectly numbered for pre-release status
   - v0.3.0 signals ongoing development
   - v1.0.0 reserved for first stable release

3. **No migration path needed:**
   - Pre-release software
   - Users expected to adapt to breaking changes

---

## Testing Requirements

### New Tests Needed

1. **Backup suffix tests:**
   - Custom suffix creates correct filename
   - Timestamp with custom suffix
   - Housekeep cleans custom suffix backups

2. **Housekeep subcommand tests:**
   - `locks` defaults to cache directory
   - `backups` defaults to current directory
   - `all` with [DIR] cleans both
   - `all` with --locks-dir and --backups-dir
   - `all` validation errors on invalid combinations

3. **Write subcommand tests:**
   - Implicit: `mutx output.txt`
   - Explicit: `mutx write output.txt`
   - Both produce identical results

4. **Version tests:**
   - `--version` shows "0.3.0"

### Existing Tests to Update

1. All housekeep CLI tests (command syntax changed)
2. Version assertion tests
3. README examples verification

---

## Architecture Notes

### Why `#[implicit_command]` Macro?

**Problem:** Clap doesn't natively support "default subcommand" pattern.

**Solution:** Custom attribute macro that marks a variant. In dispatch logic, when `args.command` is None, treat it as if the implicit command was specified.

**Benefits:**
- Clean, declarative syntax
- No args duplication
- Easy to understand intent
- Future-proof for more implicit commands

**Implementation:** Simple marker attribute. Real logic in `run()` dispatch.

### Why Nested Subcommands for Housekeep?

**Benefits:**
- Clear intent: `housekeep locks` vs `housekeep --clean-locks`
- Smart defaults per operation
- Better help messages
- Extensible for future operations

**Trade-offs:**
- More verbose for `all` case
- Breaking change from v1.1.0

**Decision:** Better UX worth breaking change in pre-release.

---

## Future Considerations

### v1.0.0 Stability Criteria

Before releasing v1.0.0:
- [ ] CLI interface stable and tested
- [ ] Security hardening complete
- [ ] Windows support clarified (experimental or supported)
- [ ] Documentation complete
- [ ] Community feedback incorporated
- [ ] Real-world usage validation

### Potential v0.4.0 Features

- Compression support for large files
- Configurable backup retention policies
- Atomic directory operations
- Transaction log for rollback

