#![warn(clippy::all, clippy::pedantic, clippy::nursery)]

mod config;
mod connect;
mod event;
mod gateway;
mod rpc;

use crate::config::Configuration;
use crate::event::NormalizedEvent;
use crate::gateway::{ProcessingError, Processor};
use crate::rpc::import::Client as LogsImportClient;
use anyhow::{Context, Result};
use backoff::future::FutureOperation as _;
use chrono::{DateTime, NaiveDateTime, Utc};
use futures::{StreamExt, TryFutureExt, TryStreamExt};
use gateway_queue_lib::GatewayEvent;
use lapin::options::{
    BasicAckOptions, BasicConsumeOptions, BasicQosOptions, BasicRejectOptions, QueueDeclareOptions,
};
use lapin::types::FieldTable;
use lapin::{Channel, Connection};
use std::convert::{Into, TryFrom};
use std::sync::Arc;
use tonic::IntoRequest;

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
/// acting as a consumer for the RabbitMQ gateway-queue messages
/// and running them through a processing pipeline
/// before forwarding them to the import service
async fn run(config: Arc<Configuration>) -> Result<()> {
    // Initialize the gateway event processor
    // and register all known gateway event handlers
    // (see gateway/processors.rs)
    // TODO connect to Discord API
    let processor_inner = gateway::Processor::new();
    let processor = Arc::new(gateway::sub_processors::register_all(processor_inner));

    // Initialize connections to external services
    let rmq_connection = connect::to_queue(Arc::clone(&config)).await?;
    let import_client = connect::to_import(Arc::clone(&config)).await?;

    // Consume raw gateway events from the Rabbit MQ queue
    // and normalize them via the fleet of processors
    normalize_gateway_events(
        rmq_connection,
        import_client,
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
    import_client: LogsImportClient,
    processor: Arc<Processor>,
    config: Arc<Configuration>,
) -> Result<()> {
    // Keep looping over the lifecycle,
    // allowing for the state to be eagerly restored after a disconnection
    // using the same backoff as initialization.
    // If the backoff is exhausted or there is another error, then the entire future exits
    // (and the service will exit accordingly)
    let mut outer_rmq_connection = Some(queue_connection);
    loop {
        let config = Arc::clone(&config);
        let processor = Arc::clone(&processor);
        let import_client = import_client.clone();

        // Reconnect to the Rabbit MQ instance if needed
        let rmq_connection = if let Some(rmq) = outer_rmq_connection.take() {
            rmq
        } else {
            connect::to_queue(Arc::clone(&config)).await?
        };

        // Declare the RMQ queue to consume incoming events from
        // and re-use the channel created to do so
        let channel = declare_event_queue(&rmq_connection, Arc::clone(&config)).await?;

        // Start listening to the queue by creating a consumer
        // (we have to convert the Stream to a TryStream before using `try_for_each_concurrent`)
        let consumer = channel
            .basic_consume(
                &config.gateway_queue.queue_name,
                &config.gateway_queue.consumer_tag,
                BasicConsumeOptions::default(),
                FieldTable::default(),
            )
            .await?;
        let event_try_stream = consumer.map(|result| result.map_err(anyhow::Error::new));
        let consume = event_try_stream.try_for_each_concurrent(
            Some(usize::from(config.queue_consumer_concurrency)),
            move |(_, delivery)| {
                let config = Arc::clone(&config);
                let processor = Arc::clone(&processor);
                let import_client = import_client.clone();
                async move {
                    let result = normalize(&delivery.data, processor)
                        .and_then(|event| import_event(event, import_client, config))
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
                                rejection.inner
                            );
                            delivery
                                .reject(BasicRejectOptions {
                                    requeue: rejection.should_requeue,
                                    ..BasicRejectOptions::default()
                                })
                                .await?;
                            Ok(())
                        }
                    }
                }
            },
        );

        if let Err(err) = consume.await {
            log::error!(
                "Could not consume event due to queue error; attempting to reconnect: `{:?}`",
                err
            );
        }
    }
}

/// Declares the Rabbit MQ queue, which is done during initialization of the Rabbit MQ connection
async fn declare_event_queue(
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

    // Declare the queue
    let queue_name = &config.gateway_queue.queue_name;
    let queue_options = QueueDeclareOptions {
        durable: true,
        ..QueueDeclareOptions::default()
    };
    rmq_channel
        .queue_declare(queue_name, queue_options, FieldTable::default())
        .await
        .context("Could not declare the RabbitMQ queue")?;

    log::info!("Declared RabbitMQ queue {}", queue_name);
    Ok(rmq_channel)
}

/// Error value created while consumption fails for a gateway event
/// and includes whether the gateway event should be requeued or not
#[derive(Debug)]
struct EventRejection {
    should_requeue: bool,
    inner: anyhow::Error,
}

/// Attempts to normalize the raw bytes from the queue into a normalized event
/// ready to be imported
async fn normalize(
    event_bytes: &[u8],
    processor: Arc<Processor>,
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
                inner: err.into(),
            });
        }
    };

    // Run the processor fleet on the event to obtain a normalized event
    processor.normalize(event).await.map_err(|err| {
        log::warn!("Event normalization failed for event: {:?}", err);
        // Reject the message with/without requeuing depending on the error
        // (poison messages will be handled by max retry policy for quorum queue)
        let should_requeue = match err {
            ProcessingError::SubProcessorNotFound(_) => false,
            ProcessingError::NoGuildId(_) => false,
            _ => true,
        };
        EventRejection {
            should_requeue,
            inner: err.into(),
        }
    })
}

/// Sends an event to the logs/import service,
/// attempting to retry in the case of transient errors
async fn import_event(
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
                    log::info!("Submitted log event '{}' at {}", id, timestamp);
                    log::debug!("Actual event: {:?}", event);
                }
                Err(err) => {
                    log::warn!(
                        "Submitted log event '{}' at invalid time ({}): {:?}",
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
                inner: err.into(),
            })
        }
    }
}

/// Attempts to create a readable timestamp string from the given Unix ms epoch
fn readable_timestamp(timestamp: u64) -> Result<String> {
    let sec =
        i64::try_from(timestamp / 1_000).context("Could not convert timestamp seconds to i64")?;
    let nsec = u32::try_from((timestamp % 1_000).saturating_mul(1_000_000))
        .context("Could not convert timestamp nanoseconds to u32")?;
    let naive_datetime = NaiveDateTime::from_timestamp_opt(sec, nsec)
        .context("Could not convert timestamp to Naive DateTime")?;
    let datetime: DateTime<Utc> = DateTime::from_utc(naive_datetime, Utc);
    Ok(datetime.format("%Y-%m-%d %H:%M:%S").to_string())
}
