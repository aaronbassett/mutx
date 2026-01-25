use clap::Parser;
use mutx::MutxError;
use std::process;
use tracing_subscriber;

mod cli;

fn main() {
    // Initialize tracing (use RUST_LOG env var to control output)
    tracing_subscriber::fmt()
        .with_writer(std::io::stderr)
        .init();

    let args = cli::Args::parse();

    if let Err(e) = cli::run(args) {
        eprintln!("Error: {}", e);
        let exit_code = match e {
            MutxError::LockTimeout { .. } | MutxError::LockWouldBlock(_) => 2,
            MutxError::Interrupted => 3,
            _ => e.exit_code(),
        };
        process::exit(exit_code);
    }
}
