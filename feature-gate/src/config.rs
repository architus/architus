//! Contains configuration options for the service that control its network topology
//! and internal behaviors

use anyhow::Context;
use architus_config_backoff::Backoff;
use serde::Deserialize;
use sloggers::terminal::TerminalLoggerConfig;

/// Configuration object loaded upon startup
#[derive(Default, Debug, Deserialize, Clone)]
pub struct Configuration {
    /// The port that the gRPC server listens on
    pub port: u16,
    /// Collection of values used to connect to the database
    pub database: Database,
    /// Parameters for the backoff used to connect to external services during initialization
    pub initialization_backoff: Backoff,
    /// Logging configuration (for service diagnostic logs, not Architus log events)
    pub logging: TerminalLoggerConfig,
    /// Size of the database connection pool
    pub connection_pool_size: u32,
}

/// Collection of values used to connect to the database
#[derive(Default, Debug, Deserialize, Clone)]
pub struct Database {
    pub user_name: String,
    pub user_password: String,
    pub host: String,
    pub port: u16,
    pub database_name: String,
}

impl Configuration {
    /// Attempts to load the config from the file, called once at startup
    /// # Errors
    /// * The path does not exist or could not be read from
    /// * Could not merge in values from environment variables
    /// * Could not parse the config into the typed struct
    pub fn try_load(path: impl AsRef<str>) -> anyhow::Result<Self> {
        let path = path.as_ref();
        // Use config to load the values and merge with the environment
        let mut settings = config::Config::default();
        settings
            .merge(config::File::with_name(path))
            .context(format!("Could not read in config file from {}", path))?
            // Add in settings from the environment (with a prefix of FEATURE_GATE_CONFIG)
            // Eg.. `FEATURE_GATE_CONFIG_DATABASE__USER_PASSWORD=X ./target/feature-gate`
            // would set the `database.user_password` key
            .merge(config::Environment::with_prefix("FEATURE_GATE_CONFIG").separator("__"))
            .context("could not merge in values from the environment")?;
        let config = settings
            .try_into()
            .context("loading the Configuration struct from the merged config failed")?;
        Ok(config)
    }
}
