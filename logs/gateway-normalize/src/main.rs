#![warn(clippy::all, clippy::pedantic, clippy::nursery)]
#![allow(clippy::future_not_send)]

mod audit_log;
mod config;
mod connect;
mod emoji;
mod event;
mod gateway;
mod rpc;
mod util;

use crate::config::Configuration;
use crate::event::NormalizedEvent;
use crate::gateway::{EventWithSource, ProcessingError, ProcessorFleet};
use crate::rpc::gateway_queue_lib::GatewayEvent;
use crate::rpc::logs::submission::Client as LogsImportClient;
use anyhow::Context;
use backoff::backoff::Backoff;
use backoff::ExponentialBackoff;
use futures::{StreamExt, TryFutureExt, TryStreamExt};
use lapin::options::{BasicAckOptions, BasicConsumeOptions, BasicQosOptions, BasicRejectOptions};
use lapin::types::FieldTable;
use lapin::{Channel, Connection};
use prost::Message;
use slog::Logger;
use sloggers::Config;
use std::convert::{Into, TryInto};
use std::sync::Arc;
use std::time::{Duration, Instant};
use tonic::IntoRequest;
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

    slog::info!(logger, "configuration loaded"; "path" => config_path);
    slog::debug!(logger, "configuration dump"; "config" => ?config);

    match run(config, logger.clone()).await {
        Ok(_) => slog::info!(logger, "service exited";),
        Err(err) => {
            slog::error!(logger, "an error ocurred during service running"; "error" => ?err)
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
    let client = Client::new(&config.secrets.discord_token);

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
    let processor = {
        let mut inner =
            gateway::ProcessorFleet::new(client, Arc::clone(&config), emojis, logger.clone());
        gateway::processors::register_all(&mut inner);
        Arc::new(inner)
    };

    // Initialize connections to external services
    let rmq_connection = connect::to_queue(Arc::clone(&config), logger.clone()).await?;
    let submission_client = connect::to_submission(Arc::clone(&config), logger.clone()).await?;

    // Consume raw gateway events from the Rabbit MQ queue
    // and normalize them via the fleet of processors
    normalize_gateway_events(
        rmq_connection,
        submission_client,
        processor,
        Arc::clone(&config),
        logger.clone(),
    )
    .await?;

    Ok(())
}

// Consumes raw gateway events from the Rabbit MQ queue
// and normalizes them via the fleet of processors
async fn normalize_gateway_events(
    queue_connection: Connection,
    submission_client: LogsImportClient,
    processor: Arc<ProcessorFleet>,
    config: Arc<Configuration>,
    logger: Logger,
) -> anyhow::Result<()> {
    // Keep looping over the lifecycle,
    // allowing for the state to be eagerly restored after a disconnection
    // using the same backoff as initialization.
    // If the backoff is exhausted or there is another error, then the entire future exits
    // (and the service will exit accordingly)
    let mut outer_rmq_connection = Some(queue_connection);
    let mut reconnection_state = ReconnectionState::new(
        config.reconnection_backoff.build(),
        config.reconnection_backoff_reset_threshold,
    );
    loop {
        let config = Arc::clone(&config);
        let processor = Arc::clone(&processor);
        let submission_client = submission_client.clone();

        // Wait for a backoff if needed
        reconnection_state.wait().await?;

        // Reconnect to the Rabbit MQ instance if needed
        let rmq_connection = if let Some(rmq) = outer_rmq_connection.take() {
            rmq
        } else {
            // This has an internal backoff loop,
            // so if it fails, then exit the service entirely
            connect::to_queue(Arc::clone(&config), logger.clone()).await?
        };

        // Run the consumer until an error occurs
        let consume = run_consume(
            rmq_connection,
            submission_client,
            processor,
            config,
            logger.clone(),
        );
        if let Err(err) = consume.await {
            slog::error!(
                logger,
                "Could not consume events due to queue error; attempting to reconnect";
                "error" => ?err,
            );
        }
    }
}

/// Represents a reconnection backoff utility wrapper
/// for a long-running task that should use an exponential backoff
/// when multiple failures occur in short succession,
/// but reset the backoff if the task has been running for a long time
/// (greater than the threshold)
#[derive(Debug)]
struct ReconnectionState {
    current: ExponentialBackoff,
    last_start: Option<Instant>,
    threshold: Duration,
}

impl ReconnectionState {
    const fn new(source: ExponentialBackoff, threshold: Duration) -> Self {
        Self {
            current: source,
            last_start: None,
            threshold,
        }
    }

    async fn wait(&mut self) -> anyhow::Result<()> {
        if let Some(last_start) = self.last_start {
            let running_time = Instant::now().duration_since(last_start);
            if running_time > self.threshold {
                // The running time was longer than the threshold to use the old backoff,
                // so reset it with the source backoff (from the config)
                self.current.reset();
            }

            match self.current.next_backoff() {
                None => return Err(anyhow::anyhow!("reconnection backoff elapsed")),
                Some(backoff) => tokio::time::sleep(backoff).await,
            }
        }

        // Mark the start of the next iteration
        self.last_start = Some(Instant::now());
        Ok(())
    }
}

/// Creates and runs a consumer,
/// stopping if the connection fails or there is a fatal normalization error
async fn run_consume(
    rmq_connection: Connection,
    submission_client: LogsImportClient,
    processor: Arc<ProcessorFleet>,
    config: Arc<Configuration>,
    logger: Logger,
) -> anyhow::Result<()> {
    // Start listening to the queue by creating a consumer
    // (we have to convert the Stream to a TryStream before using `try_for_each_concurrent`)
    let channel = create_channel(&rmq_connection, Arc::clone(&config)).await?;
    let consumer = channel
        .basic_consume(
            &config.gateway_queue.queue_name,
            &config.gateway_queue.consumer_tag,
            BasicConsumeOptions::default(),
            FieldTable::default(),
        )
        .await?;
    let event_try_stream = consumer.map(|result| result.map_err(anyhow::Error::new));
    event_try_stream
        .try_for_each_concurrent(
            Some(usize::from(config.queue_consumer_concurrency)),
            move |(_, delivery)| {
                let config = Arc::clone(&config);
                let processor = Arc::clone(&processor);
                let submission_client = submission_client.clone();
                let logger = logger.clone();
                async move {
                    let result = normalize(&delivery.data, processor, logger.clone())
                        .and_then(|event| {
                            let event_type_str = format!("{:?}", event.event_type);
                            let logger = logger.new(slog::o!(
                                "event_type" => event_type_str,
                                "guild_id" => event.guild_id,
                                "event_timestamp" => event.timestamp,
                            ));
                            submit_event(event, submission_client, config, logger)
                        })
                        .await;
                    // Acknowledge or reject the event based on the result
                    match result {
                        Ok(_) => {
                            delivery.ack(BasicAckOptions::default()).await?;
                            Ok(())
                        }
                        Err(rejection) => {
                            slog::info!(
                                logger,
                                "rejecting message due to error";
                                "requeueing" => rejection.should_requeue,
                                "error" => ?rejection.source,
                            );
                            delivery
                                .reject(BasicRejectOptions {
                                    requeue: rejection.should_requeue,
                                })
                                .await?;
                            Ok(())
                        }
                    }
                }
            },
        )
        .await?;

    Ok(())
}

/// Creates a channel to the message queue and sets the `QoS` appropriately
async fn create_channel(
    rmq_connection: &Connection,
    config: Arc<Configuration>,
) -> anyhow::Result<Channel> {
    // Create a temporary channel
    let rmq_channel = rmq_connection
        .create_channel()
        .await
        .context("could not create a new RabbitMQ channel")?;

    // Set the channel QOS appropriately
    rmq_channel
        .basic_qos(
            config.queue_consumer_concurrency.saturating_mul(2),
            BasicQosOptions { global: false },
        )
        .await
        .context("could not set the QoS level on the RabbitMQ queue")?;

    Ok(rmq_channel)
}

/// Error value created while consumption fails for a gateway event
/// and includes whether the gateway event should be requeued or not
#[derive(Debug)]
struct EventRejection {
    should_requeue: bool,
    source: anyhow::Error,
}

/// Attempts to normalize the raw bytes from the queue into a normalized event
/// ready to be submitted
async fn normalize(
    event_bytes: &[u8],
    processor: Arc<ProcessorFleet>,
    logger: Logger,
) -> anyhow::Result<NormalizedEvent, EventRejection> {
    let event = match GatewayEvent::decode(event_bytes) {
        Ok(event) => event,
        Err(err) => {
            slog::warn!(
                logger,
                "an error occurred while deserializing event from protobuf";
                "error" => ?err,
            );

            // Reject the message without requeuing
            return Err(EventRejection {
                should_requeue: false,
                source: err.into(),
            });
        }
    };

    let logger = logger.new(slog::o!(
        "gateway_event_type" => event.event_type.clone(),
        "event_id" => event.id.clone(),
        "guild_id" => event.guild_id,
    ));

    // Deserialize the inner JSON
    let event_with_source: EventWithSource = match event.try_into() {
        Ok(event) => event,
        Err(err) => {
            slog::warn!(
                logger,
                "an error occurred while decoding the inner source using MessagePack";
                "error" => ?err,
            );

            // Reject the message without requeuing
            return Err(EventRejection {
                should_requeue: false,
                source: err.into(),
            });
        }
    };

    // Run the processor fleet on the event to obtain a normalized event
    processor.normalize(event_with_source).await.map_err(|err| {
        if err.is_unexpected() {
            slog::warn!(
                logger,
                "event normalization failed for event";
                "error" => ?err,
            );
        }
        // Reject the message with/without requeuing depending on the error
        // (poison messages will be handled by max retry policy for quorum queue)
        let should_requeue = matches!(
            err,
            ProcessingError::FatalSourceError(_) | ProcessingError::NoAuditLogEntry(_)
        );
        EventRejection {
            should_requeue,
            source: err.into(),
        }
    })
}

/// Sends an event to the logs/submission service,
/// attempting to retry in the case of transient errors
async fn submit_event(
    event: NormalizedEvent,
    client: LogsImportClient,
    config: Arc<Configuration>,
    logger: Logger,
) -> anyhow::Result<(), EventRejection> {
    let send = || async {
        let mut client = client.clone();
        let response = client.submit_idempotent(event.clone().into_request()).await;
        rpc::into_backoff(response)
    };

    match backoff::future::retry(config.rpc_backoff.build(), send).await {
        Ok(response) => {
            let logger = logger.new(slog::o!("event_id" => response.id));
            slog::info!(logger, "submitted log event");
            slog::debug!(logger, "event dump"; "event" => ?event);
            Ok(())
        }
        Err(err) => {
            slog::error!(logger, "failed to submit log event"; "error" => ?err);
            Err(EventRejection {
                should_requeue: true,
                source: err.into(),
            })
        }
    }
}
