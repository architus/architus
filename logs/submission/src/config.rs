//! Contains configuration options for the service that control its network topology
//! and internal behaviors

use anyhow::{Context, Result};
use architus_config_backoff::Backoff;
use serde::Deserialize;
use sloggers::terminal::TerminalLoggerConfig;
use std::time::Duration;

/// Configuration object loaded upon startup
#[derive(Debug, Deserialize, Clone)]
pub struct Configuration {
    /// The port that the gRPC server listens on
    pub port: u16,
    /// Collection of external services that this service connects to
    pub services: Services,
    /// Parameters for the backoff used to forward events to Elasticsearch
    pub submission_backoff: Backoff,
    /// Logging configuration (for service diagnostic logs, not Architus log events)
    pub logging: TerminalLoggerConfig,
    /// How long to wait for durable submission confirmation
    /// before returning with "deadline exceeded" and encouraging retry
    #[serde(with = "serde_humantime")]
    pub submission_wait_timeout: Duration,
    /// The number of events that will trigger an immediate batch submit
    /// even if the event submission debounce period has not elapsed
    pub debounce_size: usize,
    /// The period of time since the oldest event in a batch was enqueued
    /// that the entire batch will be submitted
    #[serde(with = "serde_humantime")]
    pub debounce_period: Duration,
    /// Elasticsearch index containing the stored log events
    pub elasticsearch_index: String,
}

/// Collection of external services that this service connects to
#[derive(Debug, Deserialize, Clone)]
pub struct Services {
    /// URL of the Elasticsearch instance to store log entries in
    pub elasticsearch: String,
}

impl Configuration {
    /// Attempts to load the config from the file, called once at startup
    pub fn try_load(path: impl AsRef<str>) -> Result<Self> {
        let path = path.as_ref();
        // Use config to load the values and merge with the environment
        let mut settings = config::Config::default();
        settings
            .merge(config::File::with_name(path))
            .context(format!("Could not read in config file from {}", path))?
            // Add in settings from the environment (with a prefix of LOGS_SUBMISSION_)
            // Eg.. `LOGS_SUBMISSION_PORT=X ./target/logs-submission` would set the `port` key
            .merge(config::Environment::with_prefix("LOGS_SUBMISSION").separator("__"))
            .context("Could not merge in values from the environment")?;
        let config = settings
            .try_into()
            .context("Loading the Configuration struct from the merged config failed")?;
        Ok(config)
    }
}
