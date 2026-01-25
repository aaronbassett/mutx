# Security Hardening Design

**Date:** 2026-01-25
**Status:** Approved
**Target Version:** 1.1.0 (unreleased) → 1.2.0 (first public release)

## Overview

This design addresses critical security vulnerabilities and production reliability issues discovered during pre-release review. Since the project has not been publicly released, we can make breaking changes to ensure a secure foundation.

## Issues Addressed

### Critical Issues
1. **Lock file collision**: Default lock path can equal output path when output ends in `.lock`, causing data loss via truncation
2. **Lock file race condition**: Deleting lock files on drop breaks mutual exclusion when processes are waiting

### High Issues
3. **Symlink traversal in housekeep**: Recursive cleanup follows symlinks, allowing directory escape and deletion loops
4. **Symlink attack on lock files**: Lock creation follows symlinks and truncates targets

### Medium Issues
5. **Backup detection too broad**: Uses substring matching, risking deletion of unrelated files
6. **Version inconsistency**: Cargo.toml and CHANGELOG.md version mismatch

### Low Issues
7. **Unused dependencies**: `anyhow` and `libc` inflating dependency surface
8. **Inefficient timeout loop**: Fixed 100ms polling wastes CPU on long timeouts
9. **Windows support unclear**: Documentation claims Unix-only but CI tests Windows

## Design Solutions

### 1. Lock File Architecture

**Problem:** Lock files created next to output files cause collision and race conditions.

**Solution:** Move lock files to platform-specific cache directories.

**Lock file locations:**
- Linux: `~/.cache/mutx/locks/`
- macOS: `~/Library/Caches/mutx/locks/`
- Windows: `%LOCALAPPDATA%\mutx\locks\`

**Filename derivation:**

For output path `/absolute/path/to/output.txt`:
```
a.p.to.output.txt.a3f2d9e8.lock
├─┬──┬──────────────┬────────┘
│ │  │              └─ first 8 chars of SHA256(canonical_path)
│ │  └─ full parent directory name
│ └─ initialism of ancestor dirs (excluding parent)
└─ original filename
```

**Algorithm:**
1. Resolve output path to canonical absolute path
2. Extract components:
   - All directory names in path
   - Take first letter of each ancestor dir (skip parent)
   - Keep full parent directory name
   - Keep original filename
3. Compute SHA256 hash of canonical path, take first 8 hex chars
4. Format: `{initialism}.{parent}.{filename}.{hash}.lock`

**Lock lifecycle:**
- **Creation**: Always created in cache directory
- **Persistence**: Never deleted automatically on drop
- **Cleanup**: User runs `mutx housekeep --locks` to clean orphaned locks
- **Drop behavior**: Only releases file lock (closes file handle), leaves file on disk

**Custom lock paths:**
- Users can override with `--lock-file /custom/path`
- Validation ensures `lock_path ≠ output_path` to prevent collision
- No automatic `.lock` suffix appending to custom paths

**Implementation:**
- Use `directories` crate for platform-specific paths
- Create lock directory if it doesn't exist
- Atomic directory creation (handle race with other processes)

### 2. Symlink Security

**Problem:** Following symlinks everywhere creates security vulnerabilities.

**Solution:** Reject symlinks by default, with explicit opt-in flags.

**Default behavior (secure):**

All symlinks rejected with clear errors:
- Lock file is symlink → "Lock path is a symlink. Use --follow-lock-symlinks if intentional."
- Output file is symlink → "Output file is a symlink. Use --follow-symlinks to allow."
- Housekeep encounters symlink → Skip silently (don't traverse, don't delete)

**Opt-in flags:**

```bash
--follow-symlinks          # Allow symlinks for output files and housekeep
                          # Lock files still protected

--follow-lock-symlinks    # Allow symlinks even for lock files
                          # Implies --follow-symlinks
                          # WARNING in help text about security risk
