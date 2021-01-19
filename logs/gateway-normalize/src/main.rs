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
use crate::emoji::EmojiDb;
use crate::event::NormalizedEvent;
use crate::gateway::{ProcessingError, ProcessorFleet};
use crate::rpc::submission::Client as LogsImportClient;
use anyhow::{Context, Result};
use backoff::backoff::Backoff;
use backoff::future::FutureOperation as _;
use backoff::ExponentialBackoff;
use chrono::{DateTime, NaiveDateTime, Utc};
use futures::{StreamExt, TryFutureExt, TryStreamExt};
use gateway_queue_lib::GatewayEvent;
use lapin::options::{BasicAckOptions, BasicConsumeOptions, BasicQosOptions, BasicRejectOptions};
use lapin::types::FieldTable;
use lapin::{Channel, Connection};
use std::convert::{Into, TryFrom};
use std::sync::Arc;
use std::time::{Duration, Instant};
use tonic::IntoRequest;
use twilight_http::Client;

/// Loads the config and bootstraps the service
#[tokio::main]
async fn main() -> Result<()> {
    env_logger::init();

    // Parse the config
    let config_path = std::env::args().nth(1).expect(
        "no config path given \
        \nUsage: \
        \nlogs-gateway-normalize [config-path]",
    );
    let config = Arc::new(Configuration::try_load(config_path)?);
    run(config).await
}

/// Runs the main logic of the service,
/// acting as a consumer for the Rabbit MQ gateway-queue messages
/// and running them through a processing pipeline
/// before forwarding them to the submission service
async fn run(config: Arc<Configuration>) -> Result<()> {
    // Create a Discord API client
    let client = Client::new(&config.secrets.discord_token);

    // Load the emoji database
    let emojis = Arc::new(EmojiDb::load(&config.emoji_db_url).await?);
    log::info!(
        "Downloaded emoji shortcode mappings from {}",
        config.emoji_db_url
    );

    // Initialize the gateway event processor
    // and register all known gateway event handlers
    // (see gateway/processors.rs)
    let processor = {
        let mut inner = gateway::ProcessorFleet::new(client, Arc::clone(&config), emojis);
        gateway::processors::register_all(&mut inner);
        Arc::new(inner)
    };

    // Initialize connections to external services
    let rmq_connection = connect::to_queue(Arc::clone(&config)).await?;
    let submission_client = connect::to_submission(Arc::clone(&config)).await?;

    // Consume raw gateway events from the Rabbit MQ queue
    // and normalize them via the fleet of processors
    normalize_gateway_events(
        rmq_connection,
        submission_client,
        processor,
        Arc::clone(&config),
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
) -> Result<()> {
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
            connect::to_queue(Arc::clone(&config)).await?
        };

        // Run the consumer until an error occurs
        let consume = run_consume(rmq_connection, submission_client, processor, config);
        if let Err(err) = consume.await {
            log::error!(
                "Could not consume events due to queue error; attempting to reconnect: `{:?}`",
                err
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

    async fn wait(&mut self) -> Result<()> {
        if let Some(last_start) = self.last_start {
            let running_time = Instant::now().duration_since(last_start);
            if running_time > self.threshold {
                // The running time was longer than the threshold to use the old backoff,
                // so reset it with the source backoff (from the config)
                self.current.reset();
            }

            match self.current.next_backoff() {
                None => return Err(anyhow::anyhow!("reconnection backoff elapsed")),
                Some(backoff) => tokio::time::delay_for(backoff).await,
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
) -> Result<()> {
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
                async move {
                    let result = normalize(&delivery.data, processor)
                        .and_then(|event| submit_event(event, submission_client, config))
                        .await;
                    // Acknowledge or reject the event based on the result
                    match result {
                        Ok(_) => {
                            delivery.ack(BasicAckOptions::default()).await?;
                            Ok(())
                        }
                        Err(rejection) => {
                            let requeue_msg = if rejection.should_requeue {
                                "requeuing"
                            } else {
                                "not requeuing"
                            };
                            log::debug!(
                                "Rejecting message due to error ({}): {:?}",
                                requeue_msg,
                                rejection.source
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
) -> Result<Channel> {
    // Create a temporary channel
    let rmq_channel = rmq_connection
        .create_channel()
        .await
        .context("Could not create a new RabbitMQ channel")?;

    // Set the channel QOS appropriately
    rmq_channel
        .basic_qos(
            config.queue_consumer_concurrency.saturating_mul(2),
            BasicQosOptions { global: false },
        )
        .await
        .context("Could not set the QoS level on the RabbitMQ queue")?;

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
) -> Result<NormalizedEvent, EventRejection> {
    let event: GatewayEvent = match rmp_serde::from_read_ref(event_bytes) {
        Ok(event) => event,
        Err(err) => {
            log::warn!(
                "An error occurred while deserializing event from MessagePack: {:?}",
                err
            );

            // Reject the message without requeuing
            return Err(EventRejection {
                should_requeue: false,
                source: err.into(),
            });
        }
    };

    log::info!(
        "{}",
        serde_json::to_string(&event)
            .ok()
            .unwrap_or_else(|| String::from(""))
    );

    // Run the processor fleet on the event to obtain a normalized event
    processor.normalize(event).await.map_err(|err| {
        if err.is_unexpected() {
            log::warn!("Event normalization failed for event: {:?}", err);
        }
        // Reject the message with/without requeuing depending on the error
        // (poison messages will be handled by max retry policy for quorum queue)
        let should_requeue =
            matches!(err, ProcessingError::FatalSourceError(_) | ProcessingError::NoAuditLogEntry(_));
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
) -> Result<(), EventRejection> {
    let timestamp = event.timestamp;
    let id = event.id;
    let send = || async {
        let mut client = client.clone();
        let response = client.submit_idempotent(event.clone().into_request()).await;
        rpc::into_backoff(response)
    };

    match send.retry(config.rpc_backoff.build()).await {
        Ok(_) => {
            match readable_timestamp(timestamp) {
                Ok(timestamp) => {
                    log::info!(
                        "Submitted log event type {:?} '{}' at {}",
                        event.event_type,
                        id,
                        timestamp
                    );
                    log::debug!("Actual event: {:?}", event);
                }
                Err(err) => {
                    log::warn!(
                        "Submitted log event type {:?} '{}' at invalid time ({}): {:?}",
                        event.event_type,
                        id,
                        timestamp,
                        err
                    );
                    log::info!("Actual event: {:?}", event);
                }
            };
            Ok(())
        }
        Err(err) => {
            log::warn!("Failed to submit log event: {:?}", err);
            Err(EventRejection {
                should_requeue: true,
                source: err.into(),
            })
        }
    }
}

/// Attempts to create a readable timestamp string from the given Unix ms epoch
fn readable_timestamp(timestamp: u64) -> Result<String> {
    let sec =
        i64::try_from(timestamp / 1_000).context("Could not convert timestamp seconds to i64")?;
    let nano_sec = u32::try_from((timestamp % 1_000).saturating_mul(1_000_000))
        .context("Could not convert timestamp nanoseconds to u32")?;
    let naive_datetime = NaiveDateTime::from_timestamp_opt(sec, nano_sec)
        .context("Could not convert timestamp to Naive DateTime")?;
    let datetime: DateTime<Utc> = DateTime::from_utc(naive_datetime, Utc);
    Ok(datetime.format("%Y-%m-%d %H:%M:%S").to_string())
}
