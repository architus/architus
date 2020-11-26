use anyhow::{Context, Result};
use log::debug;
use serde::Deserialize;

/// Configuration object loaded upon startup
#[derive(Deserialize, Clone)]
pub struct Configuration {
    /// Port that the main gRPC server listens on
    pub port: u16,
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
            // Add in settings from the environment (with a prefix of LOGGING)
            // Eg.. `LOGGING_PORT=8080 ./target/logging-service`
            // would set the `port` key tot 8080
            .merge(config::Environment::with_prefix("LOGGING").separator("__"))
            .context("Could not merge in values from the environment")?;
        let config = settings
            .try_into()
            .context("Loading the Configuration struct from the merged config failed")?;
        Ok(config)
    }
}
