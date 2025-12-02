//! Core library for the `weather` CLI.
//!
//! This crate defines:
//! - Configuration & credentials handling
//! - Abstraction over weather providers
//! - Shared domain models (requests, responses)
//!
//! It is used by `weather-cli`, but can also be reused by other binaries or services.

pub mod config;
pub mod model;
pub mod provider;

pub use config::{Config, DefaultProvider, ProviderConfig};
pub use model::{WeatherRequest, WeatherResponse};
pub use provider::{ProviderId, WeatherProvider};

#[cfg(test)]
mod tests {
    // use super::*;

    #[test]
    fn it_works() {}
}
