use anyhow::{Context, Result};
use architus_config_backoff::Backoff;
use log::{debug, info};
use serde::Deserialize;
use std::time::Duration;

/// Configuration object loaded upon startup
#[derive(Debug, Deserialize, Clone)]
pub struct Configuration {
    /// Collection of secret values used to connect to services
    pub secrets: Secrets,
    /// Collection of external services that this service connects to
    pub services: Services,
    /// Parameters for the backoff used to connect to external services during initialization
    pub initialization_backoff: Backoff,
    /// Parameters for the backoff used to reconnect to the gateway queue
    pub reconnection_backoff: Backoff,
    /// The duration of successful runtime after which to reset the reconnection backoff
    #[serde(with = "serde_humantime")]
    pub reconnection_backoff_reset_threshold: Duration,
    /// Parameters for the backoff used to send RPC calls to other services
    pub rpc_backoff: Backoff,
    /// Maximum number of executing futures for gateway event normalization processing
    pub queue_consumer_concurrency: u16,
    /// Config options related to the Gateway Queue
    pub gateway_queue: GatewayQueue,
    /// User ID of the bot user, used to match when normalizing agents
    pub bot_user_id: u64,
    /// URL of a JSON document containing shortcode->emoji mappings compatible with Discord
    pub emoji_db_url: String,
}

/// Collection of secret values used to connect to services
#[derive(Debug, Deserialize, Clone)]
pub struct Secrets {
    /// Discord API bot token
    pub discord_token: String,
}

/// Collection of external services that this service connects to
#[derive(Debug, Deserialize, Clone)]
pub struct Services {
    /// Full AMQP URL to connect to the gateway queue at
    pub gateway_queue: String,
    /// HTTP URL of the logs/submission service that normalized LogEvents are forwarded to
    pub logs_submission: String,
}

/// Config options related to the Gateway Queue
#[derive(Default, Debug, Deserialize, Clone)]
pub struct GatewayQueue {
    /// Name of the durable queue that events get published to
    pub queue_name: String,
    /// Consumer tag used for the main event consumer
    pub consumer_tag: String,
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
            // Add in settings from the environment (with a prefix of LOGS_GATEWAY_NORMALIZE)
            // Eg.. `LOGS_GATEWAY_NORMALIZE_SERVICES__LOGS_IMPORT=X ./target/logs-gateway-normalize`
            // would set the `services.logs_import` key
            .merge(config::Environment::with_prefix("LOGS_GATEWAY_NORMALIZE").separator("__"))
            .context("Could not merge in values from the environment")?;
        let config = settings
            .try_into()
            .context("Loading the Configuration struct from the merged config failed")?;
        debug!("Configuration: {:?}", config);
        Ok(config)
    }
}
