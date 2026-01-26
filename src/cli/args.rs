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

    /// Fail immediately if locked (default: wait)
    #[arg(long)]
    pub no_wait: bool,

    /// Wait timeout in milliseconds (implies wait mode)
    #[arg(short = 't', long, value_name = "MILLISECONDS", conflicts_with = "no_wait")]
    pub timeout: Option<u64>,

    /// Maximum polling interval in milliseconds (default: 1000)
    #[arg(long, value_name = "MILLISECONDS", requires = "timeout")]
    pub max_poll_interval: Option<u64>,

    /// Custom lock file location
    #[arg(long, value_name = "PATH")]
    pub lock_file: Option<PathBuf>,

    /// Follow symbolic links for output files and housekeep operations
    #[arg(long)]
    pub follow_symlinks: bool,

    /// Follow symbolic links even for lock files (implies --follow-symlinks)
    /// WARNING: May be a security risk
    #[arg(long)]
    pub follow_lock_symlinks: bool,

    /// Create backup before overwrite
    #[arg(short = 'b', long)]
    pub backup: bool,

    /// Backup filename suffix
    #[arg(
        long,
        value_name = "SUFFIX",
        default_value = ".mutx.backup",
        requires = "backup"
    )]
    pub backup_suffix: String,

    /// Store backups in directory
    #[arg(long, value_name = "DIR", requires = "backup")]
    pub backup_dir: Option<PathBuf>,

    /// Add timestamp to backup filename
    #[arg(long, requires = "backup")]
    pub backup_timestamp: bool,

    /// Verbose output
    #[arg(short = 'v', action = clap::ArgAction::Count)]
    pub verbose: u8,
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

        /// Age threshold (e.g., "2h", "7d", "30m")
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
    },
}