```

**Implementation details:**

**Lock file creation:**
- Unix: Use `O_NOFOLLOW` via `OpenOptionsExt::custom_flags(libc::O_NOFOLLOW)`
- Windows: Open file, then check `metadata.file_type().is_symlink()`, error if true
- Error before truncation to prevent damage

**Output file validation:**
- Check `path.symlink_metadata()?.file_type().is_symlink()` before acquiring lock
- Skip check if `--follow-symlinks` enabled

**Housekeep traversal:**
- Use `entry.file_type()` instead of `path.is_file()` or `path.is_dir()`
- Check `file_type.is_symlink()` and skip if true (unless `--follow-symlinks`)
- Never traverse into symlinked directories by default

**Path canonicalization:**
- Resolve canonical paths BEFORE symlink checks
- Lock filename hash based on real path (not symlink path)
- Ensures same lock for all symlinks pointing to same file (if following enabled)

### 3. Backup Format and Detection

**Problem:** Broad substring matching risks deleting unrelated files.

**Solution:** Strict filename format with validated timestamp parsing.

**New backup format:**
```
{original_filename}.{YYYYMMDD_HHMMSS}.mutx.backup
```

**Examples:**
- `data.txt` → `data.txt.20260125_143022.mutx.backup`
- `config.json` → `config.json.20260125_143022.mutx.backup`

**Detection logic:**

```rust
fn is_backup_file(path: &Path) -> bool {
    path.file_name()
        .and_then(|s| s.to_str())
        .map(|name| name.ends_with(".mutx.backup"))
        .unwrap_or(false)
}

fn extract_base_filename(path: &Path) -> Option<String> {
    let name = path.file_name()?.to_str()?;

    // Must end with .mutx.backup
    let without_suffix = name.strip_suffix(".mutx.backup")?;

    // Split to get timestamp part
    let parts: Vec<&str> = without_suffix.rsplitn(2, '.').collect();
    if parts.len() != 2 {
        return None;
    }

    let timestamp = parts[0];
    let base = parts[1];

    // Validate timestamp format: YYYYMMDD_HHMMSS
    if timestamp.len() != 15 {
        return None;
    }

    if timestamp.chars().nth(8) != Some('_') {
        return None;
    }

    let date_part = &timestamp[..8];
    let time_part = &timestamp[9..];

    if !date_part.chars().all(|c| c.is_ascii_digit()) {
        return None;
    }

    if !time_part.chars().all(|c| c.is_ascii_digit()) {
        return None;
    }

    Some(base.to_string())
}
```

**Rationale:**
- `.mutx.backup` suffix makes collisions with user files virtually impossible
- Timestamp validation ensures only mutx-created backups are matched
- User files with similar names (`.backup`, `.20260125`, etc.) are safe

**Migration:**
- Existing backups with old format won't be recognized (safe)
- Users can run manual cleanup if desired
- Document change in CHANGELOG as breaking change

### 4. Lock Acquisition Timeout with Exponential Backoff

**Problem:** Fixed 100ms polling is inefficient and inflexible.

**Solution:** Exponential backoff with jitter and configurable maximum interval.

**Algorithm:**
```
Initial interval: 10ms
Backoff multiplier: 1.5
Maximum interval: configurable (default 1000ms)
Jitter: random 0-100ms added to each sleep
```

**Progression example:**
```
Attempt 1: sleep 10ms + rand(0..100)ms
Attempt 2: sleep 15ms + rand(0..100)ms
Attempt 3: sleep 22ms + rand(0..100)ms
Attempt 4: sleep 33ms + rand(0..100)ms
...
Attempt N: sleep min(current * 1.5, 1000ms) + rand(0..100)ms
```

**Data structures:**

```rust
pub struct TimeoutConfig {
    pub duration: Duration,           // total timeout duration
    pub max_poll_interval: Duration,  // default: 1000ms
}

pub enum LockStrategy {
    Wait,
    NoWait,
    Timeout(TimeoutConfig),
}
```

**CLI changes:**

```bash
--timeout <MS>                # Timeout in milliseconds (BREAKING: was seconds)
--max-poll-interval <MS>      # Max poll interval in ms (default: 1000)
```

**Examples:**
```bash
# Wait up to 5 seconds
mutx write --timeout 5000 output.txt

# Wait up to 30 seconds with max 2s poll interval
mutx write --timeout 30000 --max-poll-interval 2000 output.txt
```

**Implementation:**

```rust
let start = Instant::now();
let mut current_interval = Duration::from_millis(10);
let mut rng = rand::thread_rng();

