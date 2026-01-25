use crate::cli::Command;
use mutx::housekeep::{clean_backups, clean_locks, CleanBackupConfig, CleanLockConfig};
use anyhow::{bail, Result};
use std::path::PathBuf;
use std::time::Duration;

pub fn execute_housekeep(cmd: Command) -> Result<()> {
    if let Command::Housekeep {
        dir,
        clean_locks: do_clean_locks,
        clean_backups: do_clean_backups,
        all,
        recursive,
        older_than,
        keep_newest,
        dry_run,
        verbose,
        json,
    } = cmd
    {
        let target_dir = dir.unwrap_or_else(|| PathBuf::from("."));

        if !do_clean_locks && !do_clean_backups && !all {
            bail!("Error: Specify at least one operation: --clean-locks, --clean-backups, or --all");
        }

        let mut total_cleaned = 0;

        // Clean locks
        if do_clean_locks || all {
            let duration = parse_duration_hours(&older_than)?;
            let config = CleanLockConfig {
                dir: target_dir.clone(),
                recursive,
                older_than: duration,
                dry_run,
            };

            let cleaned = clean_locks(&config)?;

            if json {
                println!("{{\"operation\": \"clean_locks\", \"count\": {}, \"files\": {:?}}}",
                    cleaned.len(), cleaned);
            } else {
                for path in &cleaned {
                    println!("{}{}",
                        if dry_run { "[DRY RUN] Would delete: " } else { "Deleted: " },
                        path.display()
                    );
                }
                if verbose || dry_run {
                    eprintln!("Cleaned {} lock file(s)", cleaned.len());
                }
            }

            total_cleaned += cleaned.len();
        }

        // Clean backups
        if do_clean_backups || all {
            let duration = parse_duration_days(&older_than)?;
            let config = CleanBackupConfig {
                dir: target_dir.clone(),
                recursive,
                older_than: duration,
                keep_newest,
                dry_run,
            };

            let cleaned = clean_backups(&config)?;

            if json {
                println!("{{\"operation\": \"clean_backups\", \"count\": {}, \"files\": {:?}}}",
                    cleaned.len(), cleaned);
            } else {
                for path in &cleaned {
                    println!("{}{}",
                        if dry_run { "[DRY RUN] Would delete: " } else { "Deleted: " },
                        path.display()
                    );
                }
                if verbose || dry_run {
                    eprintln!("Cleaned {} backup file(s)", cleaned.len());
                }
            }

            total_cleaned += cleaned.len();
        }

        if !json && verbose {
            eprintln!("Total: {} file(s) cleaned", total_cleaned);
        }

        Ok(())
    } else {
        unreachable!()
    }
}

fn parse_duration_hours(s: &Option<String>) -> Result<Option<Duration>> {
    match s {
        Some(s) => {
            if s.ends_with('h') {
                let hours: u64 = s[..s.len()-1].parse()?;
                Ok(Some(Duration::from_secs(hours * 3600)))
            } else {
                let hours: u64 = s.parse()?;
                Ok(Some(Duration::from_secs(hours * 3600)))
            }
        }
        None => Ok(None),
    }
}

fn parse_duration_days(s: &Option<String>) -> Result<Option<Duration>> {
    match s {
        Some(s) => {
            if s.ends_with('d') {
                let days: u64 = s[..s.len()-1].parse()?;
                Ok(Some(Duration::from_secs(days * 86400)))
            } else {
                let days: u64 = s.parse()?;
                Ok(Some(Duration::from_secs(days * 86400)))
            }
        }
        None => Ok(None),
    }
}
