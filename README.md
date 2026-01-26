# mutx

A command-line tool for atomic file writes with process coordination through file locking.

## Features

- **Atomic writes**: All-or-nothing file updates using atomic rename
- **File locking**: Advisory locks prevent concurrent write conflicts
- **Backup support**: Optional backups with timestamps
- **Streaming mode**: Process large files with constant memory usage
- **Housekeeping**: Clean up orphaned locks and old backups

## ⚠️ Pre-Release Software

**Current version: 0.3.0 (pre-release)**

This software is under active development. The API and CLI are subject to change
before the v1.0.0 stable release. While the core functionality is production-ready
and well-tested, breaking changes may occur in minor version updates.

Version 1.0.0 will mark the first stable public release with guaranteed backward
compatibility.

## Installation

### Via Homebrew (macOS and Linux)

```bash
brew install aaronbassett/tap/mutx
```

### Via Cargo

```bash
cargo install mutx
```

### From Source

```bash
git clone https://github.com/aaronbassett/mutx
cd mutx
cargo build --release
# Binary will be in target/release/mutx
```

### Pre-built Binaries

Download pre-built binaries for your platform from the [releases page](https://github.com/aaronbassett/mutx/releases)

## Quick Start

```bash
# Basic usage (both forms work)
echo "new content" | mutx config.json
echo "new content" | mutx write config.json

# With backup
echo "new content" | mutx --backup config.json

# Custom backup suffix
echo "new content" | mutx --backup --backup-suffix .bak config.json

# Large file streaming
cat large_file.csv | mutx --stream output.csv

# Wait for lock with timeout (5 seconds)
generate_config.sh | mutx --timeout 5000 config.json

# Fail fast if locked
mutx --no-wait config.json < data.txt
```

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
mutx write output.txt --lock-file /tmp/my-custom.lock
```

Note: Custom lock files are not automatically cleaned by housekeep.

## Security Considerations

### Symlink Handling

By default, mutx rejects symbolic links for security:

```bash
# Reject symlinks (default)
mutx output.txt < input.txt

# Allow symlinks for output files
mutx output.txt --follow-symlinks < input.txt

# Allow symlinks even for lock files (not recommended)
mutx output.txt --follow-lock-symlinks < input.txt
```

Rationale: Following symlinks can lead to:
- Unintended file overwrites in lock file handling
- Directory traversal attacks in housekeeping operations
- Confusion about which file is actually being modified

### Backup Format

Backups use the format `{filename}.{YYYYMMDD_HHMMSS}.mutx.backup` to prevent
accidental deletion of user backup files during housekeeping.

## Usage

### Write Command

```
mutx [OPTIONS] <OUTPUT>
```

**Options:**
- `-i, --input <FILE>`: Read from file instead of stdin
- `--stream`: Use streaming mode for large files
- `--no-wait`: Fail immediately if locked (default: wait)
- `-t, --timeout <MILLISECONDS>`: Lock acquisition timeout (implies wait)
- `--max-poll-interval <MS>`: Maximum poll interval for exponential backoff (default: 1000ms)
- `-b, --backup`: Create backup before overwrite
- `--backup-suffix <SUFFIX>`: Custom backup suffix (default: .mutx.backup)
- `--backup-timestamp`: Add timestamp to backup
- `--follow-symlinks`: Allow symbolic links for output files
- `--follow-lock-symlinks`: Allow symbolic links for lock files (not recommended)
- `-v`: Verbose output (-vv for debug)

### Housekeep Command

```
mutx housekeep <SUBCOMMAND>
```

**Subcommands:**
- `locks [DIR]` - Clean orphaned lock files (default: cache directory)
- `backups [DIR]` - Clean old backup files (default: current directory)
- `all [DIR]` - Clean both locks and backups

**Common Options:**
- `-r, --recursive`: Scan subdirectories
- `--older-than <DURATION>`: Age threshold (e.g., "2h", "7d")
- `--keep-newest <N>`: Keep N newest backups per file (backups only)
- `--suffix <SUFFIX>`: Custom backup suffix to match (backups/all, default: .mutx.backup)
- `--locks-dir <DIR>`: Lock directory (all command only, requires --backups-dir)
- `--backups-dir <DIR>`: Backup directory (all command only, requires --locks-dir)
- `-n, --dry-run`: Show what would be deleted
- `-v, --verbose`: Show detailed output

## Examples

### Configuration File Updates

```bash
# Update JSON config atomically
jq '.database.max_connections = 100' config.json | mutx config.json

# With backup for safety
jq '.setting = "new"' app.json | mutx --backup app.json
```

### Concurrent Cron Jobs

```bash
# Multiple cron jobs writing to same file (automatically waits for lock)
* * * * * process_logs.sh | mutx /var/log/summary.log
* * * * * analyze_metrics.sh | mutx /var/log/summary.log
```

### Large File Processing

```bash
# Stream large CSV without loading into memory
transform_data.py < input.csv | mutx --stream output.csv
```

### Lock and Backup Cleanup

```bash
# Clean locks from cache directory
mutx housekeep locks

# Clean locks older than 1 hour
mutx housekeep locks --older-than 1h

# Clean backups, keep 3 newest per file
mutx housekeep backups --keep-newest 3 /data

# Clean custom backup suffix
mutx housekeep backups --suffix .bak

# Clean both locks (from cache) and backups (from data dir)
mutx housekeep all --locks-dir ~/.cache/mutx/locks --backups-dir /var/lib/app

# Clean both from same directory (only when using custom --lock-file paths)
mutx housekeep all /var/lib/app

# Dry run to see what would be cleaned
mutx housekeep all --dry-run --locks-dir ~/.cache/mutx/locks --backups-dir /var/lib/app
```

## Exit Codes

- `0`: Success
- `1`: General error (I/O, permission denied, invalid arguments)
- `2`: Lock acquisition failed (timeout or no-wait)
- `3`: Interrupted (SIGINT, SIGTERM)

## Platform Support

- **Unix/Linux/macOS**: Fully supported and tested. Primary development platforms.
- **Windows**: Tests pass in CI, but not actively used or tested by maintainers.
  File locking behavior may differ from Unix platforms. Use with caution in production.
  Feedback and bug reports welcome!

Lock files are stored in platform-specific cache directories:
- Linux: `~/.cache/mutx/locks/`
- macOS: `~/Library/Caches/mutx/locks/`
- Windows: `%LOCALAPPDATA%\mutx\locks\`

## Limitations

- **Advisory locks only**: Non-cooperating processes can still write
- **Lock files orphaned on SIGKILL**: Use housekeep command for cleanup
- **Single backup generation**: Use `--backup-timestamp` for multiple versions

## License

MIT OR Apache-2.0

## Contributing

Contributions welcome! Please open an issue before major changes.
