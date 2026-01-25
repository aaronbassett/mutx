mod args;
mod write_command;

pub use args::{Args, Command};
use anyhow::Result;

pub fn run(args: Args) -> Result<()> {
    match args.command {
        Some(Command::Housekeep { .. }) => {
            // TODO: Implement housekeep
            eprintln!("Housekeep not yet implemented");
            Ok(())
        }
        None => {
            write_command::execute_write(args)
        }
    }
}
