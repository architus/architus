#![warn(clippy::all, clippy::pedantic, clippy::nursery)]
#![allow(clippy::future_not_send)]

mod config;
mod connect;
mod publish;
mod rpc;
mod uptime;

use crate::config::Configuration;
use crate::rpc::logs::uptime::Client as LogsUptimeClient;
use crate::uptime::active_guilds::ActiveGuilds;
use crate::uptime::connection::Tracker;
use crate::uptime::{Event as UptimeEvent, UpdateMessage};
use anyhow::{Context, Result};
use futures::{try_join, FutureExt, Stream, StreamExt, TryStreamExt};
use slog::Logger;
use sloggers::Config;
use std::future::Future;
use std::sync::Arc;
use tokio::sync::mpsc::{self, UnboundedSender};
use tokio_stream::wrappers::UnboundedReceiverStream;
use tonic::Request;
use twilight_gateway::{Event, EventTypeFlags};

/// Loads the config and bootstraps the service
#[tokio::main]
async fn main() -> Result<()> {
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

/// Attempts to initialize the bot and listen for gateway events
async fn run(config: Arc<Configuration>, logger: Logger) -> Result<()> {
    // Create the tracker and its update message channel
    let (connection_tracker, update_tx) = Tracker::new(Arc::clone(&config), logger.clone());

    // Build an event type flags bitset of all events to receive from the gateway
    let raw_events = EventTypeFlags::SHARD_PAYLOAD;
    let lifecycle_events = EventTypeFlags::GATEWAY_HEARTBEAT_ACK
        | EventTypeFlags::GUILD_CREATE
        | EventTypeFlags::GUILD_DELETE
        | EventTypeFlags::SHARD_CONNECTED
        | EventTypeFlags::SHARD_DISCONNECTED;
    let event_type_flags = lifecycle_events | raw_events;

    // Initialize connections to external services
    let (_shard, events) =
        connect::to_shard(Arc::clone(&config), logger.clone(), event_type_flags).await?;
    let rmq_connection = connect::to_queue(Arc::clone(&config), logger.clone()).await?;
    let feature_gate_client = connect::to_feature_gate(Arc::clone(&config), logger.clone()).await?;
    let uptime_service_client =
        connect::to_uptime_service(Arc::clone(&config), logger.clone()).await?;

    // Split up the stream of gateway events into lifecycle & raw events
    let (split_future, lifecycle_event_stream, raw_event_stream) =
        split_gateway_events(events, lifecycle_events, logger.clone());

    // Notify the connection tracker of the newly connected services
    update_tx
        .send(UpdateMessage::GatewayOnline)
        .context("could not send gateway online update to uptime tracker")?;
    update_tx
        .send(UpdateMessage::QueueOnline)
        .context("could not send queue online update to uptime tracker")?;

    let (active_guilds, uptime_rx) =
        ActiveGuilds::new(feature_gate_client, Arc::clone(&config), logger.clone());

    // Listen to incoming gateway events and start re-publishing them on the queue
    // (performing the primary purpose of the service)
    let publisher_instance = publish::Publisher::new(
        rmq_connection,
        active_guilds.clone(),
        Arc::clone(&config),
        logger.clone(),
        update_tx.clone(),
    );
    let publish_sink = publisher_instance
        .consume_events(raw_event_stream)
        .map(|_| Ok::<(), anyhow::Error>(()));

    // Listen for lifecycle events and send them to the uptime tracker
    let lifecycle_event_listener =
        process_lifecycle_events(lifecycle_event_stream, update_tx.clone());

    // Pipe the uptime events from the tracker into the active guild handler
    let uptime_events_stream = connection_tracker.stream_events();
    let unfiltered_uptime_pipe = active_guilds.pipe_uptime_events(uptime_events_stream);

    // Sink all uptime events to the uptime tracking service
    let uptime_events_sink = sink_uptime_events(
        Arc::clone(&config),
        logger,
        uptime_service_client,
        uptime_rx,
    );

    // Continuously poll the set of active guilds
    let active_guilds_poll = active_guilds.go_poll();

    // Run all futures until an error is encountered
    try_join!(
        split_future.map(|_| Ok::<(), anyhow::Error>(())),
        publish_sink,
        lifecycle_event_listener,
        unfiltered_uptime_pipe,
        uptime_events_sink,
        active_guilds_poll,
    )?;

    Ok(())
}

/// Listens for lifecycle events from the Gateway and sends corresponding update messages
///
/// on the shard mpsc channel, updating the stateful uptime tracker
async fn process_lifecycle_events(
    in_stream: impl Stream<Item = Event>,
    update_tx: UnboundedSender<UpdateMessage>,
) -> Result<()> {
    type SendError = tokio::sync::mpsc::error::SendError<UpdateMessage>;
    // Convert the stream into a TryStream
    let try_stream = StreamExt::map(in_stream, Ok::<Event, SendError>);
    try_stream
        .try_for_each(move |event| {
            let update_tx = update_tx.clone();
            async move {
                match event {
                    Event::ShardConnected(_) => update_tx.send(UpdateMessage::GatewayOnline),
                    Event::ShardDisconnected(_) => update_tx.send(UpdateMessage::GatewayOffline),
                    Event::GuildCreate(guild_create) => {
                        update_tx.send(UpdateMessage::GuildOnline(guild_create.id.0))
                    }
                    Event::GuildDelete(guild_delete) => {
                        update_tx.send(UpdateMessage::GuildOffline(guild_delete.id.0))
                    }
                    // Send heartbeat updates whenever we get Discord Gateway acks
                    // so that we know the connection is still alive
                    // (and we're not sending heartbeats into the void)
                    Event::GatewayHeartbeatAck => update_tx.send(UpdateMessage::GatewayHeartbeat),
                    _ => Ok(()),
                }
            }
        })
        .await
        .context("could not send update messages to uptime tracker")?;

    Ok(())
}

/// Acts as a stream sink for uptime events,
/// sending them to the uptime service
async fn sink_uptime_events(
    config: Arc<Configuration>,
    logger: Logger,
    uptime_service_client: LogsUptimeClient,
    in_stream: impl Stream<Item = UptimeEvent>,
) -> Result<()> {
    // Generate a unique session ID that is used to detect downtime after a forced shutdown
    let session: u64 = rand::random();
    let logger = logger.new(slog::o!("session_id" => session));
    slog::info!(logger, "generated random session ID for uptime events");

    // Note: we don't exit the service if this part fails;
    // this is an acceptable degradation
    in_stream
        .for_each_concurrent(None, move |event| {
            let uptime_service_client = uptime_service_client.clone();
            let config = Arc::clone(&config);
            let logger = logger.clone();
            async move {
                let request = event.into_request(session);
                slog::debug!(logger, "sending UptimeEvent to logs/uptime"; "request" => ?request);
                let send = || async {
                    let mut uptime_service_client = uptime_service_client.clone();
                    let response = uptime_service_client
                        .gateway_submit(Request::new(request.clone()))
                        .await;
                    rpc::into_backoff(response)
                };
                if let Err(err) = backoff::future::retry(config.rpc_backoff.build(), send).await {
                    slog::warn!(logger, "submitting UptimeEvent to logs/uptime failed"; "error" => ?err);
                }
            }
        })
        .await;

    Ok(())
}

/// Splits gateway events into two streams
/// depending on whether they match the given event type flags filter.
/// The future returned as the first return argument
/// must be awaited for the splitting to occur.
fn split_gateway_events(
    in_stream: impl Stream<Item = Event>,
    filter: EventTypeFlags,
    logger: Logger,
) -> (
    impl Future<Output = ()>,
    impl Stream<Item = Event>,
    impl Stream<Item = Event>,
) {
    let (matches_tx, matches_rx) = mpsc::unbounded_channel::<Event>();
    let (other_tx, other_rx) = mpsc::unbounded_channel::<Event>();
    let split_future = async move {
        in_stream
            .for_each(|event| async {
                let event_type: EventTypeFlags = event.kind().into();
                let send_result = if filter.contains(event_type) {
                    (matches_tx.send(event), "matches")
                } else {
                    (other_tx.send(event), "other")
                };

                if let (Err(send_err), side) = send_result {
                    slog::warn!(
                        logger,
                        "sending to downstream split event stream failed";
                        "side" => side,
                        "error" => ?send_err,
                    );
                }
            })
            .await;
    };

    (
        split_future,
        UnboundedReceiverStream::new(matches_rx),
        UnboundedReceiverStream::new(other_rx),
    )
}
