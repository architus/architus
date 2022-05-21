//! Contains configuration options for the service that control its network topology
//! and internal behaviors

use anyhow::Context;
use architus_config_backoff::Backoff;
use serde::Deserialize;
use sloggers::terminal::TerminalLoggerConfig;
use std::num::NonZeroU64;
use std::time::Duration;

/// Configuration object loaded upon startup
#[derive(Debug, Deserialize, Clone)]
pub struct Configuration {
    pub discord_app_id: std::num::NonZeroU64,
    pub discord_token: String,
    pub comic_description: String,
    pub gulag_description: String,
    pub architus_user_id: std::num::NonZeroU64,

    pub default_gulag_severity: u64,
    pub default_gulag_threshold: u64,

    /// The timeout/backoff used to connect to external services during initialization
    pub initialization: BackoffAndTimeout,

    pub database: deadpool_postgres::Config,

    pub temp_test_guild: NonZeroU64,

    pub logging: TerminalLoggerConfig,
}

/// Combination of backoff and timeout config for a class of RPC's
#[derive(Debug, Deserialize, Clone)]
pub struct BackoffAndTimeout {
    #[serde(with = "humantime_serde")]
    pub attempt_timeout: Duration,
    pub backoff: Backoff,
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
            // Add in settings from the environment (with a prefix of SHARD_RS_CONFIG_)
            // Eg.. `SHARD_RS_CONFIG_PORT=X ./target/shard-rs` would set the `port` key
            .merge(config::Environment::with_prefix("SHARD_RS_CONFIG").separator("__"))
            .context("could not merge in values from the environment")?;
        let config = settings
            .try_into()
            .context("loading the Configuration struct from the merged config failed")?;
        Ok(config)
    }
}
