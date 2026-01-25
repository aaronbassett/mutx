use crate::cli::Args;
use anyhow::{Context, Result};
use mutx::{create_backup, AtomicWriter, BackupConfig, FileLock, LockStrategy, WriteMode};
use std::fs::File;
use std::io::{self, Read};
use std::time::Duration;

pub fn execute_write(args: Args) -> Result<()> {
    let output = args.output.context("Output file required")?;

    // Determine lock strategy
    let lock_strategy = if args.no_wait {
        LockStrategy::NoWait
    } else if let Some(timeout) = args.timeout {
        LockStrategy::Timeout(Duration::from_secs(timeout))
    } else {
        LockStrategy::Wait
    };

    // Determine lock file path
    let lock_path = args.lock_file.unwrap_or_else(|| {
        let mut path = output.clone();
        path.set_extension("lock");
        path
    });

    // Acquire lock
    let _lock = FileLock::acquire(&lock_path, lock_strategy).context("Failed to acquire lock")?;

    if args.verbose > 0 {
        eprintln!("Lock acquired: {}", lock_path.display());
    }

    // Create backup if requested
    if args.backup {
        let backup_config = BackupConfig {
            suffix: args.backup_suffix,
            timestamp: args.backup_timestamp,
            backup_dir: args.backup_dir,
        };

        let backup_path = create_backup(&output, &backup_config)?;
        if args.verbose > 0 && backup_path.exists() {
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
        Box::new(
            File::open(&input_file)
                .with_context(|| format!("Failed to open input file: {}", input_file.display()))?,
        )
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
