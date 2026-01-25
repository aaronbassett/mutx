# mutx

A command-line tool for atomic file writes with process coordination through file locking.

## Features

- **Atomic writes**: All-or-nothing file updates using atomic rename
- **File locking**: Advisory locks prevent concurrent write conflicts
- **Backup support**: Optional backups with timestamps
- **Streaming mode**: Process large files with constant memory usage
- **Housekeeping**: Clean up orphaned locks and old backups

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
# Basic usage
echo "new content" | mutx config.json

# With backup
echo "new content" | mutx --backup config.json

# Large file streaming
cat large_file.csv | mutx --stream output.csv

# Wait for lock with timeout
generate_config.sh | mutx --timeout 30 config.json

# Fail fast if locked
mutx --no-wait config.json < data.txt
```

## Usage

### Write Command

```
mutx [OPTIONS] <OUTPUT>
```

**Options:**
- `-i, --input <FILE>`: Read from file instead of stdin
- `--stream`: Use streaming mode for large files
- `--no-wait`: Fail immediately if locked (default: wait)
- `-t, --timeout <SECONDS>`: Lock acquisition timeout (implies wait)
- `-b, --backup`: Create backup before overwrite
- `--backup-suffix <SUFFIX>`: Custom backup suffix (default: .backup)
- `--backup-timestamp`: Add timestamp to backup
- `-v`: Verbose output (-vv for debug)

### Housekeep Command

```
mutx housekeep [OPTIONS] [DIR]
```

**Options:**
- `--clean-locks`: Clean orphaned lock files
- `--clean-backups`: Clean old backup files
- `--all`: Clean both locks and backups
- `-r, --recursive`: Scan subdirectories
- `--older-than <DURATION>`: Age threshold (e.g., "2h", "7d")
- `--keep-newest <N>`: Keep N newest backups per file
- `-n, --dry-run`: Show what would be deleted

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

### Lock Cleanup

```bash
# Clean locks older than 1 hour
mutx housekeep --clean-locks --older-than 1h /var/lib/app

# Keep only 3 newest backups
mutx housekeep --clean-backups --keep-newest 3 /data
```

## Exit Codes

- `0`: Success
- `1`: General error (I/O, permission denied, invalid arguments)
- `2`: Lock acquisition failed (timeout or no-wait)
- `3`: Interrupted (SIGINT, SIGTERM)

## Limitations

- **Advisory locks only**: Non-cooperating processes can still write
- **Unix/Linux/macOS only**: Windows support planned for v2.0
- **Lock files orphaned on SIGKILL**: Use housekeep command for cleanup
- **Single backup generation**: Use `--backup-timestamp` for multiple versions

## License

MIT OR Apache-2.0

## Contributing

Contributions welcome! Please open an issue before major changes.
