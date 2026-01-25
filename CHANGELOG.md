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
