use crate::cli::{Command, HousekeepOperation};
use mutx::housekeep::{clean_backups, clean_locks, CleanBackupConfig, CleanLockConfig};
use mutx::lock::get_lock_cache_dir;
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
        } => {
            // Smart default: use cache directory
            let target_dir = dir.unwrap_or_else(|| get_lock_cache_dir().unwrap());

            let duration = older_than
                .map(|s| parse_duration(&s))
                .transpose()?;

            let config = CleanLockConfig {
                dir: target_dir,
                recursive,
                older_than: duration,
                dry_run,
            };

            let cleaned = clean_locks(&config)?;
            report_cleaning_results("lock", &cleaned, verbose);
            Ok(())
        }

        HousekeepOperation::Backups {
            dir,
            recursive,
            older_than,
            keep_newest,
            suffix,
            dry_run,
            verbose,
        } => {
            // Smart default: use current directory
            let target_dir = dir.unwrap_or_else(|| PathBuf::from("."));

            let duration = older_than
                .map(|s| parse_duration(&s))
                .transpose()?;

            let config = CleanBackupConfig {
                dir: target_dir,
                recursive,
                older_than: duration,
                keep_newest,
                suffix,
                dry_run,
            };

            let cleaned = clean_backups(&config)?;
            report_cleaning_results("backup", &cleaned, verbose);
            Ok(())
        }

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
        } => {
            // Validation: require either dir OR both locks_dir and backups_dir
            let (locks_path, backups_path) = match (dir, locks_dir, backups_dir) {
                (Some(d), None, None) => (d.clone(), d),
                (None, Some(ld), Some(bd)) => (ld, bd),
                _ => {
                    return Err(MutxError::Other(
                        "Specify either [DIR] or both --locks-dir and --backups-dir".to_string()
                    ));
                }
            };

            let duration = older_than
                .map(|s| parse_duration(&s))
                .transpose()?;

            // Clean locks
            let lock_config = CleanLockConfig {
                dir: locks_path,
                recursive,
                older_than: duration,
                dry_run,
            };
            let cleaned_locks = clean_locks(&lock_config)?;

            // Clean backups
            let backup_config = CleanBackupConfig {
                dir: backups_path,
                recursive,
                older_than: duration,
                keep_newest,
                suffix,
                dry_run,
            };
            let cleaned_backups = clean_backups(&backup_config)?;

            // Report both
            report_cleaning_results("lock", &cleaned_locks, verbose);
            report_cleaning_results("backup", &cleaned_backups, verbose);
            Ok(())
        }
    }
}

fn report_cleaning_results(item_type: &str, cleaned: &[PathBuf], verbose: bool) {
    if cleaned.is_empty() {
        println!("No {} files to clean", item_type);
    } else {
        println!("Cleaned {} {} file(s)", cleaned.len(), item_type);
        if verbose {
            for path in cleaned {
                println!("  - {}", path.display());
            }
        }
    }
}
