use mutx::{
    check_lock_symlink, check_symlink, create_backup, derive_lock_path, validate_lock_path,
    AtomicWriter, BackupConfig, FileLock, LockStrategy, MutxError, Result, TimeoutConfig,
    WriteMode,
};
use std::fs::File;
use std::io::{self, Read};
use std::path::PathBuf;
use std::time::Duration;

#[allow(clippy::too_many_arguments)]
pub fn execute_write(
    output: PathBuf,
    input: Option<PathBuf>,
    stream: bool,
    no_wait: bool,
    timeout: Option<u64>,
    max_poll_interval: Option<u64>,
    backup: bool,
    backup_suffix: String,
    backup_dir: Option<PathBuf>,
    backup_timestamp: bool,
    lock_file: Option<PathBuf>,
    follow_symlinks: bool,
    follow_lock_symlinks: bool,
    verbose: u8,
) -> Result<()> {

    // Determine symlink policy
    let follow_symlinks_effective = follow_lock_symlinks || follow_symlinks;
    let follow_lock_symlinks_effective = follow_lock_symlinks;

    // Validate input file exists if provided
    if let Some(input_path) = &input {
        if !input_path.exists() {
            return Err(MutxError::PathNotFound(input_path.clone()));
        }
        if !input_path.is_file() {
            return Err(MutxError::NotAFile(input_path.clone()));
        }

        // Check if input is a symlink
        check_symlink(input_path, follow_symlinks_effective)?;
    }

    // Check if output is a symlink
    check_symlink(&output, follow_symlinks_effective)?;

    // Validate backup directory is a directory if provided
    if let Some(backup_dir_ref) = &backup_dir {
        if backup_dir_ref.exists() && !backup_dir_ref.is_dir() {
            return Err(MutxError::NotADirectory(backup_dir_ref.clone()));
        }
    }

    // Determine lock strategy
    let lock_strategy = if no_wait {
        LockStrategy::NoWait
    } else if let Some(timeout_ms) = timeout {
        let mut config = TimeoutConfig::new(Duration::from_millis(timeout_ms));

        if let Some(max_interval_ms) = max_poll_interval {
            config = config.with_max_interval(Duration::from_millis(max_interval_ms));
        }

        LockStrategy::Timeout(config)
    } else {
        LockStrategy::Wait
    };

    // Determine lock file path
    let lock_path = if let Some(custom_lock) = lock_file {
        custom_lock
    } else {
        derive_lock_path(&output, false)?
    };

    // Validate lock path
    validate_lock_path(&lock_path, &output)?;

    // Check if lock path is a symlink
    check_lock_symlink(&lock_path, follow_lock_symlinks_effective)?;

    // Acquire lock
    let _lock = FileLock::acquire(&lock_path, lock_strategy)?;

    if verbose > 0 {
        eprintln!("Lock acquired: {}", lock_path.display());
    }

    // Create backup if requested
    if backup {
        let backup_config = BackupConfig {
            source: output.clone(),
            suffix: backup_suffix,
            directory: backup_dir,
            timestamp: backup_timestamp,
        };

        let backup_path = create_backup(&backup_config)?;
        if verbose > 0 {
            eprintln!("Backup created: {}", backup_path.display());
        }
    }

    // Determine write mode
    let mode = if stream {
        WriteMode::Streaming
    } else {
        WriteMode::Simple
    };

    // Create writer
    let mut writer = AtomicWriter::new(&output, mode)?;

    // Read input
    let mut input_reader: Box<dyn Read> = if let Some(input_file) = input {
        Box::new(File::open(&input_file).map_err(|e| MutxError::ReadFailed {
            path: input_file,
            source: e,
        })?)
    } else {
        Box::new(io::stdin())
    };

    // Copy data
    let mut buffer = [0u8; 8192];
    loop {
        let n = input_reader.read(&mut buffer)?;
        if n == 0 {
            break;
        }
        writer.write_all(&buffer[..n])?;
    }

    // Commit write
    writer.commit()?;

    if verbose > 0 {
        eprintln!("Write completed: {}", output.display());
    }

    Ok(())
}
