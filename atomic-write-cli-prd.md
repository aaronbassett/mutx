# Product Requirements Document: atomic-write CLI

## 1. Executive Summary

**Product Name:** `atomic-write` (or `awrite`)

**Version:** 1.0

**Purpose:** A command-line tool that provides atomic file writes with process coordination through file locking, preventing data corruption and lost updates in concurrent environments.

**Target Users:**
- Shell script authors
- DevOps engineers
- System administrators
- CI/CD pipeline developers
- Application developers writing to shared configuration files

**Core Value Proposition:** Eliminate file corruption from incomplete writes, process crashes, and concurrent access while providing a simple, composable Unix tool interface.

---

## 2. Goals and Non-Goals

### Goals
1. Provide atomic file write guarantees (all-or-nothing semantics)
2. Coordinate writes across multiple processes using advisory file locks
3. Support both small files (memory buffered) and large files (streaming)
4. Offer flexible lock acquisition strategies (wait, timeout, fail-fast)
5. Maintain backward compatibility with standard Unix pipe patterns
6. Clean up lock files automatically in normal operation
7. Provide housekeeping utilities for orphaned resources
8. Generate optional backup files before overwriting
9. Support Unix file permissions and ownership preservation

### Non-Goals
1. **Not** a distributed lock manager (no network coordination)
2. **Not** providing mandatory locks (advisory only)
3. **Not** preventing non-cooperating processes from writing
4. **Not** a file versioning system (single backup only)
5. **Not** supporting Windows in v1.0 (Unix/Linux/macOS only)
6. **Not** providing encryption or compression
7. **Not** a general-purpose file manager

---

## 3. User Stories

### Story 1: Configuration File Updates
**As a** DevOps engineer
**I want to** update JSON configuration files atomically
**So that** applications never read partially-written or corrupted configs

```bash
jq '.database.max_connections = 100' config.json | atomic-write config.json
```

### Story 2: Concurrent Cron Jobs
**As a** system administrator
**I want** multiple cron jobs to safely write to the same log file
**So that** entries don't overlap or get lost

```bash
# Multiple crons running
* * * * * process_logs.sh | atomic-write --wait /var/log/summary.log
* * * * * analyze_metrics.sh | atomic-write --wait /var/log/summary.log
```

### Story 3: Large File Processing
**As a** data engineer
**I want to** stream large CSV transformations without loading into memory
**So that** I can process files larger than available RAM

```bash
# Process 10GB CSV file
transform_csv.py < huge_input.csv | atomic-write --stream huge_output.csv
```

### Story 4: Lock File Cleanup
**As a** system administrator
**I want to** clean up orphaned lock files from crashed processes
**So that** my filesystem doesn't accumulate metadata debris

```bash
atomic-write housekeep --clean-locks /var/lib/myapp
```

### Story 5: Backup Before Overwrite
**As a** cautious user
**I want** to create a backup before overwriting important files
**So that** I can recover if something goes wrong

```bash
generate_new_config.sh | atomic-write --backup config.json
```

### Story 6: Lock Contention Handling
**As a** CI/CD pipeline author
**I want** builds to fail fast when config files are locked
**So that** I get immediate feedback rather than hanging

```bash
update_config.sh | atomic-write --no-wait config.json || exit 1
```

---

## 4. Functional Requirements

### 4.1 Core Write Operations

#### FR-1: Atomic File Write
- **MUST** write data to temporary file first
- **MUST** use atomic rename operation to replace target file
- **MUST** sync temporary file to disk before rename
- **MUST** sync parent directory after rename
- **MUST** preserve atomicity guarantee: target file contains either old or new content, never partial

#### FR-2: Input Sources
- **MUST** accept input from stdin by default
- **MUST** accept input from file via `--input FILE` option
- **MUST** support arbitrary input sizes
- **MUST** handle binary and text data identically

#### FR-3: Write Modes

**Simple Mode (default):**
- Buffer entire input in memory
- Write to temp file after EOF
- Suitable for files < 100MB

**Streaming Mode (`--stream`):**
- Stream input directly to temp file as it arrives
- Constant memory usage regardless of file size
- Suitable for large files or continuous streams

### 4.2 File Locking

#### FR-4: Lock File Creation
- **MUST** create lock file at `{target}.lock`
- **MUST** use advisory locks (flock/fcntl)
- **MUST** acquire exclusive lock before writing
- **MUST** support custom lock file path via `--lock-file PATH`

