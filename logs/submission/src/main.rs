#![warn(clippy::all, clippy::pedantic, clippy::nursery)]
#![allow(clippy::future_not_send)]

mod config;
mod connect;
mod elasticsearch;
mod rpc;
mod submission;

use crate::config::Configuration;
use crate::rpc::logs::submission::submission_service_server::{
    SubmissionService, SubmissionServiceServer,
};
use crate::rpc::logs::submission::{
    EventDeterministicIdParams, SubmitIdempotentRequest, SubmitIdempotentResponse, SubmittedEvent,
};
use anyhow::Context;
use crate::elasticsearch::Client;
use futures::{try_join, StreamExt};
use futures_batch::ChunksTimeoutStreamExt;
use slog::Logger;
use sloggers::Config;
use std::net::{IpAddr, Ipv4Addr, SocketAddr};
use std::sync::atomic::{AtomicUsize, Ordering};
use std::sync::Arc;
use tokio::sync::{mpsc, oneshot};
use tokio_stream::wrappers::UnboundedReceiverStream;
use tonic::transport::Server;
use tonic::{Request, Response, Status};

/// Loads the config and bootstraps the service
#[tokio::main]
async fn main() -> anyhow::Result<()> {
    // Parse the config
    let config_path = std::env::args().nth(1).expect(
        "no config path given \
        \nUsage: \
        \nlogs-submission [config-path]",
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

/// Attempts to connect to external services and prepare the submission pipeline
async fn run(config: Arc<Configuration>, logger: Logger) -> anyhow::Result<()> {
    // Connect to Elasticsearch
    let elasticsearch =
        Arc::new(connect::to_elasticsearch(Arc::clone(&config), logger.clone()).await?);

    // Create the channel that acts as a stream source for event processing
    let (event_tx, event_rx) = mpsc::unbounded_channel::<submission::Event>();

    // Start the server on the specified port
    let addr = SocketAddr::new(IpAddr::V4(Ipv4Addr::UNSPECIFIED), config.port);
    let submission_service =
        SubmissionServiceImpl::new(Arc::clone(&config), logger.clone(), event_tx);
    let serve_future = serve_grpc(submission_service, addr, logger.clone());

    // Start the event sink coroutine that handles actually performing the submission
    // with some batching/debouncing logic included
    let event_sink_future = submit_events(
        Arc::clone(&config),
        logger.clone(),
        event_rx,
        Arc::clone(&elasticsearch),
    );

    // Upload the mapping (schema) to Elasticsearch to create the index
    let create_index_future = submission::create_index(Arc::clone(&config), logger, elasticsearch);

    // Configure the index first before running the other futures
    create_index_future.await?;
    try_join!(serve_future, event_sink_future)?;

    Ok(())
}

/// Serves the main gRPC server
async fn serve_grpc(
    submission_service: SubmissionServiceImpl,
    addr: SocketAddr,
    logger: Logger,
) -> anyhow::Result<()> {
    let submission_server = SubmissionServiceServer::new(submission_service);
    let server = Server::builder().add_service(submission_server);

    slog::info!(logger, "serving gRPC server"; "address" => addr);
    server
        .serve(addr)
        .await
        .context("an error occurred while running the gRPC server")?;

    Ok(())
}

/// Consumes events from the shared channel and attempts to submit them durably to the ES data store.
/// Waits for `config.debounce_size` events to pile up
/// or for the oldest event to be waiting `config.debounce_period`
/// before sending a batch of events to elasticsearch and notifying the sender
async fn submit_events(
    config: Arc<Configuration>,
    logger: Logger,
    event_rx: mpsc::UnboundedReceiver<submission::Event>,
    elasticsearch: Arc<Client>,
) -> anyhow::Result<()> {
    let next_correlation_id = Arc::new(AtomicUsize::new(1));

    // Batch together events and then process them as batches
    slog::info!(
        logger,
        "starting batched sink of submission events";
        "debounce_size" => config.debounce_size,
        "debounce_period" => ?config.debounce_period
    );
    UnboundedReceiverStream::new(event_rx)
        .chunks_timeout(config.debounce_size, config.debounce_period)
        .for_each_concurrent(None, move |events| {
            let correlation_id = next_correlation_id.fetch_add(1, Ordering::SeqCst);
            let config = Arc::clone(&config);
            let elasticsearch = Arc::clone(&elasticsearch);
            let batch_submit = submission::BatchSubmit::new(
                correlation_id,
                config,
                &logger,
                elasticsearch,
            );
            async move {
                batch_submit.run(events).await;
            }
        })
        .await;

    Ok(())
}

/// Generates the log event ID from the ID parameters and event type,
/// formatted to a canonical format.
fn generate_id(id_params: EventDeterministicIdParams, event_type: i32) -> String {
    return format!(
        "lgev_t{:4x}_p{:8x}_s{:8x}",
        event_type, id_params.primary_id, id_params.secondary_id
    );
}

struct SubmissionServiceImpl {
    config: Arc<Configuration>,
    event_tx: mpsc::UnboundedSender<submission::Event>,
    logger: Logger,
}

impl SubmissionServiceImpl {
    fn new(
        config: Arc<Configuration>,
        logger: Logger,
        event_tx: mpsc::UnboundedSender<submission::Event>,
    ) -> Self {
        Self {
            config,
            event_tx,
            logger,
        }
    }
}

#[tonic::async_trait]
impl SubmissionService for SubmissionServiceImpl {
    async fn submit_idempotent(
        &self,
        request: Request<SubmitIdempotentRequest>,
    ) -> anyhow::Result<Response<SubmitIdempotentResponse>, tonic::Status> {
        let event = request
            .into_inner()
            .event
            .ok_or_else(|| Status::invalid_argument("no event given"))?;

        let SubmittedEvent {
            inner,
            channel_name,
            agent_metadata,
            subject_metadata,
            auxiliary_metadata,
            id_params,
        } = event;

        let inner = inner.ok_or_else(|| Status::invalid_argument("no inner event given"))?;
        let id_params = id_params.ok_or_else(|| Status::invalid_argument("no id params given"))?;

        // guild_id is the only required field
        if inner.guild_id == 0 {
            return Err(Status::invalid_argument("missing guild_id on inner event"));
        }

        let id = generate_id(id_params, inner.r#type);
        let logger = self.logger.new(slog::o!("event_id" => id.clone()));

        // TODO consume additional metadata in SubmittedEvent message
        //      and send to revision service

        // Create the one shot channel to wait on
        let (oneshot_tx, oneshot_rx) = oneshot::channel::<submission::OperationResult>();

        // Post the event to the shared channel
        let event = submission::Event {
            id: id.clone(),
            inner: Box::new(inner),
            notifier: oneshot_tx,
        };
        self.event_tx.send(event).map_err(|err| {
            slog::error!(
                logger,
                "send error when sending to shared submission channel";
                "error" => ?err,
            );
            Status::internal("internal channel error")
        })?;

        // Wait for the event to be submitted within a reasonable timeout
        match tokio::time::timeout(self.config.submission_wait_timeout, oneshot_rx).await {
            Ok(recv_result) => match recv_result {
                Ok(submit_result) => match submit_result {
                    Ok(correlation_id) => {
                        slog::info!(
                            logger,
                            "confirmed durable submission of event";
                            "correlation_id" => correlation_id,
                        );
                        Ok(Response::new(SubmitIdempotentResponse { id }))
                    }
                    Err(err) => {
                        slog::warn!(
                            logger,
                            "submission failed for event";
                            "details" => err.internal_details,
                            "status" => ?err.status,
                            "correlation_id" => err.correlation_id,
                        );
                        Err(err.status)
                    }
                },
                Err(err) => {
                    slog::error!(
                        logger,
                        "receive error when waiting for durable submission";
                        "error" => ?err,
                    );
                    Err(Status::internal("internal channel error"))
                }
            },
            Err(err) => {
                slog::error!(
                    logger,
                    "receive timed out when waiting for durable submission";
                    "error" => ?err,
                );
                Err(Status::deadline_exceeded("internal channel timed out"))
            }
        }
    }
}
