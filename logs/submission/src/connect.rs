//! Contains utility functions that connect to external services,
//! used during service initialization and during potential reconnection

use crate::config::Configuration;
use crate::elasticsearch::{Client, PingError};
use anyhow::Context;
use slog::Logger;
use std::sync::Arc;

/// Creates a new Elasticsearch client
/// and pings it to ensure that the connection is live.
pub async fn to_elasticsearch(
    config: Arc<Configuration>,
    logger: Logger,
) -> anyhow::Result<Client> {
    let client = crate::elasticsearch::new_client(Arc::clone(&config), logger.clone())
        .context("could not create elasticsearch client")?;

    let initialization_backoff = config.initialization_backoff.build();
    let ping_elasticsearch = || async {
        match client.ping().await {
            Ok(_) => Ok(()),
            Err(err) => match &err {
                PingError::Failed(inner_err) => {
                    slog::warn!(
                        logger,
                        "pinging elasticsearch failed";
                        "error" => ?inner_err,
                    );
                    Err(backoff::Error::Transient(err))
                }
                PingError::ErrorStatusCode(status_code) => {
                    slog::warn!(
                        logger,
                        "pinging elasticsearch failed with error status code";
                        "status_code" => ?status_code,
                    );
                    Err(backoff::Error::Transient(err))
                }
            }
        }
    };

    backoff::future::retry(initialization_backoff, ping_elasticsearch)
        .await
        .context("could not ping elasticsearch to verify reachability after retrying")?;

    slog::info!(
        logger,
        "connected to Elasticsearch";
        "path" => &config.services.elasticsearch
    );
    Ok(client)
}