#### FR-5: Lock Acquisition Strategies

**Wait Mode (default `--wait`):**
- Block until lock acquired
- Log "Waiting for lock..." to stderr
- No timeout (wait indefinitely)

**Timeout Mode (`--wait --timeout SECONDS`):**
- Wait up to specified seconds
- Exit with error code 2 if timeout expires
- Log timeout to stderr

**Fail-Fast Mode (`--no-wait`):**
- Attempt lock acquisition once
- Exit immediately with error code 2 if locked
- Log "File locked by another process" to stderr

#### FR-6: Lock File Cleanup
- **MUST** delete lock file on normal exit (Drop trait)
- **MUST** delete lock file on panic/error (Drop trait)
- **MUST** leave lock file on SIGKILL/crash (documented limitation)
- **SHOULD** log lock file cleanup failures to stderr (non-fatal)

### 4.3 Backup Functionality

#### FR-7: Backup Creation
- `--backup`: Create `{target}.backup` before overwrite
- `--backup-suffix SUFFIX`: Use custom suffix (e.g., `.bak`, `.old`)
- `--backup-dir DIR`: Store backups in separate directory
- **MUST** overwrite previous backup (single generation)
- **MUST** use atomic rename for backup creation
- **MUST** skip backup if target doesn't exist

#### FR-8: Backup Timestamping
- `--backup-timestamp`: Add timestamp to backup filename
- Format: `{target}.{YYYYMMDD-HHMMSS}.backup`
- Allows multiple backup generations
- User responsible for cleanup of timestamped backups

### 4.4 Permissions and Ownership

#### FR-9: Permission Preservation
- **MUST** preserve file mode (permissions) from original by default
- `--mode OCTAL`: Set specific Unix permissions (e.g., `0644`)
- `--no-preserve-mode`: Use default umask permissions
- **MUST** apply specified mode if original doesn't exist

#### FR-10: Ownership Preservation (Unix only)
- `--preserve-owner`: Attempt to preserve owner/group from original
- `--try-preserve-owner`: Ignore EPERM errors (non-root users)
- **MUST** document that ownership preservation requires appropriate privileges

### 4.5 Housekeeping Operations

#### FR-11: Lock File Cleanup Command
```bash
atomic-write housekeep --clean-locks [DIR]
```
- Scan directory (recursively with `--recursive`) for `*.lock` files
- Check if lock holder process is alive (flock try-lock test)
- Delete orphaned lock files (lock acquirable = no holder)
- Report deleted lock files to stdout
- `--dry-run`: Show what would be deleted without deleting
- `--older-than HOURS`: Only clean locks older than N hours

#### FR-12: Backup File Cleanup Command
```bash
atomic-write housekeep --clean-backups [DIR]
```
- Scan directory for `*.backup` and timestamped backup files
- `--keep-newest N`: Keep N most recent backups per file
- `--older-than DAYS`: Delete backups older than N days
- Report deleted backups to stdout
- `--dry-run`: Show what would be deleted

#### FR-13: Combined Housekeeping
```bash
atomic-write housekeep --all [DIR]
```
- Run both lock and backup cleanup
- Single command for maintenance scripts

---

## 5. Technical Requirements

### 5.1 Implementation

#### TR-1: Core Dependencies
- **Rust** (stable channel, MSRV: 1.70+)
- **atomic-write-file** crate (0.2+)
- **fs2** crate for file locking
- **clap** (4.x) for CLI parsing
- **anyhow** for error handling

#### TR-2: Platform Support
- Linux (primary target)
- macOS (full support)
- Other Unix (best-effort)
- Windows (future consideration)

#### TR-3: Performance Targets
- Simple mode: < 10ms overhead for files < 1MB
- Streaming mode: < 5% overhead vs direct write
- Lock acquisition: < 1ms when uncontended
- Memory: O(1) in streaming mode, O(n) in simple mode

### 5.2 Error Handling

#### TR-4: Exit Codes
- `0`: Success
- `1`: General error (I/O error, permission denied, invalid arguments)
- `2`: Lock acquisition failed (timeout or no-wait)
- `3`: Interrupted (SIGINT, SIGTERM)

#### TR-5: Error Messages
- **MUST** write errors to stderr
- **MUST** include context (filename, operation)
- **MUST** suggest remediation when possible
- **SHOULD** use consistent format: `Error: {context}: {error}`

