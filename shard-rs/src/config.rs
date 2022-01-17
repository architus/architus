//! Contains configuration options for the service that control its network topology
//! and internal behaviors

use anyhow::Context;
use serde::Deserialize;
use sloggers::terminal::TerminalLoggerConfig;

/// Configuration object loaded upon startup
#[derive(Debug, Deserialize, Clone)]
pub struct Configuration {
    pub discord_app_id: std::num::NonZeroU64,
    pub discord_token: String,
    pub comic_description: String,

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
