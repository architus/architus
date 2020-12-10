use anyhow::{Context, Result};
use backoff::ExponentialBackoff;
use log::{debug, info};
use serde::Deserialize;
use std::time::Duration;

/// Configuration object loaded upon startup
#[derive(Default, Debug, Deserialize, Clone)]
pub struct Configuration {
    /// Collection of secret values used to connect to services
    pub secrets: Secrets,
    /// Collection of external services that this service connects to
    pub services: Services,
    /// Parameters for the backoff used to connect to external services during initialization
    pub initialization_backoff: ConfigBackoff,
    /// Name of the durable queue that events get published to
    pub rabbitmq_queue_name: String,
    /// Length of time that consecutive guild uptime events are grouped together in
    #[serde(with = "serde_humantime")]
    pub guild_uptime_debounce_delay: Duration,
}

/// Collection of secret values used to connect to services
#[derive(Default, Debug, Deserialize, Clone)]
pub struct Secrets {
    /// Discord bot token used to authenticate with the Gateway API
    pub discord_token: String,
}

/// Collection of external services that this service connects to
#[derive(Default, Debug, Deserialize, Clone)]
pub struct Services {
    /// Full AMQP URL to connect to the gateway queue at
    pub gateway_queue: String,
}

/// Controls an exponential backoff and can be loaded from a config file
/// TODO move to shared crate
#[derive(Default, Debug, Deserialize, Clone)]
pub struct ConfigBackoff {
    #[serde(with = "serde_humantime")]
    pub initial_interval: Duration,
    #[serde(with = "serde_humantime")]
    pub max_interval: Duration,
    #[serde(with = "serde_humantime")]
    pub duration: Duration,
    pub multiplier: f64,
}

impl ConfigBackoff {
    pub fn build(&self) -> ExponentialBackoff {
        self.into()
    }
}

impl<'a> Into<ExponentialBackoff> for &'a ConfigBackoff {
    fn into(self) -> ExponentialBackoff {
        ExponentialBackoff {
            current_interval: self.initial_interval.clone(),
            initial_interval: self.initial_interval.clone(),
            multiplier: self.multiplier.clone(),
            max_interval: self.max_interval.clone(),
            max_elapsed_time: Some(self.duration.clone()),
            ..Default::default()
        }
    }
}

impl Configuration {
    /// Attempts to load the config from the file, called once at startup
    pub fn try_load(path: impl AsRef<str>) -> Result<Self> {
        let path = path.as_ref();
        info!("Loading configuration from {}", path);
        // Use config to load the values and merge with the environment
        let mut settings = config::Config::default();
        settings
            .merge(config::File::with_name(path))
            .context(format!("Could not read in config file from {}", path))?
            // Add in settings from the environment (with a prefix of INGRESS)
            // Eg.. `LOGS_GATEWAY_INGRESS_SECRETS__DISCORD_TOKEN=X ./target/logs-gateway-ingress`
            // would set the `secrets.discord_token` key
            .merge(config::Environment::with_prefix("LOGS_GATEWAY_INGRESS").separator("__"))
            .context("Could not merge in values from the environment")?;
        let config = settings
            .try_into()
            .context("Loading the Configuration struct from the merged config failed")?;
        debug!("Configuration: {:?}", config);
        Ok(config)
    }
}
