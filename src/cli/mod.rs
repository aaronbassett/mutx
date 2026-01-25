mod args;
mod housekeep_command;
mod write_command;

use mutx::Result;
pub use args::{Args, Command};

pub fn run(args: Args) -> Result<()> {
    match args.command {
        Some(cmd @ Command::Housekeep { .. }) => housekeep_command::execute_housekeep(cmd),
        None => write_command::execute_write(args),
    }
}
