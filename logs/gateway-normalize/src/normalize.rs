use crate::config::Configuration;
use crate::connect;
use crate::event::NormalizedEvent;
use crate::gateway::{EventWithSource, ProcessorFleet};
use crate::reconnection;
use crate::rpc;
use crate::rpc::gateway_queue_lib::GatewayEvent;
use crate::rpc::logs::submission::Client as LogsImportClient;
use anyhow::Context;
use futures::{StreamExt, TryFutureExt, TryStreamExt};
use lapin::options::{BasicAckOptions, BasicConsumeOptions, BasicQosOptions, BasicRejectOptions};
use lapin::types::FieldTable;
use lapin::{Channel, Connection};
use prost::Message;
use slog::Logger;
use std::convert::{Into, TryInto};
use std::sync::Arc;
use tonic::IntoRequest;

/// Error value created while consumption fails for a gateway event
/// and includes whether the gateway event should be requeued or not
#[derive(Debug)]
struct Rejection {
    should_requeue: bool,
    source: anyhow::Error,
}

pub struct Consumer {
    submission_client: LogsImportClient,
    processor: Arc<ProcessorFleet>,
    config: Arc<Configuration>,
    logger: Logger,
}

impl Consumer {
    pub fn new(
        submission_client: LogsImportClient,
        processor: Arc<ProcessorFleet>,
        config: Arc<Configuration>,
        logger: Logger,
    ) -> Self {
        Self {
            submission_client,
            processor,
            config,
            logger,
        }
    }

    // Consumes raw gateway events from the Rabbit MQ queue
    // and normalizes them via the fleet of processors
    pub async fn run(&self, initial_queue_connection: Connection) -> anyhow::Result<()> {
        // Keep looping over the lifecycle,
        // allowing for the state to be eagerly restored after a disconnection
        // using the same backoff as initialization.
        // If the backoff is exhausted or there is another error, then the entire future exits
        // (and the service will exit accordingly)
        let mut outer_rmq_connection = Some(initial_queue_connection);
        let mut reconnection_state = reconnection::State::new(
            self.config.reconnection_backoff.build(),
            self.config.reconnection_backoff_reset_threshold,
        );
        loop {
            // Wait for a backoff if needed
            reconnection_state.wait().await?;

            // Reconnect to the Rabbit MQ instance if needed
            let rmq_connection = if let Some(rmq) = outer_rmq_connection.take() {
                rmq
            } else {
                // This has an internal backoff loop,
                // so if it fails, then exit the service entirely
                connect::connect_to_queue(Arc::clone(&self.config), self.logger.clone()).await?
            };

            // Run the consumer until an error occurs
            let consume_future = self.run_until_message_queue_error(rmq_connection);
            if let Err(err) = consume_future.await {
                slog::error!(
                    self.logger,
                    "Could not consume events due to queue error; attempting to reconnect";
                    "error" => ?err,
                );
            }
        }
    }

    /// Creates and runs a consumer for the message queue,
    /// stopping if the connection fails
    async fn run_until_message_queue_error(
        &self,
        rmq_connection: Connection,
    ) -> anyhow::Result<()> {
        // Start listening to the queue by creating a consumer
        // (we have to convert the Stream to a TryStream before using `try_for_each_concurrent`)
        let channel = self.create_channel(&rmq_connection).await?;
        let consumer = channel
            .basic_consume(
                &self.config.gateway_queue.queue_name,
                &self.config.gateway_queue.consumer_tag,
                BasicConsumeOptions::default(),
                FieldTable::default(),
            )
            .await?;
        let event_try_stream = consumer.map(|result| result.map_err(anyhow::Error::new));

        // Use try_for_each_concurrent to normalize each event coming from the message queue.
        // Note: because we're using `try_for_each_concurrent`,
        // it's possible for a future to only be partially awaited before being dropped.
        // In this case, it's fine because the message queue has consumer acknowledgements,
        // so normalization will be retried eventually,
        // even if a normalization future was dropped without finishing.
        event_try_stream
            .try_for_each_concurrent(
                Some(usize::from(self.config.queue_consumer_concurrency)),
                move |(_, delivery)| {
                    async move {
                        let result = self
                            .normalize(&delivery.data)
                            .and_then(|event| self.submit_event(event))
                            .await;
                        // Acknowledge or reject the event based on the result
                        match result {
                            Ok(_) => {
                                delivery.ack(BasicAckOptions::default()).await?;
                                Ok(())
                            }
                            Err(rejection) => {
                                slog::info!(
                                    self.logger,
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

    /// Sends an event to the logs/submission service,
    /// attempting to retry in the case of transient errors
    async fn submit_event(&self, event: NormalizedEvent) -> anyhow::Result<(), Rejection> {
        let event_type_str = format!("{:?}", event.event_type);
        let logger = self.logger.new(slog::o!(
            "event_type" => event_type_str,
            "guild_id" => event.guild_id,
            "event_timestamp" => event.timestamp,
        ));

        let send = || async {
            // Cloning the gRPC client is cheap; internally it is ref-counted
            let mut client = self.submission_client.clone();

            let response = client.submit_idempotent(event.clone().into_request()).await;
            rpc::into_backoff(response)
        };

        match backoff::future::retry(self.config.rpc_backoff.build(), send).await {
            Ok(response) => {
                let logger = logger.new(slog::o!("event_id" => response.id));
                slog::info!(logger, "submitted log event");
                slog::debug!(logger, "event dump"; "event" => ?event);
                Ok(())
            }
            Err(err) => {
                slog::error!(logger, "failed to submit log event"; "error" => ?err);
                Err(Rejection {
                    should_requeue: true,
                    source: err.into(),
                })
            }
        }
    }

    /// Attempts to normalize the raw bytes from the queue into a normalized event
    /// ready to be submitted
    async fn normalize(&self, event_bytes: &[u8]) -> anyhow::Result<NormalizedEvent, Rejection> {
        let event = match GatewayEvent::decode(event_bytes) {
            Ok(event) => event,
            Err(err) => {
                slog::warn!(
                    self.logger,
                    "an error occurred while deserializing event from protobuf";
                    "error" => ?err,
                );

                // Reject the message without requeuing
                return Err(Rejection {
                    should_requeue: false,
                    source: err.into(),
                });
            }
        };

        let logger = self.logger.new(slog::o!(
            "gateway_event_type" => event.event_type.clone(),
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
                return Err(Rejection {
                    should_requeue: false,
                    source: err.into(),
                });
            }
        };

        // Run the processor fleet on the event to obtain a normalized event
        self.processor
            .normalize(event_with_source)
            .await
            .map_err(|err| {
                if err.is_unexpected() {
                    slog::warn!(
                        logger,
                        "event normalization failed for event";
                        "error" => ?err,
                    );
                }
                // Reject the message with/without requeuing depending on the error
                // (poison messages will be handled by max retry policy for quorum queue)
                Rejection {
                    should_requeue: err.should_requeue(),
                    source: err.into(),
                }
            })
    }

    /// Creates a channel to the message queue and sets the `QoS` appropriately
    async fn create_channel(&self, rmq_connection: &Connection) -> anyhow::Result<Channel> {
        // Create a temporary channel
        let rmq_channel = rmq_connection
            .create_channel()
            .await
            .context("could not create a new RabbitMQ channel")?;

        // Set the channel QOS appropriately
        rmq_channel
            .basic_qos(
                self.config.queue_consumer_concurrency.saturating_mul(2),
                BasicQosOptions { global: false },
            )
            .await
            .context("could not set the QoS level on the RabbitMQ queue")?;

        Ok(rmq_channel)
    }
}
