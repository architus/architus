use anyhow::{Context, Result};
use log::debug;
use serde::Deserialize;
use std::time::Duration;

/// Configuration object loaded upon startup
#[derive(Deserialize, Clone)]
pub struct Configuration {
    /// Collection of secret values used to connect to services
    pub secrets: Secrets,
    /// Collection of external services that this service connects to
    pub services: Services,
    /// Maximum number of executing futures for gateway event normalization processing
    pub normalization_stream_concurrency: usize,
    /// Maximum number of executing futures for normalized event importing
    pub import_stream_concurrency: usize,
    /// Max interval to use between import retries
    #[serde(with = "serde_humantime")]
    pub import_backoff_max_interval: Duration,
    /// Overall duration of import retries (exceeding this causes the operation to give up)
    #[serde(with = "serde_humantime")]
    pub import_backoff_duration: Duration,
    /// Multiplier between consecutive backoff intervals to use between import retries
    pub import_backoff_multiplier: f64,
    /// Initial backoff interval to use between import retries
    #[serde(with = "serde_humantime")]
    pub import_backoff_initial_interval: Duration,
}

/// Collection of secret values used to connect to services
#[derive(Deserialize, Clone)]
pub struct Secrets {
    /// Discord bot token used to authenticate with the Gateway API
    pub discord_token: String,
}

/// Collection of external services that this service connects to
#[derive(Deserialize, Clone)]
pub struct Services {
    /// URL of the logging service that normalized LogEvents are forwarded to
    pub logging: String,
}

impl Configuration {
    /// Attempts to load the config from the file, called once at startup
    pub fn try_load(path: impl AsRef<str>) -> Result<Self> {
        let path = path.as_ref();
        debug!("Loading configuration from {}", path);
        // Use config to load the values and merge with the environment
        let mut settings = config::Config::default();
        settings
            .merge(config::File::with_name(path))
            .context(format!("Could not read in config file from {}", path))?
            // Add in settings from the environment (with a prefix of APP)
            // Eg.. `INGRESS_SECRETS__DISCORD_TOKEN=X ./target/ingress-service`
            // would set the `secrets.discord_token` key
            .merge(config::Environment::with_prefix("INGRESS").separator("__"))
            .context("Could not merge in values from the environment")?;
        let config = settings
            .try_into()
            .context("Loading the Configuration struct from the merged config failed")?;
        Ok(config)
    }
}
