#![warn(clippy::all, clippy::pedantic, clippy::nursery)]
#![allow(clippy::future_not_send)]

mod active_guilds;
mod config;
mod connect;
mod filter;
mod publish;
mod queue;
mod rpc;

use crate::active_guilds::ActiveGuilds;
use crate::config::Configuration;
use crate::rpc::gateway_queue_lib::GatewayEvent;
use anyhow::Context;
use futures::{Stream, StreamExt};
use slog::Logger;
use sloggers::Config;
use std::convert::TryInto;
use std::sync::Arc;
use std::time::{SystemTime, UNIX_EPOCH};
use twilight_gateway::{Event, EventTypeFlags};

/// Loads the config and bootstraps the service
#[tokio::main]
async fn main() -> anyhow::Result<()> {
    // Parse the config
    let config_path = std::env::args().nth(1).expect(
        "no config path given \
        \nUsage: \
        \nlogs-gateway-ingress [config-path]",
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

/// Attempts to initialize the bot and listen for gateway events
async fn run(config: Arc<Configuration>, logger: Logger) -> anyhow::Result<()> {
    // Initialize connections to external services
    let (_shard, raw_event_stream) = connect::connect_to_shard(
        Arc::clone(&config),
        logger.clone(),
        EventTypeFlags::SHARD_PAYLOAD,
    )
    .await?;
    let initial_rmq_connection =
        connect::connect_to_queue(Arc::clone(&config), logger.clone()).await?;
    let feature_gate_client =
        connect::connect_to_feature_gate(Arc::clone(&config), logger.clone()).await?;

    let active_guilds = Arc::new(ActiveGuilds::new(
        feature_gate_client.clone(),
        Arc::clone(&config),
        logger.clone(),
    ));

    let gateway_ingress = GatewayIngress::new(
        logger.clone(),
        Arc::clone(&config),
        Arc::clone(&active_guilds),
    );

    // Listen to incoming gateway events and start re-publishing them on the queue
    // (performing the primary purpose of the service)
    let handle_event_future =
        gateway_ingress.handle_raw_events(raw_event_stream, initial_rmq_connection);

    // Continuously poll the set of active guilds
    let active_guilds_poll_future = active_guilds.go_poll();

    // Run all futures
    futures::join!(handle_event_future, active_guilds_poll_future);

    Ok(())
}

struct GatewayIngress {
    logger: Logger,
    config: Arc<Configuration>,
    active_guilds: Arc<ActiveGuilds>,
}

impl GatewayIngress {
    fn new(logger: Logger, config: Arc<Configuration>, active_guilds: Arc<ActiveGuilds>) -> Self {
        Self {
            logger,
            config,
            active_guilds,
        }
    }

    /// Handles filtering, converting, and publishing events
    /// to the durable message queue for later processing in the logs-gateway-normalize service
    async fn handle_raw_events(
        &self,
        raw_event_stream: impl Stream<Item = Event>,
        initial_rmq_connection: lapin::Connection,
    ) {
        // Converts the raw event to a partially-deserialized gateway event
        let converted_event_stream = raw_event_stream.filter_map(|raw_event| async {
            // Note the time of ingestion
            let time_ms: u64 = SystemTime::now()
                .duration_since(UNIX_EPOCH)
                .expect("Time went backwards")
                .as_millis()
                .try_into()
                .expect("System time could not fit into u64");

            match crate::filter::try_convert_raw_event(raw_event, time_ms) {
                Ok(event_option) => {
                    match &event_option {
                        Some(event) => {
                            slog::debug!(
                                self.logger,
                                "got event from gateway";
                                "guild_id" => event.guild_id,
                                "event_timestamp" => event.ingress_timestamp,
                                "event_type" => &event.event_type,
                            );
                        },
                        None => {
                            slog::debug!(
                                self.logger,
                                "got event from gateway but dropping";
                                "event_timestamp" => time_ms,
                            );
                        },
                    }
                    event_option
                },
                Err(err) => {
                    err.log(&self.logger);
                    None
                }
            }
        });

        // Filter events by whether or not the guild has indexing enabled
        // (which is the same as being considered "active")
        let filtered_event_stream = converted_event_stream.filter_map(|event| async {
            // ActiveGuilds::is_active requires `Arc<Self>` as the method target
            // (to ensure a valid self-access even if this future is dropped),
            // so we need to clone it before calling the method.
            let active_guilds = Arc::clone(&self.active_guilds);
            if active_guilds.is_active(event.guild_id).await {
                Some(event)
            } else {
                slog::debug!(
                    self.logger,
                    "dropping event due to guild not being active";
                    "guild_id" => event.guild_id,
                    "event_timestamp" => event.ingress_timestamp,
                    "event_type" => &event.event_type,
                );
                None
            }
        });

        let (event_bounded_queue, bounded_event_stream) = queue::BoundedQueue::<GatewayEvent>::new(
            queue::BoundedQueueConfig {
                identifier: String::from("gateway events"),
                max_size: self.config.raw_events.queue_length,
                warning_threshold: self.config.raw_events.warn_threshold,
                watch_size_interval: self.config.raw_events.watch_period,
            },
            &self.logger,
        );
        let watch_size_future = event_bounded_queue.watch_size();
        let pipe_future = event_bounded_queue.pipe_in(filtered_event_stream);

        let publisher = publish::Publisher::new(
            initial_rmq_connection,
            Arc::clone(&self.config),
            self.logger.clone(),
        );

        // This blocks indefinitely
        let consume_future = publisher.consume_events(bounded_event_stream);

        futures::join!(watch_size_future, pipe_future, consume_future);
    }
}
