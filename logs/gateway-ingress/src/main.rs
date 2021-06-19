#![warn(clippy::all, clippy::pedantic, clippy::nursery)]
#![allow(clippy::future_not_send)]

mod config;
mod connect;
mod rpc;
mod uptime;

use crate::config::Configuration;
use crate::rpc::logs::uptime::Client as LogsUptimeClient;
use crate::uptime::active_guilds::ActiveGuilds;
use crate::uptime::connection::Tracker;
use crate::uptime::{Event as UptimeEvent, UpdateMessage};
use anyhow::{anyhow, Context, Result};
use architus_amqp_pool::{Manager, Pool, PoolError};
use architus_id::{time, HoarFrost, IdProvisioner};
use deadpool::Runtime;
use futures::{try_join, Stream, StreamExt, TryStreamExt};
use gateway_queue_lib::GatewayEventOwned;
use lapin::options::{BasicPublishOptions, QueueDeclareOptions};
use lapin::types::FieldTable;
use lapin::{BasicProperties, Channel, Connection};
use lazy_static::lazy_static;
use slog::Logger;
use sloggers::Config;
use std::collections::BTreeMap;
use std::sync::Arc;
use tokio::sync::mpsc::UnboundedSender;
use tonic::Request;
use twilight_gateway::{Event, EventType, EventTypeFlags, Intents, Shard};
use twilight_model::gateway::event::gateway::GatewayEventDeserializer;
use twilight_model::gateway::event::shard::Payload;
use twilight_model::gateway::OpCode;

lazy_static! {
    /// Includes all guild-related events to signal to Discord that we intend to
    /// receive and process them
    pub static ref INTENTS: Intents = Intents::GUILDS
        | Intents::GUILD_MEMBERS
        | Intents::GUILD_BANS
        | Intents::GUILD_EMOJIS
        | Intents::GUILD_INTEGRATIONS
        | Intents::GUILD_WEBHOOKS
        | Intents::GUILD_INVITES
        | Intents::GUILD_VOICE_STATES
        | Intents::GUILD_MEMBERS
        | Intents::GUILD_MESSAGES
        | Intents::GUILD_MESSAGE_REACTIONS;
}

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
    let (shard, events) = connect::to_shard(Arc::clone(&config), logger.clone(), event_type_flags).await?;
    let shard_shared = Arc::new(shard);
    let rmq_connection = connect::to_queue(Arc::clone(&config), logger.clone()).await?;
    let feature_gate_client = connect::to_feature_gate(Arc::clone(&config), logger.clone()).await?;
    let uptime_service_client =
        connect::to_uptime_service(Arc::clone(&config), logger.clone()).await?;

    // Split up the stream of gateway events into lifecycle & raw events
    let (lifecycle_event_stream, raw_event_stream) = split_gateway_events(events, lifecycle_events, raw_events);

    // Notify the connection tracker of the newly connected services
    update_tx
        .send(UpdateMessage::GatewayOnline)
        .context("could not send gateway online update to uptime tracker")?;
    update_tx
        .send(UpdateMessage::QueueOnline)
        .context("could not send queue online update to uptime tracker")?;

    let (active_guilds, uptime_rx) = ActiveGuilds::new(feature_gate_client, Arc::clone(&config), logger.clone());

    // Listen to incoming gateway events and start re-publishing them on the queue
    // (performing the primary purpose of the service)
    let publish_sink = publish_events(
        rmq_connection,
        active_guilds.clone(),
        Arc::clone(&shard_shared),
        Arc::clone(&config),
        logger.clone(),
        update_tx.clone(),
    );

    // Listen for lifecycle events and send them to the uptime tracker
    let lifecycle_event_listener = process_lifecycle_events(
        lifecycle_event_stream,
        update_tx.clone(),
    );

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
        lifecycle_event_listener,
        unfiltered_uptime_pipe,
        uptime_events_sink,
        publish_sink,
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

