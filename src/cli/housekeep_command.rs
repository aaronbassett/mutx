use crate::cli::Command;
use anyhow::{bail, Result};
use mutx::housekeep::{clean_backups, clean_locks, CleanBackupConfig, CleanLockConfig};
use mutx::utils::parse_duration;
use std::path::PathBuf;

pub fn execute_housekeep(cmd: Command) -> Result<()> {
    let Command::Housekeep {
        dir,
        clean_locks: do_clean_locks,
        clean_backups: do_clean_backups,
        all,
        recursive,
        older_than,
        keep_newest,
        dry_run,
        verbose,
    } = cmd;
    let target_dir = dir.unwrap_or_else(|| PathBuf::from("."));

    if !do_clean_locks && !do_clean_backups && !all {
        bail!("Error: Specify at least one operation: --clean-locks, --clean-backups, or --all");
    }

    let mut total_cleaned = 0;

    // Clean locks
    if do_clean_locks || all {
        let duration = match &older_than {
            Some(s) => Some(parse_duration(s)?),
            None => None,
        };
        let config = CleanLockConfig {
            dir: target_dir.clone(),
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

        total_cleaned += cleaned.len();
    }

    // Clean backups
    if do_clean_backups || all {
        let duration = match &older_than {
            Some(s) => Some(parse_duration(s)?),
            None => None,
        };
        let config = CleanBackupConfig {
            dir: target_dir.clone(),
            recursive,
            older_than: duration,
            keep_newest,
            dry_run,
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

        total_cleaned += cleaned.len();
    }

    if verbose {
        eprintln!("Total: {} file(s) cleaned", total_cleaned);
    }

    Ok(())
}