Examples:
```
Error: config.json: Permission denied (use sudo or check ownership)
Error: data.json: Lock acquisition timeout after 30s
Error: /readonly/file.txt: Cannot create lock file (read-only filesystem)
```

### 5.3 Logging and Observability

#### TR-6: Verbosity Levels
- Default: Errors only
- `-v, --verbose`: Info messages (lock acquisition, mode selection)
- `-vv`: Debug messages (temp file paths, lock file operations)
- `--quiet`: Suppress all output except errors

#### TR-7: Structured Output
- `--json`: Output structured JSON to stderr for parsing by scripts
```json
{
  "level": "info",
  "message": "Lock acquired",
  "lock_file": "/tmp/config.json.lock",
  "wait_time_ms": 1523
}
```

---

## 6. CLI Interface Design

### 6.1 Command Structure

```
atomic-write [OPTIONS] <OUTPUT>
atomic-write housekeep [HOUSEKEEP-OPTIONS] [DIR]
```

### 6.2 Primary Write Command

```bash
atomic-write [OPTIONS] <OUTPUT>
```

#### Positional Arguments
- `<OUTPUT>`: Target file path (required)

#### Input Options
- `-i, --input <FILE>`: Read from file instead of stdin
- `--stream`: Use streaming mode (constant memory)

#### Locking Options
- `--wait`: Wait for lock (default)
- `--no-wait`: Fail immediately if locked
- `-t, --timeout <SECONDS>`: Wait timeout (requires --wait)
- `--lock-file <PATH>`: Custom lock file location

#### Backup Options
- `-b, --backup`: Create backup before overwrite
- `--backup-suffix <SUFFIX>`: Backup filename suffix (default: `.backup`)
- `--backup-dir <DIR>`: Store backups in directory
- `--backup-timestamp`: Add timestamp to backup filename

#### Permission Options
- `-m, --mode <OCTAL>`: Set file permissions (e.g., 0644)
- `--preserve-mode`: Preserve mode from original (default)
- `--no-preserve-mode`: Use umask default
- `--preserve-owner`: Preserve owner/group (requires privileges)
- `--try-preserve-owner`: Preserve owner, ignore EPERM

#### Output Options
- `-v, --verbose`: Verbose output
- `-vv`: Debug output
- `-q, --quiet`: Suppress non-error output
- `--json`: Structured JSON output

#### Other Options
- `-h, --help`: Show help
- `-V, --version`: Show version

### 6.3 Housekeep Command

```bash
atomic-write housekeep [OPTIONS] [DIR]
```

#### Positional Arguments
- `[DIR]`: Directory to clean (default: current directory)

#### Operation Options
- `--clean-locks`: Clean orphaned lock files
- `--clean-backups`: Clean old backup files
- `--all`: Clean both locks and backups

#### Filter Options
- `-r, --recursive`: Scan subdirectories
- `--older-than <HOURS|DAYS>`: Age threshold
  - Locks: hours (e.g., `--older-than 2h`)
  - Backups: days (e.g., `--older-than 7d`)
- `--keep-newest <N>`: Keep N newest backups per file (backups only)

#### Execution Options
- `-n, --dry-run`: Show what would be deleted
- `-v, --verbose`: Show detailed information
- `--json`: Structured output

### 6.4 Usage Examples

```bash
# Basic usage - stdin to file
echo "data" | atomic-write config.json

# From file
atomic-write --input source.txt output.txt

# Large file streaming
transform_data.py < input.csv | atomic-write --stream output.csv

# Wait for lock with timeout
generate_config.sh | atomic-write --wait --timeout 30 config.json

# Fail fast if locked
atomic-write --no-wait config.json < data.txt

# Create backup before overwrite
atomic-write --backup config.json <<EOF
new configuration
EOF

# Set specific permissions
atomic-write --mode 0600 secrets.json < credentials.txt

# Custom backup location
atomic-write --backup --backup-dir /backups config.json < new_config.txt

# Timestamped backups
atomic-write --backup-timestamp config.json < new_config.txt

# Housekeeping - clean locks older than 1 hour
atomic-write housekeep --clean-locks --older-than 1h /var/lib/app

# Clean old backups, keep 3 newest
atomic-write housekeep --clean-backups --keep-newest 3 --recursive /data

# Dry run to see what would be cleaned
atomic-write housekeep --all --dry-run --recursive /var/lib/app

# JSON output for scripting
atomic-write --json config.json < data.txt 2> result.json
```

