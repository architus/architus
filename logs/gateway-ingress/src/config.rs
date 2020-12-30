//! Contains configuration options for the service that control its network topology
//! and internal behaviors

use anyhow::{Context, Result};
use architus_config_backoff::Backoff;
use deadpool::managed::PoolConfig;
use lapin::types::AMQPValue;
use log::{debug, info};
use serde::Deserialize;
use std::collections::BTreeMap;
use std::time::Duration;

/// Configuration object loaded upon startup
#[derive(Default, Debug, Deserialize, Clone)]
pub struct Configuration {
    /// Collection of secret values used to connect to services
    pub secrets: Secrets,
    /// Collection of external services that this service connects to
    pub services: Services,
    /// Parameters for the backoff used to connect to external services during initialization
    pub initialization_backoff: Backoff,
    /// Parameters for the backoff used to send RPC calls to other services
    pub rpc_backoff: Backoff,
    /// Name of the feature that enables indexing on a guild
    pub indexing_feature: String,
    /// Config options related to the Gateway Queue
    pub gateway_queue: GatewayQueue,
    /// Length of time that consecutive guild uptime events are grouped together in
    #[serde(with = "serde_humantime")]
    pub guild_uptime_debounce_delay: Duration,
    /// Size of the guild chunks to send to the feature-gate service to check their feature
    pub feature_gate_batch_check_size: usize,
    /// How long to keep offline guilds in the active guild cache
    /// (Allows for detecting guilds that enable/disable their indexing rapidly)
    #[serde(with = "serde_humantime")]
    pub active_guild_eviction_duration: Duration,
    /// The amount of time to wait between polls to the feature-gate service
    /// to retrieve the current status of all guilds and whether they have indexing enabled
    /// Lowering increases I/O on the feature-gate and lock contention on the processing hot-path
    /// while increasing response times for indexing enable/disable actions
    #[serde(with = "serde_humantime")]
    pub active_guilds_poll_interval: Duration,
}

/// Collection of secret values used to connect to services
#[derive(Default, Debug, Deserialize, Clone)]
pub struct Secrets {
    /// Discord bot token used to authenticate with the Gateway API
    pub discord_token: String,
}

/// Collection of external services that this service connects to
#[derive(Default, Debug, Deserialize, Clone)]
pub struct Services {
    /// Full AMQP URL to connect to the gateway queue at
    pub gateway_queue: String,
    /// HTTP URL of the feature-gate service
    pub feature_gate: String,
    /// HTTP URL of the logs/uptime service
    pub logs_uptime: String,
}

/// Config options related to the Gateway Queue
#[derive(Default, Debug, Deserialize, Clone)]
pub struct GatewayQueue {
    /// Name of the exchange that events are sent to
    pub exchange: String,
    /// Routing key for messages
    pub routing_key: String,
    /// Configuration for the connection pool that sits in front of a connection to the gateway queue
    pub connection_pool: PoolConfig,
    /// Name of the durable queue that events get published to
    pub queue_name: String,
    /// Whether the queue will retain messages (required when x-queue-type is 'quorum')
    pub durable: bool,
    /// Parameters to send along with queue declaration
    pub queue_parameters: Option<BTreeMap<String, AMQPValue>>,
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
            // Add in settings from the environment (with a prefix of LOGS_GATEWAY_INGRESS)
            // Eg.. `LOGS_GATEWAY_INGRESS_SECRETS__DISCORD_TOKEN=X ./target/logs-gateway-ingress`
            // would set the `secrets.discord_token` key
            .merge(config::Environment::with_prefix("LOGS_GATEWAY_INGRESS").separator("__"))
            .context("Could not merge in values from the environment")?;
        let config = settings
            .try_into()
            .context("Loading the Configuration struct from the merged config failed")?;
        debug!("Configuration: {:?}", config);
        Ok(config)
    }
}
