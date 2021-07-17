//! Contains configuration options for the service that control its network topology
//! and internal behaviors

use anyhow::Context;
use serde::Deserialize;
use sloggers::terminal::TerminalLoggerConfig;

/// Configuration object loaded upon startup
#[derive(Default, Debug, Deserialize, Clone)]
pub struct Configuration {
    /// The port that the gRPC server listens on
    pub port: u16,
    /// Logging configuration (for service diagnostic logs, not Architus log events)
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
            // Add in settings from the environment (with a prefix of LOGS_UPTIME_CONFIG_)
            // Eg.. `LOGS_UPTIME_CONFIG_PORT=X ./target/logs-uptime` would set the `port` key
            .merge(config::Environment::with_prefix("LOGS_UPTIME_CONFIG").separator("__"))
            .context("could not merge in values from the environment")?;
        let config = settings
            .try_into()
            .context("loading the Configuration struct from the merged config failed")?;
        Ok(config)
    }
}
