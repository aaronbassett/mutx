use clap::Parser;
use std::process;

mod cli;

fn main() {
    let args = cli::Args::parse();

    if let Err(err) = cli::run(args) {
        // TODO: Update to use MutxError in Task 8
        eprintln!("Error: {}", err);
        process::exit(1);
    }
}
