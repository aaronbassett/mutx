use clap::Parser;
use std::process;

mod cli;

fn main() {
    let args = cli::Args::parse();

    if let Err(err) = cli::run(args) {
        let app_err = mutx::error::AppError::from_anyhow(err);
        eprintln!("{}", app_err);
        process::exit(app_err.exit_code());
    }
}
