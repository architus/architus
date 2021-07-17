//! Contains utility functions that connect to external services,
//! used during service initialization and during potential reconnection

use crate::config::Configuration;
use anyhow::Context;
use elasticsearch::http::transport::Transport;
use elasticsearch::Elasticsearch;
use slog::Logger;
use std::sync::Arc;

/// Creates a new Elasticsearch client
/// and pings it to ensure that the connection is live.
pub async fn to_elasticsearch(config: Arc<Configuration>, logger: Logger) -> anyhow::Result<Elasticsearch> {
    let es_path = &config.services.elasticsearch;
    let es_transport =
        Transport::single_node(es_path).context("could not create elasticsearch client")?;
    let elasticsearch = Elasticsearch::new(es_transport);

    let initialization_backoff = config.initialization_backoff.build();
    let ping_elasticsearch = || async {
        elasticsearch
            .ping()
            .send()
            .await
            .map_err(|err| {
                slog::warn!(
                    logger,
                    "pinging elasticsearch failed";
                    "error" => ?err,
                );
                backoff::Error::Transient(err.into())
            })?
            .error_for_status_code()
            .map_err(|err| {
                slog::warn!(
                    logger,
                    "pinging elasticsearch returned non-success error code";
                    "error" => ?err,
                );
                backoff::Error::Transient(err.into())
            })?;
        Ok::<(), backoff::Error<anyhow::Error>>(())
    };

    backoff::future::retry(initialization_backoff, ping_elasticsearch)
        .await
        .context("could not ping elasticsearch to verify reachability after retrying")?;

    slog::info!(logger, "connected to Elasticsearch"; "path" => es_path);
    Ok(elasticsearch)
}