---

## 7. Error Scenarios and Handling

### 7.1 Lock Contention

**Scenario:** Another process holds the lock

**Default Behavior (--wait):**
```
$ echo "data" | atomic-write config.json
Waiting for lock on config.json...
[blocks until available]
Lock acquired
```

**With --no-wait:**
```
$ echo "data" | atomic-write --no-wait config.json
Error: config.json: File locked by another process
$ echo $?
2
```

**With --timeout:**
```
$ echo "data" | atomic-write --wait --timeout 5 config.json
Waiting for lock on config.json (5s timeout)...
Error: config.json: Lock acquisition timeout after 5s
$ echo $?
2
```

### 7.2 Permission Errors

**Scenario:** Cannot create lock file

```
$ atomic-write /etc/system/config.json < data.txt
Error: /etc/system/config.json.lock: Permission denied
Hint: Run with sudo or check directory permissions
$ echo $?
1
```

**Scenario:** Cannot write target file

```
$ atomic-write readonly.txt < data.txt
Error: readonly.txt: Permission denied
$ echo $?
1
```

### 7.3 Filesystem Errors

**Scenario:** No space left on device

```
$ echo "data" | atomic-write /mnt/full/file.txt
Error: /mnt/full/file.txt: No space left on device
Hint: Temp file written but cannot be renamed
$ echo $?
1
```

**Scenario:** Read-only filesystem

```
$ atomic-write /mnt/ro/file.txt < data.txt
Error: /mnt/ro/.file.txt.abc123: Read-only file system
$ echo $?
1
```

### 7.4 Input Errors

**Scenario:** Input file doesn't exist

```
$ atomic-write --input missing.txt output.txt
Error: missing.txt: No such file or directory
$ echo $?
1
```

**Scenario:** stdin is closed

```
$ atomic-write output.txt < /dev/null
Error: No input provided
$ echo $?
1
```

### 7.5 Interruption Handling

**Scenario:** SIGINT (Ctrl+C) during operation

```
$ large_stream | atomic-write output.txt
^C
Interrupted
Cleaning up temporary files...
$ echo $?
3
```

**Behavior:**
- Catch SIGINT/SIGTERM
- Delete temporary file (if created)
- Delete lock file (Drop handler)
- Exit with code 3

---

## 8. Security Considerations

### 8.1 Symlink Attacks

**Risk:** Attacker creates symlink at lock file path

**Mitigation:**
- Use `O_NOFOLLOW` when creating lock files
- Fail if lock file is a symlink
- Document that lock files should be in trusted directories

### 8.2 TOCTOU Races

**Risk:** Time-of-check-time-of-use between stat and open

**Mitigation:**
- Use atomic operations (O_CREAT | O_EXCL)
- Rely on atomic-write-file crate's mitigations
- Document that target directory must be trusted

### 8.3 Temporary File Exposure

**Risk:** Sensitive data in temporary files readable by others

**Mitigation:**
- Temporary files inherit target file permissions
- With `--mode`, apply to temp file immediately
- Document that temp files are visible (atomic guarantee > stealth)

### 8.4 Lock File Permissions

**Risk:** Lock file created with overly permissive mode

**Mitigation:**
- Create lock files with mode 0644 (or restrictive umask)
- Allow `--lock-mode` to set lock file permissions separately
- Document lock file visibility

### 8.5 Backup Data Leakage

**Risk:** Backups contain sensitive data with wrong permissions

**Mitigation:**
- Backups inherit original file permissions
- Document that backups should be secured independently
- Provide `--backup-mode` for explicit backup permissions

---

## 9. Performance Requirements

### 9.1 Throughput Targets

| File Size | Mode | Overhead vs `>` | Throughput |
|-----------|------|-----------------|------------|
| < 1 KB | Simple | < 5ms | N/A |
| 1 MB | Simple | < 10ms | > 100 MB/s |
| 100 MB | Streaming | < 5% | > 500 MB/s |
| 10 GB | Streaming | < 5% | > 500 MB/s |

### 9.2 Lock Performance

- Uncontended lock acquisition: < 1ms
- Lock release on drop: < 1ms
- Lock file cleanup: < 5ms per file

### 9.3 Memory Usage

| Mode | Input Size | Memory Usage |
|------|------------|--------------|
| Simple | N bytes | O(N) |
| Streaming | N bytes | O(1) - buffer size ~64KB |

