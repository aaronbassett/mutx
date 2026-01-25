# Changelog

All notable changes to this project will be documented in this file.

The format is based on [Keep a Changelog](https://keepachangelog.com/en/1.0.0/),
and this project adheres to [Semantic Versioning](https://semver.org/spec/v2.0.0.html).

## [1.1.0] - 2026-01-25

### Fixed
- **Critical**: Eliminated all unwrap() calls that could cause panics in production
- **Critical**: Fixed race conditions in lock cleanup (TOCTOU vulnerabilities)
- **Critical**: Made backup operations atomic using copy-temp-rename strategy
- Fixed --timeout requiring explicit --wait flag (now implies wait mode)
- Fixed poor error messages for duration parsing
- Fixed 23+ deprecated assert_cmd test patterns
- Fixed redundant closures and unused imports found by clippy

### Changed
- **Breaking**: Removed unused CLI flags: --wait (now default behavior)
- Replaced anyhow with thiserror for structured error types
- Error messages now include context (file paths, durations, etc.)
- Applied rustfmt formatting across entire codebase

### Added
- Structured logging with tracing (use RUST_LOG environment variable)
- Comprehensive error types with proper exit codes
- Early path validation with clear error messages
- Duration parsing utility supporting s/m/h/d units
- CI workflow for GitHub Actions (Linux/macOS/Windows)
- LICENSE-MIT and LICENSE-APACHE files
- Improved README with detailed installation instructions
- Path validation tests for input/output files

### Security
- Fixed TOCTOU race conditions in lock file cleanup
- Eliminated panic paths in Drop implementations
- Added atomic backup operations (no partial files on failure)
- Improved error handling throughout codebase

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
