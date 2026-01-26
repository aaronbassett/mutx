# Changelog

All notable changes to this project will be documented in this file.

The format is based on [Keep a Changelog](https://keepachangelog.com/en/1.0.0/),
and this project adheres to [Semantic Versioning](https://semver.org/spec/v2.0.0.html).

## [1.1.0] - Unreleased

### Security Fixes

- **BREAKING**: Lock files now stored in platform cache directory
  - Linux: `~/.cache/mutx/locks/`
  - macOS: `~/Library/Caches/mutx/locks/`
  - Windows: `%LOCALAPPDATA%\mutx\locks\`
  - Prevents collision when output filename ends in `.lock`
  - Eliminates race condition from lock file deletion
  - Lock files persist for proper mutual exclusion
  - Run `mutx housekeep --locks` to clean orphaned locks
  - Lock filename format: `{initialism}.{parent}.{filename}.{hash}.lock`

- **BREAKING**: Symlinks rejected by default
  - Prevents symlink traversal attacks in housekeep operations
  - Prevents lock file symlink attacks that could clobber arbitrary files
  - Use `--follow-symlinks` to allow symlinks for output files and housekeep
  - Use `--follow-lock-symlinks` to allow symlinks for lock files (security risk)
  - Housekeep skips symlinks by default to prevent directory escape

- **BREAKING**: Backup filename format changed to prevent collisions
  - New format: `{filename}.{YYYYMMDD_HHMMSS}.mutx.backup`
  - Example: `data.txt.20260125_143022.mutx.backup`
  - Timestamp validation ensures only mutx-created backups are cleaned
  - Prevents accidental deletion of user backup files
  - Old backup format no longer recognized by housekeep (manual cleanup if needed)

### Breaking Changes

- `--timeout` now takes milliseconds instead of seconds
  - Old: `mutx write --timeout 5` (5 seconds)
  - New: `mutx write --timeout 5000` (5000 milliseconds)
- Lock file location moved from output directory to platform cache directory
- Symlinks rejected by default for all operations (require explicit flags)
- Backup filename format changed from `.backup` to `.YYYYMMDD_HHMMSS.mutx.backup`
- Default backup suffix changed from `.backup` to `.mutx.backup`

### Improvements

- Lock acquisition uses exponential backoff with jitter
  - Starts at 10ms for low latency on quick lock releases
  - Exponential backoff with 1.5x multiplier reduces CPU usage
  - Random jitter (0-100ms) prevents thundering herd on simultaneous timeouts
- Add `--max-poll-interval` flag to configure maximum timeout polling interval
  - Default: 1000ms
  - Example: `mutx write --timeout 30000 --max-poll-interval 2000`
- Removed unused dependencies (`anyhow`, `libc` as global dep)
- Added `directories` crate for platform-specific cache paths
- Added `rand` crate for timeout jitter
- Added `sha2` crate for lock filename hashing
- Keep `libc` as Unix-only dependency for O_NOFOLLOW

### Bug Fixes

- Fixed lock file collision when output filename ends in `.lock`
- Fixed race condition where deleting lock files breaks mutual exclusion
- Fixed symlink traversal vulnerability in housekeep recursive cleanup
- Fixed backup detection matching unrelated user files with `.backup` in name
- Fixed base filename extraction in housekeep using unreliable string search

### Documentation

- Clarified Windows support status in README (experimental, not actively tested)
- Added security rationale to error messages for symlink rejection
- Documented lock file persistence behavior (no auto-deletion)

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
