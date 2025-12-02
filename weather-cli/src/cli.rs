use clap::{Parser, Subcommand};

/// Top-level CLI struct.
#[derive(Debug, Parser)]
#[command(name = "weather", version, about = "Weather CLI")]
pub struct Cli {
    #[command(subcommand)]
    pub command: Command,
}

#[derive(Debug, Subcommand)]
pub enum Command {
    /// Configure credentials for a specific provider.
    Configure {
        /// Provider short name, e.g. "openweather" or "weatherapi".
        provider: String,
    },

    /// Show weather for an address.
    Show {
        /// Address or location name.
        address: String,

        /// Optional date/time; if absent, means "now".
        #[arg(long)]
        date: Option<String>,
    },
}

impl Cli {
    pub async fn run(self) -> anyhow::Result<()> {
        match self.command {
            Command::Configure { provider } => {
                // TODO: interactive configuration for `provider`
                println!("Configuring provider: {provider}");
            }
            Command::Show { address, date } => {
                // TODO: resolve provider from config, call core, print human-readable output
                println!("Showing weather for: {address}, date: {date:?}");
            }
        }

        Ok(())
    }
}
