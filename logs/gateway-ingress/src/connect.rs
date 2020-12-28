//! Contains utility functions that connect to external services,
//! used during service initialization and during potential reconnection

use crate::config::Configuration;
use crate::rpc::feature_gate::Client as FeatureGateClient;
use crate::rpc::uptime::Client as LogsUptimeClient;
use crate::INTENTS;
use anyhow::{Context, Result};
use backoff::future::FutureOperation as _;
use lapin::{Connection, ConnectionProperties};
use std::sync::Arc;
use twilight_gateway::Shard;

/// Attempts to initialize a gateway connection
pub async fn to_shard(config: Arc<Configuration>) -> Result<Shard> {
    let initialization_backoff = config.initialization_backoff.build();
    let shard_connect = || async {
        let mut shard = Shard::new(config.secrets.discord_token.clone(), *INTENTS);
        shard.start().await.map_err(|err| {
            log::warn!(
                "Couldn't start bot shard, retrying after backoff: {:?}",
                err
            );
            err
        })?;
        Ok(shard)
    };
    let shard = shard_connect
        .retry(initialization_backoff)
        .await
        .context("Could not start shard")?;
    log::info!("Created shard and preparing to listen for gateway events");
    Ok(shard)
}

/// Creates a new connection to Rabbit MQ
pub async fn to_queue(config: Arc<Configuration>) -> Result<Connection> {
    let initialization_backoff = config.initialization_backoff.build();
    let rmq_url = config.services.gateway_queue.clone();
    let rmq_connect = || async {
        let conn = Connection::connect(&rmq_url, ConnectionProperties::default())
            .await
            .map_err(|err| {
                log::warn!(
                    "Couldn't connect to RabbitMQ, retrying after backoff: {:?}",
                    err
                );
                err
            })?;
        Ok(conn)
    };
    let rmq_connection = rmq_connect
        .retry(initialization_backoff)
        .await
        .context("Could not connect to the RabbitMQ gateway queue")?;
    log::info!("Connected to RabbitMQ at {}", rmq_url);
    Ok(rmq_connection)
}

/// Creates a new connection to the feature gate service
pub async fn to_feature_gate(config: Arc<Configuration>) -> Result<FeatureGateClient> {
    let initialization_backoff = config.initialization_backoff.build();
    let feature_gate_url = config.services.feature_gate.clone();
    let connect = || async {
        let conn = FeatureGateClient::connect(feature_gate_url.clone())
            .await
            .map_err(|err| {
                log::warn!(
                    "Couldn't connect to feature-gate, retrying after backoff: {:?}",
                    err
                );
                err
            })?;
        Ok(conn)
    };
    let connection = connect
        .retry(initialization_backoff)
        .await
        .context("Could not connect to feature-gate")?;
    log::info!("Connected to feature-gate at {}", feature_gate_url);
    Ok(connection)
}

/// Creates a new connection to the logs/uptime service
pub async fn to_uptime_service(config: Arc<Configuration>) -> Result<LogsUptimeClient> {
    let initialization_backoff = config.initialization_backoff.build();
    let uptime_url = config.services.logs_uptime.clone();
    let connect = || async {
        let conn = LogsUptimeClient::connect(uptime_url.clone())
            .await
            .map_err(|err| {
                log::warn!(
                    "Couldn't connect to logs/uptime, retrying after backoff: {:?}",
                    err
                );
                err
            })?;
        Ok(conn)
    };
    let connection = connect
        .retry(initialization_backoff)
        .await
        .context("Could not connect to logs/uptime")?;
    log::info!("Connected to logs/uptime at {}", uptime_url);
    Ok(connection)
}
