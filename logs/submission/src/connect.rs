//! Contains utility functions that connect to external services,
//! used during service initialization

use crate::config::Configuration;
use crate::elasticsearch::{Client, PingError};
use crate::timeout::TimeoutOr;
use anyhow::Context;
use slog::Logger;
use std::sync::Arc;

/// Creates a new Elasticsearch client
/// and pings it to ensure that the connection is live.
pub async fn connect_to_elasticsearch(
    config: Arc<Configuration>,
    logger: Logger,
) -> anyhow::Result<Client> {
    let client = crate::elasticsearch::new_client(&config, logger.clone())
        .context("could not create elasticsearch client")?;

    let initialization_backoff = config.initialization.backoff.build();
    let timeout = config.initialization.attempt_timeout;
    let ping_elasticsearch = || async {
        match crate::timeout::timeout(timeout, client.ping()).await {
            Ok(_) => Ok(()),
            Err(err) => {
                match &err {
                    TimeoutOr::Timeout(timeout) => {
                        slog::warn!(
                            logger,
                            "pinging elasticsearch timed out";
                            "timeout" => ?timeout,
                        );
                    }
                    TimeoutOr::Other(PingError::Failed(inner_err)) =>  {
                            slog::warn!(
                                logger,
                                "pinging elasticsearch failed";
                                "error" => ?inner_err,
                            );
                    }
                    TimeoutOr::Other(PingError::ErrorStatusCode(status_code)) => {
                        slog::warn!(
                            logger,
                            "pinging elasticsearch failed with error status code";
                            "status_code" => ?status_code,
                        );
                    }
                }
                Err(backoff::Error::Transient(err))
            }
        }
    };

    backoff::future::retry(initialization_backoff, ping_elasticsearch)
        .await
        .context("could not ping elasticsearch to verify reachability after retrying")?;

    slog::info!(
        logger,
        "connected to Elasticsearch";
        "url" => &config.elasticsearch.url
    );
    Ok(client)
}
