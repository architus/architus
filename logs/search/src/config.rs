//! Contains configuration options for the service that control its network topology
//! and internal behaviors

use anyhow::Context;
use architus_config_backoff::Backoff;
use serde::Deserialize;
use sloggers::terminal::TerminalLoggerConfig;

/// Configuration object loaded upon startup
#[derive(Debug, Deserialize, Clone)]
pub struct Configuration {
    /// Parameters for the database connection to Elasticsearch
    pub elasticsearch: Elasticsearch,
    /// Options related to the GraphQL search API
    pub graphql: GraphQL,
    /// Parameters for the backoff used to connect to external services during initialization
    pub initialization_backoff: Backoff,
    /// Logging configuration (for service diagnostic logs, not Architus log events)
    pub logging: TerminalLoggerConfig,
    /// Configuration for Rocket, the HTTP framework
    pub rocket: rocket::Config,
}

/// Parameters for the database connection to Elasticsearch
#[derive(Debug, Deserialize, Clone)]
pub struct Elasticsearch {
    /// URL of the Elasticsearch instance to search log entries from
    pub url: String,
    /// Elasticsearch index containing the stored log events.
    /// This should already exist; this service will not create it.
    pub index: String,
    /// Username to use when connecting to Elasticsearch.
    /// If given, this user should have RBAC permissions for:
    /// - read (to search log events) for the log event index
    /// If empty, then authentication is disabled.
    pub auth_username: String,
    /// Password to use when connecting to Elasticsearch.
    /// Ignored if the user is empty.
    pub auth_password: String,
}

/// Options related to the GraphQL search API
#[derive(Debug, Deserialize, Clone)]
pub struct GraphQL {
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
    pub fn try_load(path: impl AsRef<str>) -> anyhow::Result<Self> {
        let path = path.as_ref();
        // Use config to load the values and merge with the environment
        let mut settings = config::Config::default();
        settings
            .set_default("rocket", rocket_config_defaults()?)
            .context("could not set default Rocket config values")?
            .merge(config::File::with_name(path))
            .context(format!("Could not read in config file from {}", path))?
            // Add in settings from the environment (with a prefix of LOGS_SEARCH_CONFIG)
            // Eg.. `LOGS_SEARCH_CONFIG_LOG_INDEX=logs ./target/logs-search`
            // would set the `log_index` key to logs
            .merge(config::Environment::with_prefix("LOGS_SEARCH_CONFIG").separator("__"))
            .context("could not merge in values from the environment")?;
        let config = settings
            .try_into()
            .context("loading the Configuration struct from the merged config failed")?;
        Ok(config)
    }
}

/// Gets the `rocket::Config` default values as `config::Value` instance
/// to be folded into a `config::Config` instance as a sub-key
fn rocket_config_defaults() -> anyhow::Result<config::Value> {
    let default_rocket_config = rocket::Config::default();

    // Create a `config::Config` from the rocket config spread as the root
    let as_config = config::Config::try_from(&default_rocket_config)
        .context("could not convert rocket::Config to config::Config")?;

    // Convert the `config::Config` to a `config::Value`
    let as_value = as_config
        .try_into::<config::Value>()
        .context("could not convert config::Config to config::Value")?;

    Ok(as_value)
}
