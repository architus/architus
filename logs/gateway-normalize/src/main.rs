#![warn(clippy::all, clippy::pedantic, clippy::nursery)]
#![allow(clippy::future_not_send)]

mod audit_log;
mod config;
mod connect;
mod emoji;
mod gateway;
mod normalize;
mod reconnection;
mod rpc;
mod util;

use crate::config::Configuration;
use anyhow::Context;
use slog::Logger;
use sloggers::Config;
use std::sync::Arc;
use twilight_http::Client;

/// Loads the config and bootstraps the service
#[tokio::main]
async fn main() -> anyhow::Result<()> {
    // Parse the config
    let config_path = std::env::args().nth(1).expect(
        "no config path given \
        \nUsage: \
        \nlogs-gateway-normalize [config-path]",
    );
    let config = Arc::new(Configuration::try_load(&config_path)?);

    // Set up the logger from the config
    let logger = config
        .logging
        .build_logger()
        .context("could not build logger from config values")?;

    slog::info!(
        logger,
        "starting service";
        "config_path" => config_path,
        "arguments" => ?std::env::args().collect::<Vec<_>>(),
    );
    slog::debug!(logger, "configuration dump"; "config" => ?config);
    slog::debug!(logger, "env dump"; "env" => ?std::env::vars().collect::<Vec<_>>());

    match run(config, logger.clone()).await {
        Ok(_) => slog::info!(logger, "service exited";),
        Err(err) => {
            slog::error!(
                logger,
                "an error occurred during service execution";
                "error" => ?err,
            );
        }
    }
    Ok(())
}

/// Runs the main logic of the service,
/// acting as a consumer for the Rabbit MQ gateway-queue messages
/// and running them through a processing pipeline
/// before forwarding them to the submission service
async fn run(config: Arc<Configuration>, logger: Logger) -> anyhow::Result<()> {
    // Create a Discord API client
    let client = Client::new(config.secrets.discord_token.clone());

    // Load the emoji database
    let emojis = Arc::new(emoji::Db::load(&config.emoji_db_url).await?);
    slog::info!(
        logger,
        "downloaded emoji shortcode mappings";
        "emoji_db_url" => &config.emoji_db_url,
    );

    // Initialize the gateway event processor
    // and register all known gateway event handlers
    // (see gateway/processors.rs)
    let processor_fleet = {
        let mut inner =
            gateway::ProcessorFleet::new(client, Arc::clone(&config), emojis, logger.clone());
        gateway::processors::register_all(&mut inner);
        Arc::new(inner)
    };

    // Initialize connections to external services
    let initial_rmq_connection =
        connect::connect_to_queue(Arc::clone(&config), logger.clone()).await?;
    let submission_client =
        connect::connect_to_submission(Arc::clone(&config), logger.clone()).await?;

    // Consume raw gateway events from the Rabbit MQ queue
    // and normalize them via the fleet of processors
    let consumer = normalize::Consumer::new(
        submission_client,
        processor_fleet,
        Arc::clone(&config),
        logger.clone(),
    );
    consumer.run(initial_rmq_connection).await?;

    Ok(())
}
