//! Binary crate for the `weather` command-line tool.
//!
//! This crate focuses on:
//! - Parsing CLI arguments
//! - Interactive configuration
//! - Human-friendly output formatting

use clap::Parser;
use cli::Cli;

mod cli;

#[tokio::main]
async fn main() {
    let cli = Cli::parse();
    let verbose = cli.verbose;

    if let Err(err) = cli.run().await {
        eprintln!("\nerror: {err}");

        if verbose {
            eprintln!("\nError chain:");
            for (i, cause) in err.chain().enumerate().skip(1) {
                eprintln!("  {i}: {cause}");
            }
        } else {
            eprintln!("(run with -v or --verbose to see the full error chain)");
        }

        std::process::exit(1);
    }
}