/// Manages the lifecycle of the connection to the downstream Rabbit MQ queue
/// and the stream of raw events being forwarded from the Discord Gateway.
/// Attempts to re-connect to Rabbit MQ upon lost connection,
/// and will update the stateful uptime tracker accordingly
async fn publish_events(
    queue_connection: Connection,
    active_guilds: ActiveGuilds,
    raw_event_stream: impl Stream<Item = Event>,,
    config: Arc<Configuration>,
    logger: Logger,
    update_tx: UnboundedSender<UpdateMessage>,
) -> Result<()> {
    // Create a new Id provisioner and use it throughout
    let id_provisioner = Arc::new(IdProvisioner::new(Some(logger.clone())));

    // Keep looping over the lifecycle,
    // allowing for the state to be eagerly restored after a disconnection
    // using the same backoff as initialization.
    // If the backoff is exhausted or there is another error, then the entire future exits
    // (and the service will exit accordingly)
    let mut outer_rmq_connection = Some(queue_connection);
    loop {
        let id_provisioner = Arc::clone(&id_provisioner);
        let config = Arc::clone(&config);
        let active_guilds = active_guilds.clone();

        // Reconnect to the Rabbit MQ instance if needed
        let rmq_connection = if let Some(rmq) = outer_rmq_connection.take() {
            rmq
        } else {
            let connection = connect::to_queue(Arc::clone(&config), logger.clone()).await?;
            update_tx
                .send(UpdateMessage::QueueOnline)
                .context("could not send queue online update to uptime tracker")?;
            connection
        };

        // Declare the RMQ queue to publish incoming events to
        declare_event_queue(&rmq_connection, Arc::clone(&config), logger.clone()).await?;

        // Create a pool for the RMQ channels
        let manager = Manager::new(rmq_connection);
        let mut pool_config = config.gateway_queue.connection_pool.clone();
        pool_config.runtime = Runtime::Tokio1;
        let channel_pool = Pool::from_config(manager, pool_config);

        // Start listening to the stream
        // (we have to convert the Stream to a TryStream before using `try_for_each_concurrent`)
        let event_try_stream = raw_event_stream.map(Ok::<Event, anyhow::Error>);
        let process = event_try_stream.try_for_each_concurrent(None, move |event| {
            // Provision an ID and note the timestamp immediately
            let timestamp = time::millisecond_ts();
            let id = id_provisioner.with_ts(timestamp);
            let logger = logger.new(slog::o!("event_timestamp" => timestamp, "event_id" => id));

            let config = Arc::clone(&config);
            let active_guilds = active_guilds.clone();
            let channel_pool = channel_pool.clone();

            // Create the `GatewayEvent` from the raw event
            async move {
                if let Some(gateway_event) = convert_raw_event(event, timestamp, id, logger.clone())
                {
                    // Make sure the guild is active before forwarding
                    let should_publish = active_guilds.is_active(gateway_event.guild_id).await;
                    if should_publish {
                        try_publish(gateway_event, channel_pool, config, logger).await?;
                    }
                }

                Ok(())
            }
        });

        if let Err(err) = process.await {
            slog::error!(
                logger,
                "an error occurred while listening to gateway events; attempting to reconnect";
                "error" => ?err,
                "rabbit_url" => config.services.gateway_queue,
            );
            update_tx
                .send(UpdateMessage::QueueOffline)
                .context("could not send queue offline update to uptime tracker")?;
        }
    }
}

