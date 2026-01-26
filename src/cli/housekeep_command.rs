use crate::cli::{Command, HousekeepOperation};
use mutx::housekeep::{clean_backups, clean_locks, CleanBackupConfig, CleanLockConfig};
use mutx::utils::parse_duration;
use mutx::{MutxError, Result};
use std::path::PathBuf;

pub fn execute_housekeep(cmd: Command) -> Result<()> {
    let Command::Housekeep { operation } = cmd else {
        return Err(MutxError::Other(
            "Internal error: expected Housekeep command".to_string(),
        ));
    };

    match operation {
        HousekeepOperation::Locks {
            dir,
            recursive,
            older_than,
            dry_run,
            verbose,
        } => execute_clean_locks(dir, recursive, older_than, dry_run, verbose),
        HousekeepOperation::Backups {
            dir,
            recursive,
            older_than,
            keep_newest,
            suffix,
            dry_run,
            verbose,
        } => execute_clean_backups(
            dir,
            recursive,
            older_than,
            keep_newest,
            suffix,
            dry_run,
            verbose,
        ),
        HousekeepOperation::All {
            dir,
            locks_dir,
            backups_dir,
            recursive,
            older_than,
            keep_newest,
            suffix,
            dry_run,
            verbose,
        } => execute_clean_all(
            dir,
            locks_dir,
            backups_dir,
            recursive,
            older_than,
            keep_newest,
            suffix,
            dry_run,
            verbose,
        ),
    }
}

fn execute_clean_locks(
    dir: Option<PathBuf>,
    recursive: bool,
    older_than: Option<String>,
    dry_run: bool,
    verbose: bool,
) -> Result<()> {
    let target_dir = dir.unwrap_or_else(|| PathBuf::from("."));
    let duration = match &older_than {
        Some(s) => Some(parse_duration(s)?),
        None => None,
    };

    let config = CleanLockConfig {
        dir: target_dir,
        recursive,
        older_than: duration,
        dry_run,
    };

    let cleaned = clean_locks(&config)?;

    for path in &cleaned {
        println!(
            "{}{}",
            if dry_run {
                "[DRY RUN] Would delete: "
            } else {
                "Deleted: "
            },
            path.display()
        );
    }

    if verbose || dry_run {
        eprintln!("Cleaned {} lock file(s)", cleaned.len());
    }

    Ok(())
}

fn execute_clean_backups(
    dir: Option<PathBuf>,
    recursive: bool,
    older_than: Option<String>,
    keep_newest: Option<usize>,
    suffix: String,
    dry_run: bool,
    verbose: bool,
) -> Result<()> {
    let target_dir = dir.unwrap_or_else(|| PathBuf::from("."));
    let duration = match &older_than {
        Some(s) => Some(parse_duration(s)?),
        None => None,
    };

    let config = CleanBackupConfig {
        dir: target_dir,
        recursive,
        older_than: duration,
        keep_newest,
        dry_run,
        suffix,
    };

    let cleaned = clean_backups(&config)?;

    for path in &cleaned {
        println!(
            "{}{}",
            if dry_run {
                "[DRY RUN] Would delete: "
            } else {
                "Deleted: "
            },
            path.display()
        );
    }

    if verbose || dry_run {
        eprintln!("Cleaned {} backup file(s)", cleaned.len());
    }

    Ok(())
}

fn execute_clean_all(
    dir: Option<PathBuf>,
    locks_dir: Option<PathBuf>,
    backups_dir: Option<PathBuf>,
    recursive: bool,
    older_than: Option<String>,
    keep_newest: Option<usize>,
    suffix: String,
    dry_run: bool,
    verbose: bool,
) -> Result<()> {
    // Validate that either dir is provided, or both locks_dir and backups_dir
    if dir.is_none() && (locks_dir.is_none() || backups_dir.is_none()) {
        return Err(MutxError::Other(
            "Must provide either DIR or both --locks-dir and --backups-dir".to_string(),
        ));
    }

    let mut total_cleaned = 0;

    // Determine directories
    let locks_target = locks_dir.or_else(|| dir.clone()).unwrap();
    let backups_target = backups_dir.or_else(|| dir.clone()).unwrap();

    // Clean locks
    let duration = match &older_than {
        Some(s) => Some(parse_duration(s)?),
        None => None,
    };

    let lock_config = CleanLockConfig {
        dir: locks_target,
        recursive,
        older_than: duration.clone(),
        dry_run,
    };

    let cleaned = clean_locks(&lock_config)?;

    for path in &cleaned {
        println!(
            "{}{}",
            if dry_run {
                "[DRY RUN] Would delete: "
            } else {
                "Deleted: "
            },
            path.display()
        );
    }

    if verbose || dry_run {
        eprintln!("Cleaned {} lock file(s)", cleaned.len());
    }

    total_cleaned += cleaned.len();

    // Clean backups
    let backup_config = CleanBackupConfig {
        dir: backups_target,
        recursive,
        older_than: duration,
        keep_newest,
        dry_run,
        suffix,
    };

    let cleaned = clean_backups(&backup_config)?;

    for path in &cleaned {
        println!(
            "{}{}",
            if dry_run {
                "[DRY RUN] Would delete: "
            } else {
                "Deleted: "
            },
            path.display()
        );
    }

    if verbose || dry_run {
        eprintln!("Cleaned {} backup file(s)", cleaned.len());
    }

    total_cleaned += cleaned.len();

    if verbose {
        eprintln!("Total: {} file(s) cleaned", total_cleaned);
    }

    Ok(())
}
