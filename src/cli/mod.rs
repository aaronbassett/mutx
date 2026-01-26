mod args;
mod housekeep_command;
mod write_command;

pub use args::{Args, Command, HousekeepOperation};
use mutx::{MutxError, Result};

pub fn run(args: Args) -> Result<()> {
    match args.command {
        Some(Command::Write {
            output,
            input,
            stream,
            no_wait,
            timeout,
            max_poll_interval,
            backup,
            backup_suffix,
            backup_dir,
            backup_timestamp,
            lock_file,
            follow_symlinks,
            follow_lock_symlinks,
            verbose,
        }) => {
            // Explicit: mutx write output.txt
            write_command::execute_write(
                output,
                input,
                stream,
                no_wait,
                timeout,
                max_poll_interval,
                backup,
                backup_suffix,
                backup_dir,
                backup_timestamp,
                lock_file,
                follow_symlinks,
                follow_lock_symlinks,
                verbose,
            )
        }
        Some(Command::Housekeep { operation }) => {
            housekeep_command::execute_housekeep(Command::Housekeep { operation })
        }
        None => {
            // Implicit: mutx output.txt
            // Use top-level args for backward compatibility
            write_command::execute_write(
                args.output.ok_or_else(|| {
                    MutxError::Other("OUTPUT argument required".to_string())
                })?,
                args.input,
                args.stream,
                args.no_wait,
                args.timeout,
                args.max_poll_interval,
                args.backup,
                args.backup_suffix,
                args.backup_dir,
                args.backup_timestamp,
                args.lock_file,
                args.follow_symlinks,
                args.follow_lock_symlinks,
                args.verbose,
            )
        }
    }
}
