//! Contains configuration options for the service that control its network topology
//! and internal behaviors

use anyhow::Context;
use serde::Deserialize;
use sloggers::terminal::TerminalLoggerConfig;

use std::time::Duration;

/// Configuration object loaded upon startup
#[derive(Debug, Deserialize, Clone)]
pub struct Configuration {
    pub discord_token: String,
    pub manager_uri: String,
    pub worker_threads: usize,
    pub architus_id: u64,

    #[serde(with = "humantime_serde")]
    pub fail_wait_time: Duration,

    pub logging: TerminalLoggerConfig,
}

impl Configuration {
    /// Attempts to load the config from the file, called once at startup
    pub fn try_load(path: impl AsRef<str>) -> anyhow::Result<Self> {
        let path = path.as_ref();
        // Use config to load the values and merge with the environment
        let mut settings = config::Config::default();
        settings
            .merge(config::File::with_name(path))
            .context(format!("Could not read in config file from {}", path))?
            // Add in settings from the environment (with a prefix of LOG_CONSUMER_CONFIG_)
            // Eg.. `LOG_CONSUMER_CONFIG_PORT=X ./target/scrape-consumer` would set the `port` key
            .merge(config::Environment::with_prefix("LOG_CONSUMER_CONFIG").separator("__"))
            .context("could not merge in values from the environment")?;
        let config = settings
            .try_into()
            .context("loading the Configuration struct from the merged config failed")?;
        Ok(config)
    }
}
