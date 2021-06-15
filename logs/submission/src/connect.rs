//! Contains utility functions that connect to external services,
//! used during service initialization and during potential reconnection

use crate::config::Configuration;
use anyhow::{Context, Result};
use elasticsearch::http::transport::Transport;
use elasticsearch::Elasticsearch;
use slog::Logger;
use std::sync::Arc;

/// Creates a new Elasticsearch client
/// (but doesn't appear to establish a connection yet)
pub fn to_elasticsearch(config: Arc<Configuration>, logger: Logger) -> Result<Elasticsearch> {
    let es_path = &config.services.elasticsearch;
    let es_transport =
        Transport::single_node(es_path).context("Could not connect to Elasticsearch")?;
    let elasticsearch = Elasticsearch::new(es_transport);
    slog::info!(logger, "connected to Elasticsearch"; "path" => es_path);

    Ok(elasticsearch)
}
