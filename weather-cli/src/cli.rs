use chrono::{DateTime, Utc};
use clap::{Parser, Subcommand, ValueHint};
use inquire::Text;
use weather_core::{
    Config, ProviderId, WeatherRequest, WeatherResponse, provider::default_provider_from_config,
};

/// Top-level CLI struct.
#[derive(Debug, Parser)]
#[command(
    name = "weather",
    version,
    about = "Weather CLI",
    long_about = "
        A small weather command-line tool that can talk to multiple providers \
        (OpenWeather and WeatherAPI), store your API keys locally, and \
        show current weather for a given address.",
    after_help = "\
        EXAMPLES:
            # Configure OpenWeather provider
            weather configure openweather

            # Configure WeatherAPI
            weather configure weatherapi

            # List providers and see which one is default
            weather provider list

            # Switch default provider
            weather provider use weatherapi

            # Show current weather for a location
            weather show \"Kyiv\"

            # Show weather for a specific time
            weather show \"Kyiv\" --date 2025-12-04T12:00:00Z
        "
)]
pub struct Cli {
    #[arg(short, long, global = true)]
    pub verbose: bool,
    #[command(subcommand)]
    pub command: Command,
}

#[derive(Debug, Subcommand)]
pub enum Command {
    /// Configure credentials for a specific provider.
    Configure {
        /// Run `weather provider list` to see all supported providers.
        #[arg(value_name = "PROVIDER")]
        provider: String,
    },

    /// Show weather for an address.
    Show {
        /// Address or location name, e.g. "Kyiv".
        #[arg(value_name = "ADDRESS", value_hint = ValueHint::Other)]
        address: String,

        /// Optional date/time in RFC3339 format, e.g. 2025-12-04T12:00:00Z;
        #[arg(long, value_name = "RFC3339_DATETIME")]
        date: Option<String>,
    },

    /// Provider management commands.
    Provider {
        #[command(subcommand)]
        command: ProviderCommand,
    },
}

#[derive(Debug, Subcommand)]
pub enum ProviderCommand {
    /// List all providers and show which ones are configured / default.
    List,

    /// Set default provider (must be already configured).
    Use { provider: String },
}

impl Cli {
    pub async fn run(self) -> anyhow::Result<()> {
        match self.command {
            Command::Configure { provider } => {
                run_configure(provider)?;
            }
            Command::Show { address, date } => {
                run_show(address, date).await?;
            }
            Command::Provider { command } => match command {
                ProviderCommand::List => {
                    run_provider_list()?;
                }
                ProviderCommand::Use { provider } => {
                    run_provider_use(provider)?;
                }
            },
        }

        Ok(())
    }
}

fn parse_date_opt(s: Option<String>) -> anyhow::Result<Option<DateTime<Utc>>> {
    if let Some(raw) = s {
        let dt = DateTime::parse_from_rfc3339(&raw)
            .map_err(|e| anyhow::anyhow!("Failed to parse --date as RFC3339: {e}"))?;
        Ok(Some(dt.with_timezone(&Utc)))
    } else {
        Ok(None)
    }
}

fn print_weather(response: &WeatherResponse) {
    println!("Provider:       {}", response.provider);
    println!("Location:       {}", response.location_name);
    println!("Observed at:    {}", response.observation_time);
    println!("Condition:      {}", response.condition);
    println!("Temperature:    {:.1} °C", response.temperature_c);
    println!("Feels like:     {:.1} °C", response.feels_like_c);
    println!("Humidity:       {} %", response.humidity_pct);
    println!("Wind speed:     {:.1} m/s", response.wind_speed_mps);
}

/// Handle `weather configure <provider>`.
fn run_configure(provider: String) -> anyhow::Result<()> {
    let provider_id = ProviderId::try_from(provider.as_str())?;

    let prompt = format!("Enter API key for provider '{provider_id}':");
    let api_key = Text::new(&prompt)
        .with_placeholder("API key")
        .with_help_message("You can get this from your provider's dashboard.")
        .prompt()?;

    let mut cfg = Config::load()?;

    cfg.upsert_provider_api_key(provider_id, api_key);
    cfg.save()?;

    println!("Configuration updated.");
    if let Ok(default_id) = cfg.default_provider_id() {
        println!("Current default provider: {default_id}");
    }

    Ok(())
}

/// Handle `weather show <address> [--date ...]`.
async fn run_show(address: String, date: Option<String>) -> anyhow::Result<()> {
    let when = parse_date_opt(date)?;

    let cfg = Config::load()?;
    let provider = default_provider_from_config(&cfg)?;

    let request = WeatherRequest { address, when };

    let response = provider.get_weather(&request).await?;

    print_weather(&response);

    Ok(())
}

fn run_provider_list() -> anyhow::Result<()> {
    let cfg = Config::load()?;

    let default_id = cfg.default_provider_id().ok(); // ignore error, might be None

    println!("Providers:");
    println!();

    for id in ProviderId::all() {
        let name = id.as_str();
        let configured = cfg.is_provider_configured(*id);
        let is_default = default_id == Some(*id);

        let status = if configured {
            if is_default { "configured, default" } else { "configured" }
        } else {
            "not configured"
        };

        println!("  - {:<12}  {}", name, status);
    }

    println!();
    println!("Use `weather configure <provider>` to configure a provider.");
    println!("Use `weather provider use <provider>` to switch the default provider.");

    Ok(())
}

fn run_provider_use(provider: String) -> anyhow::Result<()> {
    let id = ProviderId::try_from(provider.as_str())?;

    let mut cfg = Config::load()?;

    if !cfg.is_provider_configured(id) {
        return Err(anyhow::anyhow!(
            "Provider '{id}' is not configured.\n\
             Hint: run `weather configure {id}` first to add an API key."
        ));
    }

    cfg.set_default_provider(id);
    cfg.save()?;

    println!("Default provider set to '{id}'.");

    Ok(())
}
