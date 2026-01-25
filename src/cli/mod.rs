mod args;

pub use args::{Args, Command};
use anyhow::{bail, Result};

pub fn run(args: Args) -> Result<()> {
    // Validate that OUTPUT is provided when no subcommand is given
    if args.command.is_none() && args.output.is_none() {
        bail!("error: the following required arguments were not provided:\n  <OUTPUT>\n\nUsage: mutx [OPTIONS] <OUTPUT>\n\nFor more information, try '--help'.");
    }

    // TODO: Implement
    println!("Args: {:?}", args);
    Ok(())
}
