use clap::{Parser, Subcommand};
use std::path::PathBuf;

#[derive(Parser, Debug)]
#[command(
    name = "mutx",
    version,
    about = "Atomic file writes with process coordination through file locking",
    long_about = None
)]
pub struct Args {
    #[command(subcommand)]
    pub command: Option<Command>,

    /// Target file path (required if no subcommand)
    #[arg(value_name = "OUTPUT")]
    pub output: Option<PathBuf>,

    /// Read from file instead of stdin
    #[arg(short, long, value_name = "FILE")]
    pub input: Option<PathBuf>,

    /// Use streaming mode (constant memory)
    #[arg(long)]
    pub stream: bool,

    /// Wait for lock (default)
    #[arg(long, conflicts_with = "no_wait")]
    pub wait: bool,

    /// Fail immediately if locked
    #[arg(long, conflicts_with = "wait")]
    pub no_wait: bool,

    /// Wait timeout in seconds (requires --wait)
    #[arg(short = 't', long, value_name = "SECONDS", requires = "wait")]
    pub timeout: Option<u64>,

    /// Custom lock file location
    #[arg(long, value_name = "PATH")]
    pub lock_file: Option<PathBuf>,

    /// Create backup before overwrite
    #[arg(short = 'b', long)]
    pub backup: bool,

    /// Backup filename suffix
    #[arg(long, value_name = "SUFFIX", default_value = ".backup", requires = "backup")]
    pub backup_suffix: String,

    /// Store backups in directory
    #[arg(long, value_name = "DIR", requires = "backup")]
    pub backup_dir: Option<PathBuf>,

    /// Add timestamp to backup filename
    #[arg(long, requires = "backup")]
    pub backup_timestamp: bool,

    /// Set file permissions (octal, e.g., 0644)
    #[arg(short = 'm', long, value_name = "OCTAL")]
    pub mode: Option<String>,

    /// Use umask default permissions instead of preserving
    #[arg(long)]
    pub no_preserve_mode: bool,

    /// Preserve owner/group (requires privileges)
    #[arg(long)]
    pub preserve_owner: bool,

    /// Preserve owner, ignore EPERM errors
    #[arg(long, conflicts_with = "preserve_owner")]
    pub try_preserve_owner: bool,

    /// Verbose output
    #[arg(short = 'v', action = clap::ArgAction::Count)]
    pub verbose: u8,

    /// Suppress non-error output
    #[arg(short = 'q', long, conflicts_with = "verbose")]
    pub quiet: bool,

    /// Structured JSON output
    #[arg(long)]
    pub json: bool,
}

#[derive(Subcommand, Debug)]
pub enum Command {
    /// Housekeeping operations
    Housekeep {
        /// Directory to clean (default: current directory)
        #[arg(value_name = "DIR")]
        dir: Option<PathBuf>,

        /// Clean orphaned lock files
        #[arg(long)]
        clean_locks: bool,

        /// Clean old backup files
        #[arg(long)]
        clean_backups: bool,

        /// Clean both locks and backups
        #[arg(long)]
        all: bool,

        /// Scan subdirectories
        #[arg(short = 'r', long)]
        recursive: bool,

        /// Age threshold (e.g., "2h" for locks, "7d" for backups)
        #[arg(long, value_name = "DURATION")]
        older_than: Option<String>,

        /// Keep N newest backups per file (backups only)
        #[arg(long, value_name = "N")]
        keep_newest: Option<usize>,

        /// Show what would be deleted without deleting
        #[arg(short = 'n', long)]
        dry_run: bool,

        /// Verbose output
        #[arg(short = 'v', long)]
        verbose: bool,

        /// Structured JSON output
        #[arg(long)]
        json: bool,
    },
}
