//! Contains utility functions that connect to external services,
//! used during service initialization and during potential reconnection

use crate::config::Configuration;
use crate::rpc::import::Client as LogsImportClient;
use anyhow::{Context, Result};
use backoff::future::FutureOperation as _;
use lapin::{Connection, ConnectionProperties};
use std::sync::Arc;

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

/// Creates a new connection to the logs/import service
pub async fn to_import(config: Arc<Configuration>) -> Result<LogsImportClient> {
    let initialization_backoff = config.initialization_backoff.build();
    let import_url = config.services.logs_import.clone();
    let connect = || async {
        let conn = LogsImportClient::connect(import_url.clone())
            .await
            .map_err(|err| {
                log::warn!(
                    "Couldn't connect to logs/import, retrying after backoff: {:?}",
                    err
                );
                err
            })?;
        Ok(conn)
    };
    let connection = connect
        .retry(initialization_backoff)
        .await
        .context("Could not connect to logs/import")?;
    log::info!("Connected to logs/import at {}", import_url);
    Ok(connection)
}