### 9.4 Scalability

- Housekeep command: Handle 10,000+ lock files in < 5 seconds
- Housekeep command: Handle 100,000+ backups in < 30 seconds

---

## 10. Testing Strategy

### 10.1 Unit Tests

- Lock acquisition/release
- Drop handler cleanup verification
- Permission preservation logic
- Backup file creation
- Error message generation
- CLI argument parsing

### 10.2 Integration Tests

#### Basic Functionality
- Simple mode write (stdin)
- Simple mode write (file input)
- Streaming mode write
- Permission preservation
- Backup creation

#### Locking
- Wait mode (multiple processes)
- No-wait mode (fail on contention)
- Timeout mode (expire after N seconds)
- Lock file cleanup on normal exit
- Lock file cleanup on panic

#### Housekeeping
- Clean orphaned locks
- Clean old backups
- Dry-run mode
- Recursive scanning

### 10.3 Crash Tests

Simulate crashes and verify:
- Target file unchanged (if crash before commit)
- Target file fully written (if crash after commit)
- Lock files orphaned (documented behavior)
- Temporary files cleaned up where possible

Use Linux-specific crash testing techniques:
- SIGKILL during write
- Forced kernel panic simulation
- Power loss simulation (sync + crash)

### 10.4 Concurrent Tests

- Multiple processes writing to same file (with locks)
- Lock contention with wait/timeout
- Race condition detection
- Stress test with 100+ concurrent writers

### 10.5 Property-Based Tests

Use proptest or similar:
- Any input produces valid output or error
- File is never partially written
- Lock acquisition is mutually exclusive
- Cleanup is idempotent

---

## 11. Documentation Requirements

### 11.1 Man Page

Comprehensive man page covering:
- Synopsis
- Description
- Options (all flags)
- Examples
- Exit codes
- Files (lock file locations)
- Security considerations
- Limitations (advisory locks, crash behavior)

Format: `man atomic-write`

### 11.2 README

GitHub README with:
- Installation instructions
- Quick start examples
- Use case scenarios
- Comparison with other tools (sponge, tee, etc.)
- FAQ
- Contributing guidelines

### 11.3 Technical Design Document

Architecture documentation:
- Lock file strategy rationale
- Inode replacement behavior
- Drop trait implementation
- Platform-specific considerations
- Performance characteristics

### 11.4 Tutorial

Step-by-step guides:
- Basic file updates
- Shell script integration
- Cron job coordination
- Large file processing
- Backup strategies
- Housekeeping automation

### 11.5 Troubleshooting Guide

Common issues and solutions:
- Permission denied errors
- Orphaned lock files
- Lock contention debugging
- Performance tuning
- Crash recovery

---

## 12. Compatibility and Interoperability

### 12.1 Shell Compatibility

- **MUST** work with bash, zsh, sh (POSIX)
- **SHOULD** work with fish, tcsh
- **MUST** handle signals correctly (SIGINT, SIGTERM, SIGPIPE)
- **MUST** respect standard file descriptors (stdin=0, stdout=1, stderr=2)

### 12.2 Tool Compatibility

- **MUST** compose with standard Unix tools (grep, awk, sed, jq)
- **MUST** work in pipelines
- **MUST** handle binary data (no text assumptions)
- **SHOULD** integrate with common tools:
  - `jq` for JSON manipulation
  - `yq` for YAML manipulation
  - `xmlstarlet` for XML
  - `curl` for downloads

### 12.3 Process Managers

Document compatibility with:
- systemd
- supervisor
- Docker containers
- Kubernetes pods
- Cron
- systemd timers

---

## 13. Packaging and Distribution

### 13.1 Binary Distribution

- Prebuilt binaries for:
  - Linux x86_64 (glibc 2.31+)
  - Linux aarch64
  - macOS x86_64 (10.15+)
  - macOS aarch64 (11.0+)
- Available via:
  - GitHub Releases
  - Homebrew (macOS)
  - APT repository (Debian/Ubuntu)
  - YUM/DNF repository (RHEL/Fedora)
  - AUR (Arch Linux)

### 13.2 Source Distribution

- crates.io for Rust users: `cargo install atomic-write`
- Source tarballs on GitHub Releases
- Git repository tags

### 13.3 Container Images

- Docker image: `ghcr.io/yourorg/atomic-write:latest`
- Alpine-based (minimal size)
- Scratch-based (security-focused)

