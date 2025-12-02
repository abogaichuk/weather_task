//! Binary crate for the `weather` command-line tool.
//!
//! This crate focuses on:
//! - Parsing CLI arguments
//! - Interactive configuration
//! - Human-friendly output formatting

use clap::Parser;

mod cli;

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    let cmd = cli::Cli::parse();
    cmd.run().await
}