loop {
    match file.try_lock_exclusive() {
        Ok(_) => break,
        Err(e) if e.kind() == io::ErrorKind::WouldBlock => {
            if start.elapsed() >= config.duration {
                return Err(MutxError::LockTimeout { ... });
            }

            // Calculate sleep time with backoff + jitter
            let base_interval = current_interval.min(config.max_poll_interval);
            let jitter = Duration::from_millis(rng.gen_range(0..100));
            let sleep_time = base_interval + jitter;

            std::thread::sleep(sleep_time);

            // Exponential backoff for next iteration
            current_interval = Duration::from_millis(
                (current_interval.as_millis() as f64 * 1.5) as u64
            );
        }
        Err(e) => return Err(MutxError::LockAcquisitionFailed { ... }),
    }
}
```

**Rationale:**
- Jitter prevents thundering herd when multiple processes timeout simultaneously
- Exponential backoff reduces CPU usage for long timeouts
- Configurable max interval allows tuning for different use cases
- Fast initial polling (10ms) ensures low latency for quick lock releases

### 5. Dependency Cleanup and Documentation

**Remove unused dependencies:**
```toml
# DELETE from Cargo.toml
anyhow = "1.0"    # Replaced by thiserror
libc = "0.2"      # Unused, will use std::os::unix instead
```

**Add new dependencies:**
```toml
rand = "0.8"           # For jitter in timeout loop
directories = "5.0"    # For platform-specific cache directories
```

**Versioning strategy:**
1. Current work: Bump `Cargo.toml` to `1.1.0`
2. Consolidate security fixes under `1.1.0 (unreleased)` in CHANGELOG
3. Before first public release: Bump to `1.2.0` and mark as released

**Windows documentation (README.md):**

```markdown
## Platform Support

- **Unix/Linux/macOS**: Fully supported and tested
- **Windows**: Tests pass in CI, but not actively used by maintainers.
  File locking behavior may differ. Use with caution in production.
  Feedback and bug reports welcome!
```

**CHANGELOG.md additions:**

```markdown
## [1.1.0] - Unreleased

### Security Fixes

- **BREAKING**: Lock files now stored in platform cache directory
  - Prevents collision when output filename ends in `.lock`
  - Eliminates race condition from lock file deletion
  - Lock files persist for proper mutual exclusion
  - Run `mutx housekeep --locks` to clean orphaned locks

- **BREAKING**: Symlinks rejected by default
  - Prevents symlink traversal attacks in housekeep
  - Prevents lock file symlink attacks
  - Use `--follow-symlinks` to allow (for output files only)
  - Use `--follow-lock-symlinks` to allow for lock files (unsafe)

- **BREAKING**: Backup filename format changed
  - New format: `.YYYYMMDD_HHMMSS.mutx.backup`
  - Prevents accidental deletion of user files
  - Old backups won't be automatically cleaned

### Breaking Changes

- `--timeout` now takes milliseconds instead of seconds
- Lock file location moved to cache directory
- Symlinks rejected by default
- Backup filename format changed

### Improvements

- Lock acquisition uses exponential backoff with jitter
- Add `--max-poll-interval` to configure timeout polling
- Removed unused dependencies (`anyhow`, `libc`)
- Improved Windows platform documentation

### Bug Fixes

- Fixed lock file collision when output ends in `.lock`
- Fixed race condition in lock file cleanup
- Fixed symlink traversal vulnerability in housekeep
- Fixed backup detection matching unrelated files
```

## Implementation Considerations

### Error Messages

Provide clear, actionable errors:

```rust
// Symlink rejection
"Output file '{path}' is a symbolic link.\n\
 Use --follow-symlinks to write to symlinks.\n\
 This is disabled by default for security."

// Lock collision (should be impossible with cache dir, but validate anyway)
"Lock file path cannot equal output file path.\n\
 Lock: {lock_path}\n\
 Output: {output_path}\n\
 Specify a different path with --lock-file."

// Invalid backup format during housekeep
"Skipping file '{path}': not a valid mutx backup.\n\
 Expected format: filename.YYYYMMDD_HHMMSS.mutx.backup"
