//! Contains utility functions that connect to external services,
//! used during service initialization and during potential reconnection

use crate::config::Configuration;
use crate::rpc::logs::submission::Client as LogsSubmissionClient;
use anyhow::Context;
use lapin::{Connection, ConnectionProperties};
use slog::Logger;
use std::sync::Arc;

/// Creates a new connection to Rabbit MQ
pub async fn connect_to_queue(
    config: Arc<Configuration>,
    logger: Logger,
) -> anyhow::Result<Connection> {
    let initialization_backoff = config.initialization_backoff.build();
    let rmq_url = config.services.gateway_queue.clone();
    let rmq_connect = || async {
        let conn = Connection::connect(&rmq_url, ConnectionProperties::default())
            .await
            .map_err(|err| {
                slog::warn!(
                    logger,
                    "couldn't connect to RabbitMQ, retrying after backoff";
                    "error" => ?err,
                );
                err
            })?;
        Ok(conn)
    };
    let rmq_connection = backoff::future::retry(initialization_backoff, rmq_connect)
        .await
        .context("could not connect to the RabbitMQ gateway queue")?;
    slog::info!(logger, "connected to RabbitMQ"; "rmq_url" => rmq_url);
    Ok(rmq_connection)
}

/// Creates a new connection to the logs/submission service
pub async fn connect_to_submission(
    config: Arc<Configuration>,
    logger: Logger,
) -> anyhow::Result<LogsSubmissionClient> {
    let initialization_backoff = config.initialization_backoff.build();
    let submission_url = config.services.logs_submission.clone();
    let connect = || async {
        let conn = LogsSubmissionClient::connect(submission_url.clone())
            .await
            .map_err(|err| {
                slog::warn!(
                    logger,
                    "couldn't connect to logs/submission, retrying after backoff";
                    "error" => ?err,
                );
                err
            })?;
        Ok(conn)
    };
    let connection = backoff::future::retry(initialization_backoff, connect)
        .await
        .context("could not connect to logs/submission")?;
    slog::info!(logger, "connected to logs/submission"; "submission_url" => submission_url);
    Ok(connection)
}
