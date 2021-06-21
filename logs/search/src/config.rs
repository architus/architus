//! Contains configuration options for the service that control its network topology
//! and internal behaviors

use anyhow::{Context, Result};
use architus_config_backoff::Backoff;
use serde::Deserialize;
use sloggers::terminal::TerminalLoggerConfig;

/// Configuration object loaded upon startup
#[derive(Debug, Deserialize, Clone)]
pub struct Configuration {
    /// Collection of external services that this service connects to
    pub services: Services,
    /// Parameters for the backoff used to connect to external services during initialization
    pub initialization_backoff: Backoff,
    /// Options related to the GraphQL search API
    pub graphql: GraphQL,
    /// Elasticsearch index containing the stored log events
    pub log_index: String,
    /// Logging configuration (for service diagnostic logs, not Architus log events)
    pub logging: TerminalLoggerConfig,
}

/// Collection of external services that this service connects to
#[derive(Debug, Deserialize, Clone)]
pub struct Services {
    /// URL of the Elasticsearch instance to search log entries from
    pub elasticsearch: String,
}

/// Options related to the GraphQL search API
#[derive(Debug, Deserialize, Clone)]
pub struct GraphQL {
    /// Port that the optional GraphQL HTTP server runs on
    pub http_port: u16,
    /// Default limit of items to fetch in a single page if none is given
    pub default_page_size: usize,
    /// Limit on a single page's size
    /// This is important large pages greatly increase resource utilization
    /// `https://www.elastic.co/guide/en/elasticsearch/reference/7.10/paginate-search-results.html`
    pub max_page_size: usize,
    /// Limit on overall pagination size.
    /// This is important because of the way Elasticsearch works;
    /// deep pagination requires ignored pages to still be loaded,
    /// so we limit then to avoid this restriction.
    /// This should be resolved via UX design on the frontend
    /// `https://www.elastic.co/guide/en/elasticsearch/reference/7.10/paginate-search-results.html`
    pub max_pagination_amount: usize,
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
            // Add in settings from the environment (with a prefix of LOGS_SEARCH_CONFIG)
            // Eg.. `LOGS_SEARCH_CONFIG_PORT=8080 ./target/logs-search`
            // would set the `port` key tot 8080
            .merge(config::Environment::with_prefix("LOGS_SEARCH_CONFIG").separator("__"))
            .context("could not merge in values from the environment")?;
        let config = settings
            .try_into()
            .context("loading the Configuration struct from the merged config failed")?;
        Ok(config)
    }
}
