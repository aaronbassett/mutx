# Changelog

All notable changes to this project will be documented in this file.

The format is based on [Keep a Changelog](https://keepachangelog.com/en/1.0.0/),
and this project adheres to [Semantic Versioning](https://semver.org/spec/v2.0.0.html).

## [Unreleased]

## [0.3.0] - 2026-01-30

**Note:** Version numbers rolled back from v1.1.0 to v0.3.0 to better signal
pre-release status. Version 1.0.0 is reserved for the first stable public release.

### Added

- Explicit `mutx write` subcommand (implicit `mutx` still works for backward compatibility)
- Housekeep subcommands: `locks`, `backups`, `all`
- `--suffix` flag for `housekeep backups` to clean custom backup patterns
- `--locks-dir` and `--backups-dir` flags for `housekeep all` for targeted cleanup

### Fixed

- Backup suffix validation now occurs before lock acquisition in write command
- Incorrect help text for `--follow-symlinks` flag corrected
- Misleading `housekeep all` examples in README clarified
- **CRITICAL:** `--backup-suffix` now functional (was silently ignored in v1.1.0)
- Housekeep locks defaults to cache directory instead of current directory
- README examples corrected to match actual CLI syntax
- Lock file collision when output filename ends in `.lock`
- Race condition where deleting lock files breaks mutual exclusion
- Symlink traversal vulnerability in housekeep recursive cleanup
- Backup detection matching unrelated user files with `.backup` in name
- Base filename extraction in housekeep using unreliable string search

### Changed

- **BREAKING:** Housekeep now uses subcommands instead of flags:
  - `mutx housekeep --clean-locks` → `mutx housekeep locks`
  - `mutx housekeep --clean-backups` → `mutx housekeep backups`
  - `mutx housekeep --all` → `mutx housekeep all [DIR]`
- **BREAKING:** `--timeout` now takes milliseconds instead of seconds
  - Old: `mutx write --timeout 5` (5 seconds)
  - New: `mutx write --timeout 5000` (5000 milliseconds)
- **BREAKING:** Lock file location moved from output directory to platform cache directory
  - Linux: `~/.cache/mutx/locks/`
  - macOS: `~/Library/Caches/mutx/locks/`
  - Windows: `%LOCALAPPDATA%\mutx\locks\`
  - Prevents collision when output filename ends in `.lock`
  - Eliminates race condition from lock file deletion
  - Lock files persist for proper mutual exclusion
  - Run `mutx housekeep locks` to clean orphaned locks
  - Lock filename format: `{initialism}.{parent}.{filename}.{hash}.lock`
- **BREAKING:** Symlinks rejected by default
  - Prevents symlink traversal attacks in housekeep operations
  - Prevents lock file symlink attacks that could clobber arbitrary files
  - Use `--follow-symlinks` to allow symlinks for output files and housekeep
  - Use `--follow-lock-symlinks` to allow symlinks for lock files (security risk)
  - Housekeep skips symlinks by default to prevent directory escape
- **BREAKING:** Backup filename format changed to prevent collisions
  - New format: `{filename}.{YYYYMMDD_HHMMSS}.mutx.backup`
  - Example: `data.txt.20260125_143022.mutx.backup`
  - Timestamp validation ensures only mutx-created backups are cleaned
  - Prevents accidental deletion of user backup files
  - Old backup format no longer recognized by housekeep (manual cleanup if needed)
- **BREAKING:** Default backup suffix changed from `.backup` to `.mutx.backup`
- Version numbering strategy: v0.x for pre-release, v1.0.0+ for stable
- Removed unused `mutx-macros` workspace dependency

### Security

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

### Documentation

- Clarified Windows support status in README (experimental, not actively tested)
- Added security rationale to error messages for symlink rejection
- Documented lock file persistence behavior (no auto-deletion)

## [1.1.0] - 2026-01-26 (Superseded by 0.3.0)

**Note:** This version was released but immediately superseded by v0.3.0 to correct
version numbering strategy. All features from this release are included in v0.3.0.

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

[Unreleased]: https://github.com/aaronbassett/mutx/compare/v0.3.0...HEAD
[0.3.0]: https://github.com/aaronbassett/mutx/compare/v1.1.0...v0.3.0
[1.1.0]: https://github.com/aaronbassett/mutx/compare/v1.0.0...v1.1.0
[1.0.0]: https://github.com/aaronbassett/mutx/releases/tag/v1.0.0
