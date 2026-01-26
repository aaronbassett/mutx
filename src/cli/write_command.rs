use crate::cli::Args;
use mutx::{
    create_backup, derive_lock_path, validate_lock_path, AtomicWriter, BackupConfig, FileLock,
    LockStrategy, MutxError, Result, TimeoutConfig, WriteMode,
};
use std::fs::File;
use std::io::{self, Read};
use std::time::Duration;

pub fn execute_write(args: Args) -> Result<()> {
    let output = args
        .output
        .ok_or_else(|| MutxError::Other("Output file required".to_string()))?;

    // Validate input file exists if provided
    if let Some(input_path) = &args.input {
        if !input_path.exists() {
            return Err(MutxError::PathNotFound(input_path.clone()));
        }
        if !input_path.is_file() {
            return Err(MutxError::NotAFile(input_path.clone()));
        }
    }

    // Validate backup directory is a directory if provided
    if let Some(backup_dir) = &args.backup_dir {
        if backup_dir.exists() && !backup_dir.is_dir() {
            return Err(MutxError::NotADirectory(backup_dir.clone()));
        }
    }

    // Determine lock strategy
    let lock_strategy = if args.no_wait {
        LockStrategy::NoWait
    } else if let Some(timeout_ms) = args.timeout {
        let mut config = TimeoutConfig::new(Duration::from_millis(timeout_ms));

        if let Some(max_interval_ms) = args.max_poll_interval {
            config = config.with_max_interval(Duration::from_millis(max_interval_ms));
        }

        LockStrategy::Timeout(config)
    } else {
        LockStrategy::Wait
    };

    // Determine lock file path
    let lock_path = if let Some(custom_lock) = args.lock_file {
        custom_lock
    } else {
        derive_lock_path(&output, false)?
    };

    // Validate lock path
    validate_lock_path(&lock_path, &output)?;

    // Acquire lock
    let _lock = FileLock::acquire(&lock_path, lock_strategy)?;

    if args.verbose > 0 {
        eprintln!("Lock acquired: {}", lock_path.display());
    }

    // Create backup if requested
    if args.backup {
        let backup_config = BackupConfig {
            source: output.clone(),
            suffix: args.backup_suffix,
            directory: args.backup_dir,
            timestamp: args.backup_timestamp,
        };

        let backup_path = create_backup(&backup_config)?;
        if args.verbose > 0 {
            eprintln!("Backup created: {}", backup_path.display());
        }
    }

    // Determine write mode
    let mode = if args.stream {
        WriteMode::Streaming
    } else {
        WriteMode::Simple
    };

    // Create writer
    let mut writer = AtomicWriter::new(&output, mode)?;

    // Read input
    let mut input: Box<dyn Read> = if let Some(input_file) = args.input {
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
        let n = input.read(&mut buffer)?;
        if n == 0 {
            break;
        }
        writer.write_all(&buffer[..n])?;
    }

    // Commit write
    writer.commit()?;

    if args.verbose > 0 {
        eprintln!("Write completed: {}", output.display());
    }

    Ok(())
}