---

## 14. Versioning and Releases

### 14.1 Semantic Versioning

- **MAJOR**: Breaking CLI changes
- **MINOR**: New features, backward compatible
- **PATCH**: Bug fixes, performance improvements

### 14.2 Stability Guarantees

- CLI interface: Stable after 1.0
- Config file format: Stable after 1.0
- Lock file format: Implementation detail (not guaranteed)
- Error message format: Best effort (may improve)

---

## 15. Future Enhancements (Out of Scope for 1.0)

### 15.1 v1.1 Candidates
- Windows support
- Config file for default options (~/.atomic-write.toml)
- Shell completion (bash, zsh, fish)
- Progress bars for large files

### 15.2 v2.0 Candidates
- Distributed locking (Redis, etcd)
- Multi-generation backups (rotation policies)
- Compression on-the-fly
- Encryption at rest
- Checksums/integrity verification
- Transaction logs

### 15.3 Ecosystem Integration
- Git hooks integration
- Pre-commit hooks
- GitHub Actions
- GitLab CI
- Ansible module

---

## 16. Success Metrics

### 16.1 Adoption Metrics
- Downloads per month (target: 10k in first 6 months)
- GitHub stars (target: 500 in first year)
- crates.io downloads (target: 5k in first year)

### 16.2 Quality Metrics
- Test coverage > 80%
- Zero critical bugs in first 3 months
- Response time to issues < 48 hours
- Documentation completeness score > 90%

### 16.3 Performance Benchmarks
- No performance regressions > 5% between releases
- Memory usage within specified targets
- Lock overhead < 1ms in 99th percentile

---

## 17. Open Questions

1. **Lock file location strategy**: Should we support XDG_RUNTIME_DIR for lock files?
2. **Backup rotation**: Should we include built-in rotation policies or leave to external tools?
3. **Verification**: Should we add `--verify` to compare input hash with written output hash?
4. **Diff mode**: Should we have `--diff` to show what changed before overwriting?
5. **Merge strategy**: Should we support `--merge` for concurrent updates (e.g., append-only logs)?

---

## 18. Dependencies and Risks

### 18.1 Dependencies
- atomic-write-file crate maintenance
- fs2 crate compatibility
- Rust stability (MSRV policy)

### 18.2 Technical Risks
- Platform-specific filesystem bugs (fsync reliability)
- Advisory lock limitations (non-cooperating processes)
- Drop trait execution guarantees (SIGKILL, crashes)

### 18.3 Mitigation Strategies
- Extensive testing across platforms
- Clear documentation of limitations
- Graceful degradation where possible
- Community engagement for issue discovery

---

## 19. Timeline (Indicative)

- **Week 1-2**: Core implementation (write, lock, basic CLI)
- **Week 3**: Backup functionality
- **Week 4**: Housekeeping commands
- **Week 5-6**: Testing (unit, integration, crash tests)
- **Week 7**: Documentation (man page, README, tutorials)
- **Week 8**: Packaging and release preparation
- **Week 9**: Beta release and community feedback
- **Week 10**: 1.0 release

---

## 20. Appendices

### Appendix A: Comparison with Existing Tools

| Tool | Atomic | Locking | Streaming | Backup | Housekeeping |
|------|--------|---------|-----------|--------|--------------|
| `>` redirect | ✗ | ✗ | ✓ | ✗ | N/A |
| `sponge` | ✗* | ✗ | ✗ | ✗ | N/A |
| `tee` | ✗ | ✗ | ✓ | ✗ | N/A |
| **atomic-write** | ✓ | ✓ | ✓ | ✓ | ✓ |

\* sponge buffers but doesn't use atomic rename

### Appendix B: Lock File Format

Lock files are empty regular files. The lock is held via advisory flock/fcntl on the file descriptor, not file contents.

### Appendix C: Glossary

- **Advisory Lock**: Cooperative lock mechanism; only works if all processes check
- **Atomic Operation**: All-or-nothing operation (no partial completion)
- **Inode**: Filesystem data structure representing a file
- **fsync**: System call to flush file buffers to disk
- **Drop Trait**: Rust destructor mechanism for cleanup
- **Temporary File**: Hidden file created during write operation (e.g., `.file.abc123`)

---

**Document Version:** 1.0
**Last Updated:** 2026-01-24
**Author:** Product Team
**Status:** Draft for Review