/// Declares the Rabbit MQ queue, which is done during initialization of the Rabbit MQ connection
async fn declare_event_queue(
    rmq_connection: &Connection,
    config: Arc<Configuration>,
    logger: Logger,
) -> Result<Channel> {
    // Create a temporary channel
    let rmq_channel = rmq_connection
        .create_channel()
        .await
        .context("could not create a new RabbitMQ channel")?;

    // Declare the queue
    let queue_name = &config.gateway_queue.queue_name;
    let queue_options = QueueDeclareOptions {
        durable: config.gateway_queue.durable,
        ..QueueDeclareOptions::default()
    };
    let arguments = config
        .gateway_queue
        .queue_parameters
        .as_ref()
        .cloned()
        .map(|map| {
            FieldTable::from(
                map.into_iter()
                    .map(|(key, value)| (lapin::types::ShortString::from(key), value))
                    .collect::<BTreeMap<_, _>>(),
            )
        })
        .unwrap_or_else(FieldTable::default);
    rmq_channel
        .queue_declare(queue_name, queue_options, arguments)
        .await
        .context("could not declare the RabbitMQ queue")?;

    slog::info!(
        logger,
        "declared RabbitMQ queue";
        "queue_name" => queue_name,
        "rabbit_url" => &config.services.gateway_queue,
    );
    Ok(rmq_channel)
}