```

### Testing Strategy

**New test cases needed:**

1. Lock file location and naming
   - Verify cache directory creation
   - Verify filename format with various paths
   - Verify hash collision resistance
   - Test custom `--lock-file` paths

2. Symlink security
   - Verify rejection of symlinked output files
   - Verify rejection of symlinked lock files
   - Verify housekeep skips symlinks
   - Test `--follow-symlinks` flag behavior
   - Test `--follow-lock-symlinks` flag behavior

3. Backup format
   - Verify new backup filename format
   - Verify detection only matches mutx backups
   - Verify old-format backups are ignored
   - Test base filename extraction with edge cases

4. Timeout backoff
   - Verify exponential backoff progression
   - Verify jitter is applied
   - Verify max interval is respected
   - Test `--max-poll-interval` configuration

5. Lock persistence
   - Verify lock files persist after drop
   - Verify housekeep can clean orphaned locks
   - Verify multiple processes can wait on same lock

### Migration Guide for Users

Since this is pre-release, no migration needed. For documentation:

**What changed:**
- Lock files now hidden in cache directory instead of next to output files
- Symlinks require explicit opt-in via flags
- Backup files have new naming format
- `--timeout` now uses milliseconds

**What users need to do:**
- Nothing! Changes are automatic
- Old lock files in project directories can be safely deleted
- Old backup files won't be auto-cleaned (safe)
- If using `--timeout`, multiply values by 1000 (seconds → milliseconds)

### Performance Impact

**Lock file location:**
- Slightly slower: extra directory creation, longer path resolution
- Impact: negligible (microseconds)

**Symlink checking:**
- Cost: one extra `symlink_metadata()` call per operation
- Impact: negligible (microseconds)

**Exponential backoff:**
- Benefit: reduces CPU usage during lock contention
- Cost: slightly longer average wait time due to jitter
- Impact: net positive for system load

**Backup detection:**
- Cost: stricter string parsing and validation
- Impact: negligible (only during housekeep)

All performance impacts are negligible compared to I/O operations.

## Risks and Mitigations

### Risk: Cache directory permissions issues
- **Impact**: Lock creation fails on systems with restrictive cache permissions
- **Mitigation**: Clear error message with fallback suggestion to use `--lock-file`
- **Likelihood**: Low (cache directories typically user-writable)

### Risk: Hash collision in lock filenames
- **Impact**: Two different files share same lock
- **Mitigation**: SHA256 provides 2^32 collision resistance with 8 chars (sufficient for single user)
- **Likelihood**: Extremely low (astronomical number of files needed)

### Risk: Breaking changes impact non-existent users
- **Impact**: None (not publicly released)
- **Mitigation**: N/A
- **Likelihood**: N/A

### Risk: Platform differences in cache directory behavior
- **Impact**: Lock files cleaned unexpectedly on some systems
- **Mitigation**: Document in README, use `directories` crate (battle-tested)
- **Likelihood**: Low (cache dirs designed for persistence)

## Success Criteria

- [ ] All critical and high security issues resolved
- [ ] Test coverage for new security features
- [ ] Clear error messages for security violations
- [ ] Documentation updated with breaking changes
- [ ] CHANGELOG.md complete for 1.1.0
- [ ] Version bumped to 1.1.0 in Cargo.toml
- [ ] All tests passing on Linux, macOS, Windows
- [ ] Clippy clean with no warnings
- [ ] Ready for 1.2.0 public release

## Timeline

**Phase 1: Core security (Critical + High issues)**
1. Implement lock file cache directory storage
2. Implement symlink rejection
3. Update tests

**Phase 2: Reliability improvements (Medium issues)**
4. Implement backup format changes
5. Implement exponential backoff
6. Update documentation

**Phase 3: Cleanup (Low issues)**
7. Remove unused dependencies
8. Update README platform section
9. Finalize CHANGELOG

Estimated effort: 1-2 days for implementation + testing

## Future Considerations

**Not in scope for 1.1.0/1.2.0:**

- Configurable lock timeout strategy (e.g., exponential vs linear)
- Lock file encryption for sensitive environments
- Distributed lock coordination across network filesystems
- Backup compression
- Alternative backup storage backends

These can be considered for future releases based on user feedback.
