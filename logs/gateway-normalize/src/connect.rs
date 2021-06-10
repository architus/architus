//! Contains utility functions that connect to external services,
//! used during service initialization and during potential reconnection

use crate::config::Configuration;
use crate::rpc::logs::submission::Client as LogsSubmissionClient;
use anyhow::{Context, Result};
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
    let rmq_connection = backoff::future::retry(initialization_backoff, rmq_connect)
        .await
        .context("Could not connect to the RabbitMQ gateway queue")?;
    log::info!("Connected to RabbitMQ at {}", rmq_url);
    Ok(rmq_connection)
}

/// Creates a new connection to the logs/submission service
pub async fn to_submission(config: Arc<Configuration>) -> Result<LogsSubmissionClient> {
    let initialization_backoff = config.initialization_backoff.build();
    let submission_url = config.services.logs_submission.clone();
    let connect = || async {
        let conn = LogsSubmissionClient::connect(submission_url.clone())
            .await
            .map_err(|err| {
                log::warn!(
                    "Couldn't connect to logs/submission, retrying after backoff: {:?}",
                    err
                );
                err
            })?;
        Ok(conn)
    };
    let connection = backoff::future::retry(initialization_backoff, connect)
        .await
        .context("Could not connect to logs/submission")?;
    log::info!("Connected to logs/submission at {}", submission_url);
    Ok(connection)
}