/// Attempts to synchronously convert a raw gateway event into our struct
/// that will eventually be published to the gateway queue
fn convert_raw_event(
    event: Event,
    timestamp: u64,
    id: HoarFrost,
    logger: Logger,
) -> Option<GatewayEventOwned> {
    if let Event::ShardPayload(Payload { bytes }) = event {
        let json = match std::str::from_utf8(&bytes) {
            Ok(json) => json,
            Err(err) => {
                slog::warn!(
                    logger,
                    "an error occurred while deserializing gateway JSON";
                    "error" => ?err
                );
                return None;
            }
        };

        // Use twilight's fast pre-deserializer to determine the op type,
        // and only deserialize it if it:
        // - is a proper Gateway dispatch event
        // - has a matching processor
        let deserializer = GatewayEventDeserializer::from_json(json)?;
        let (op, _, event_type) = deserializer.into_parts();
        if op != OpCode::Event as u8 {
            return None;
        }

        // Make sure we should forward the event
        let event_type_str = event_type.as_deref()?;
        let event_type =
            serde_json::from_str::<EventType>(&format!(r#""{}""#, event_type_str)).ok();
        if !should_forward(event_type) {
            return None;
        }

        let value = serde_json::from_str::<serde_json::Value>(json).ok()?;
        if let serde_json::Value::Object(map) = value {
            // Attempt to find the ".d" value (contains the Gateway message payload)
            // https://discord.com/developers/docs/topics/gateway#payloads-gateway-payload-structure
            let mut map = map;
            let inner_json = map.remove("d")?;

            // Make sure the guild id can be extracted before forwarding
            let guild_id_option =
                try_extract_guild_id(&inner_json, event_type, event_type_str, logger.clone());
            let guild_id = if let Some(guild_id) = guild_id_option {
                guild_id
            } else {
                slog::warn!(
                    logger,
                    "no guild id was extracted for event";
                    "event_type" => event_type_str,
                    "inner_json" => inner_json,
                );
                return None;
            };

            return Some(GatewayEventOwned {
                id,
                ingress_timestamp: timestamp,
                inner: inner_json,
                event_type: event_type_str.to_owned(),
                guild_id,
            });
        }
    }

    None
}

/// Attempts to publish a single gateway event to the durable queue
async fn try_publish(
    gateway_event: GatewayEventOwned,
    channel_pool: architus_amqp_pool::Pool,
    config: Arc<Configuration>,
    logger: Logger,
) -> Result<()> {
    // Serialize the event into a binary buffer using MessagePack
    let buf = match rmp_serde::to_vec(&gateway_event) {
        Ok(buf) => buf,
        Err(err) => {
            slog::warn!(
                logger,
                "an error occurred while serializing event to MessagePack";
                "error" => ?err,
            );
            return Ok(());
        }
    };

    // Asynchronously obtain a channel from the pool
    let channel = channel_pool.get().await.map_err(|err| match err {
        PoolError::Backend(err) => err,
        PoolError::Closed => anyhow!("The pool has been closed"),
        PoolError::NoRuntimeSpecified => panic!("No async runtime was specified"),
        PoolError::Timeout(timeout) => {
            anyhow!("Timeout error from pool: {:?}", timeout)
        }
    })?;

    // Finally, publish the event to the durable queue
    channel
        .basic_publish(
            &config.gateway_queue.exchange,
            &config.gateway_queue.routing_key,
            BasicPublishOptions::default(),
            buf,
            // 2 = persistent/durable
            // `https://www.rabbitmq.com/publishers.html#message-properties`
            BasicProperties::default().with_delivery_mode(2),
        )
        .await
        .context("could not publish gateway event to queue")?;

    Ok(())
}

/// Determines whether the ingress shard should forward events to the queue
/// (certain events, such as raw gateway lifecycle events, should not be forwarded)
const fn should_forward(event_type: Option<EventType>) -> bool {
    // Don't forward lifecycle events (or typing/presence updates):
    // `https://discord.com/developers/docs/topics/gateway#commands-and-events-gateway-events`
    // Default to forwarding an event if it is not identified
    !matches!(
        event_type,
        Some(EventType::GatewayHeartbeat)
            | Some(EventType::GatewayHeartbeatAck)
            | Some(EventType::GatewayHello)
            | Some(EventType::GatewayInvalidateSession)
            | Some(EventType::GatewayReconnect)
            | Some(EventType::GiftCodeUpdate)
            | Some(EventType::MemberChunk)
            | Some(EventType::PresenceUpdate)
            | Some(EventType::PresencesReplace)
            | Some(EventType::Ready)
            | Some(EventType::Resumed)
            | Some(EventType::ShardConnected)
            | Some(EventType::ShardConnecting)
            | Some(EventType::ShardDisconnected)
            | Some(EventType::ShardIdentifying)
            | Some(EventType::ShardReconnecting)
            | Some(EventType::ShardPayload)
            | Some(EventType::ShardResuming)
            | Some(EventType::TypingStart)
            | Some(EventType::UnavailableGuild)
            | Some(EventType::VoiceServerUpdate)
            // Disable forwarding events for guilds coming off/online
            // Note: that means we'll have to have some other mechanism for logging bot joins/leaves
            | Some(EventType::GuildCreate)
            | Some(EventType::GuildDelete)
            // Disable forwarding events for those that are handled via the audit log
            | Some(EventType::BanAdd)
            | Some(EventType::BanRemove)
            | Some(EventType::ChannelCreate)
            | Some(EventType::ChannelDelete)
            | Some(EventType::ChannelPinsUpdate)
            | Some(EventType::ChannelUpdate)
            | Some(EventType::GuildEmojisUpdate)
            | Some(EventType::GuildIntegrationsUpdate)
            | Some(EventType::GuildUpdate)
            | Some(EventType::InviteCreate)
            | Some(EventType::InviteDelete)
            | Some(EventType::MemberUpdate)
            | Some(EventType::RoleCreate)
            | Some(EventType::RoleDelete)
            | Some(EventType::RoleUpdate)
            | Some(EventType::UserUpdate)
            | Some(EventType::VoiceStateUpdate)
            | Some(EventType::WebhooksUpdate)
    )
}

/// Attempts to extract a guild id from a partially-serialized gateway event
fn try_extract_guild_id(
    json_value: &serde_json::Value,
    _event_type: Option<EventType>,
    raw_event_type: &str,
    logger: Logger,
) -> Option<u64> {
    if let serde_json::Value::Object(map) = json_value {
        if let Some(guild_id_value) = map.get("guild_id") {
            if let serde_json::Value::String(guild_id_string) = guild_id_value {
                // Attempt to parse the guild id string to a u64
                return guild_id_string.parse::<u64>().ok();
            }
        }
    }

    slog::warn!(
        logger,
        "couldn't identify guild_id value for event";
        "event_type" => raw_event_type,
    );
    None
}
