mod args;
mod write_command;
mod housekeep_command;

pub use args::{Args, Command};
use anyhow::Result;

pub fn run(args: Args) -> Result<()> {
    match args.command {
        Some(cmd @ Command::Housekeep { .. }) => {
            housekeep_command::execute_housekeep(cmd)
        }
        None => {
            write_command::execute_write(args)
        }
    }
}
