use clap::Parser;
use anyhow::Result;

mod cli;

fn main() -> Result<()> {
    let args = cli::Args::parse();
    cli::run(args)
}
