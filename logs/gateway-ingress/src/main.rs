#![warn(clippy::all, clippy::pedantic, clippy::nursery)]
#![allow(clippy::future_not_send)]

mod active_guilds;
mod amqp_pool;
mod config;
mod connection;
mod debounced_pool;

mod feature_gate {
    #![allow(clippy::all, clippy::pedantic, clippy::nursery)]
    tonic::include_proto!("featuregate");
}

use crate::active_guilds::{ActiveGuilds, FeatureGateClient};
use crate::config::Configuration;
use crate::connection::{Tracker, UpdateMessage};
use anyhow::{anyhow, Context, Result};
use architus_id::{time, HoarFrost, IdProvisioner};
use backoff::future::FutureOperation as _;
use deadpool::managed::PoolError;
use futures::{try_join, Stream, StreamExt, TryStreamExt};
use gateway_queue_lib::GatewayEvent;
use lapin::options::{BasicPublishOptions, QueueDeclareOptions};
use lapin::{types::FieldTable, BasicProperties, Connection, ConnectionProperties};
use lazy_static::lazy_static;
use std::sync::Arc;
use tokio::sync::mpsc::UnboundedSender;
use twilight_gateway::{Event, EventType, EventTypeFlags, Intents, Shard};
use twilight_model::gateway::event::gateway::GatewayEventDeserializer;
use twilight_model::gateway::event::shard::Payload;
use twilight_model::gateway::OpCode;

/// Attempts to initialize the bot and listen for gateway events
#[tokio::main]
async fn main() -> Result<()> {
    env_logger::init();

    // Parse the config
    let config_path = std::env::args().nth(1).expect(
        "no config path given \
        \nUsage: \
        \nlogs-gateway-ingress [config-path]",
    );
    let config = Arc::new(Configuration::try_load(config_path)?);

    // Create the tracker and its update message channel
    let (tracker, update_tx) = Tracker::new(Arc::clone(&config));

    // Connect to the Discord Gateway and initialize the shard
    let shard = connect_to_shard(Arc::clone(&config))
        .await
        .context("Could not connect to the Discord gateway")?;
    let shard_shared = Arc::new(shard);
    update_tx
        .send(UpdateMessage::GatewayOnline)
        .context("Could not send gateway online update to uptime tracker")?;

    // Connect to the Rabbit MQ queue once to ensure that it has a healthy connection
    let rmq_connection = connect_to_queue(Arc::clone(&config))
        .await
        .context("Could not connect RabbitMQ")?;
    update_tx
        .send(UpdateMessage::QueueOnline)
        .context("Could not send queue online update to uptime tracker")?;

    // Connect to the feature gate service to ensure that it has a healthy connection
    let feature_gate_client = connect_to_feature_gate(Arc::clone(&config))
        .await
        .context("Could not connect to the active guild service")?;
    let active_guilds = ActiveGuilds::new(feature_gate_client, Arc::clone(&config));

    // Connect to the uptime service to ensure that it has a healthy connection
    let uptime_service_client = connect_to_uptime_service(Arc::clone(&config))
        .await
        .context("Could not connect to the uptime service")?;

    // Connect to RabbitMQ and start re-publishing events on the queue
    // (performing the primary purpose of the service)
    let publish_sink = publish_events(
        rmq_connection,
        active_guilds.clone(),
        Arc::clone(&shard_shared),
        Arc::clone(&config),
        update_tx.clone(),
    );

    // Listen for lifecycle events and send them to the uptime tracker
    let lifecycle_events = EventTypeFlags::GATEWAY_HEARTBEAT_ACK
        | EventTypeFlags::GUILD_CREATE
        | EventTypeFlags::GUILD_DELETE
        | EventTypeFlags::SHARD_CONNECTED
        | EventTypeFlags::SHARD_DISCONNECTED;
    let lifecycle_event_listener = process_lifecycle_events(
        shard_shared.some_events(lifecycle_events),
        update_tx.clone(),
    );

    // Pipe the uptime events from the tracker into the active guild handler
    let uptime_events_stream = tracker.stream_events();
    let active_filtered_events_stream = active_guilds.pipe_uptime_events(uptime_events_stream);

    // Sink all uptime events to the uptime tracking service
    let uptime_events_sink =
        sink_uptime_events(uptime_service_client, active_filtered_events_stream);

    // Continuously poll the set of active guilds
    let active_guilds_poll = active_guilds.go_poll();

    // Run all futures until an error is encountered
    try_join!(
        lifecycle_event_listener,
        uptime_events_sink,
        publish_sink,
        active_guilds_poll,
    )?;
    Ok(())
}

/// Represents a bulk uptime event that is eventually dispatched to the uptime service
/// in addition to the timestamp that the event happened at
#[derive(Clone, Debug, PartialEq)]
pub enum UptimeEvent {
    Online { guilds: Vec<u64>, timestamp: u64 },
    Offline { guilds: Vec<u64>, timestamp: u64 },
    Heartbeat { guilds: Vec<u64>, timestamp: u64 },
}

/// Attempts to initialize a gateway connection
async fn connect_to_shard(config: Arc<Configuration>) -> Result<Shard> {
    let initialization_backoff = config.initialization_backoff.build();
    let shard_connect = || async {
        let mut shard = Shard::new(config.secrets.discord_token.clone(), *INTENTS);
        shard.start().await.map_err(|err| {
            log::warn!(
                "Couldn't start bot shard, retrying after backoff: {:?}",
                err
            );
            err
        })?;
        Ok(shard)
    };
    let shard = shard_connect
        .retry(initialization_backoff)
        .await
        .context("Could not start shard")?;
    log::info!("Created shard and preparing to listen for gateway events");
    Ok(shard)
}

/// Creates a new connection to Rabbit MQ
async fn connect_to_queue(config: Arc<Configuration>) -> Result<Connection> {
    let initialization_backoff = config.initialization_backoff.build();
    let rmq_url = config.services.gateway_queue.clone();
    let rmq_connect = || async {
        let conn = Connection::connect(&rmq_url, ConnectionProperties::default())
            .await
            .map_err(|err| {
                log::warn!(
                    "Couldn't connect to RabbitMQ, retrying after backoff: {:?}",
                    err
                );
                err
            })?;
        Ok(conn)
    };
    let rmq_connection = rmq_connect
        .retry(initialization_backoff)
        .await
        .context("Could not connect to the RabbitMQ gateway queue")?;
    log::info!("Connected to RabbitMQ at {}", rmq_url);
    Ok(rmq_connection)
}

/// Creates a new connection to the feature gate service
async fn connect_to_feature_gate(config: Arc<Configuration>) -> Result<FeatureGateClient> {
    let initialization_backoff = config.initialization_backoff.build();
    let feature_gate_url = config.services.feature_gate.clone();
    let connect = || async {
        let conn = FeatureGateClient::connect(feature_gate_url.clone())
            .await
            .map_err(|err| {
                log::warn!(
                    "Couldn't connect to feature-gate, retrying after backoff: {:?}",
                    err
                );
                err
            })?;
        Ok(conn)
    };
    let connection = connect
        .retry(initialization_backoff)
        .await
        .context("Could not connect to feature-gate")?;
    log::info!("Connected to feature-gate at {}", feature_gate_url);
    Ok(connection)
}

/// Creates a new connection to the uptime service
/// TODO implement and change return type
async fn connect_to_uptime_service(_config: Arc<Configuration>) -> Result<()> {
    Ok(())
}

/// Listens for lifecycle events from the Gateway and sends corresponding update messages
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
        .context("Could not send update messages to uptime tracker")?;

    Ok(())
}

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

/// Manages the lifecycle of the connection to the downstream Rabbit MQ queue
/// and the stream of raw events being forwarded from the Discord Gateway.
/// Attempts to re-connect to Rabbit MQ upon lost connection,
/// and will update the stateful uptime tracker accordingly
async fn publish_events(
    rmq_connection: Connection,
    active_guilds: ActiveGuilds,
    shard: Arc<Shard>,
    config: Arc<Configuration>,
    update_tx: UnboundedSender<UpdateMessage>,
) -> Result<()> {
    // Create a new Id provisioner and use it throughout
    let id_provisioner = Arc::new(IdProvisioner::new());

    // Keep looping over the lifecycle,
    // allowing for the state to be eagerly restored after a disconnection
    // using the same backoff as initialization.
    // If the backoff is exhausted or there is another error, then the entire future exits
    // (and the service will exit accordingly)
    let mut rmq_connection = Some(rmq_connection);
    loop {
        let id_provisioner = Arc::clone(&id_provisioner);
        let config = Arc::clone(&config);
        let active_guilds = active_guilds.clone();

        // Reconnect to the Rabbit MQ instance if needed
        let rmq = if let Some(rmq) = rmq_connection.take() {
            rmq
        } else {
            let rmq = connect_to_queue(Arc::clone(&config))
                .await
                .context("Could not reconnect to RabbitMQ")?;
            update_tx
                .send(UpdateMessage::QueueOnline)
                .context("Could not send queue online update to uptime tracker")?;
            rmq
        };

        // Declare the RMQ channel to publish incoming events to
        let rmq_channel = rmq
            .create_channel()
            .await
            .context("Could not create a new RabbitMQ channel")?;
        let queue_options = QueueDeclareOptions {
            durable: true,
            ..QueueDeclareOptions::default()
        };
        rmq_channel
            .queue_declare(
                &config.gateway_queue.queue_name,
                queue_options,
                FieldTable::default(),
            )
            .await
            .context("Could not declare the RabbitMQ queue")?;
        drop(rmq_channel);
        log::info!(
            "Declared RabbitMQ queue {}",
            config.gateway_queue.queue_name
        );

        // Create a pool for the RMQ channels
        let manager = amqp_pool::Manager::new(rmq);
        let channels =
            amqp_pool::Pool::from_config(manager, config.gateway_queue.connection_pool.clone());

        // Start listening to the stream
        // (we have to convert the Stream to a TryStream before using `try_for_each_concurrent`)
        let event_stream = shard.some_events(EventTypeFlags::SHARD_PAYLOAD);
        let event_try_stream = event_stream.map(Ok::<Event, anyhow::Error>);
        let process = event_try_stream.try_for_each_concurrent(None, move |event| {
            let id_provisioner = Arc::clone(&id_provisioner);
            let config = Arc::clone(&config);
            let active_guilds = active_guilds.clone();
            let channels = channels.clone();
            async move {
                let timestamp = time::millisecond_ts();
                let id = id_provisioner.with_ts(timestamp);

                // Create the `GatewayEvent` from the raw event
                if let Some(gateway_event) = process_raw_event(event, timestamp, id) {
                    // Make sure the guild is active before forwarding
                    let should_process = match gateway_event.guild_id {
                        Some(id) => active_guilds.is_active(id).await,
                        None => true,
                    };
                    if !should_process {
                        return Ok(());
                    }

                    // Serialize the event into a binary buffer using MessagePack
                    let buf = match rmp_serde::to_vec(&gateway_event) {
                        Ok(buf) => buf,
                        Err(err) => {
                            log::warn!(
                                "An error occurred while serializing event to MessagePack: {:?}",
                                err
                            );
                            return Ok(());
                        }
                    };

                    // Asynchronously obtain a channel from the pool
                    let channel = channels.get().await.map_err(|err| match err {
                        PoolError::Backend(err) => err,
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
                        .context("Could not publish gateway event to queue")?;
                }

                Ok(())
            }
        });

        if let Err(err) = process.await {
            log::error!("An error occurred while listening to gateway events; attempting to reconnect: {:?}", err);
            update_tx
                .send(UpdateMessage::QueueOffline)
                .context("Could not send queue offline update to uptime tracker")?;
        }
    }
}

/// Attempts to synchronously convert a raw gateway event into our struct
/// that will eventually be published to the gateway queue
fn process_raw_event(event: Event, timestamp: u64, id: HoarFrost) -> Option<GatewayEvent> {
    if let Event::ShardPayload(Payload { bytes }) = event {
        let json = match std::str::from_utf8(&bytes) {
            Ok(json) => json,
            Err(err) => {
                log::warn!(
                    "An error occurred while deserializing gateway JSON: {:?}",
                    err
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
            let guild_id = try_extract_guild_id(&inner_json, event_type, event_type_str);
            return Some(GatewayEvent {
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
    )
}

/// Attempts to extract a guild id from a partially-serialized gateway event
fn try_extract_guild_id(
    json_value: &serde_json::Value,
    _event_type: Option<EventType>,
    raw_event_type: &str,
) -> Option<u64> {
    if let serde_json::Value::Object(map) = json_value {
        if let Some(guild_id_value) = map.get("guild_id") {
            if let serde_json::Value::String(guild_id_string) = guild_id_value {
                // Attempt to parse the guild id string to a u64
                return guild_id_string.parse::<u64>().ok();
            }
        }
    }

    log::warn!(
        "Couldn't identify guild_id value for event type '{}'",
        raw_event_type
    );
    None
}

/// Acts as a stream sink for uptime events,
/// sending them to the uptime service
async fn sink_uptime_events(
    _uptime_service_client: (),
    in_stream: impl Stream<Item = UptimeEvent>,
) -> Result<()> {
    // Note: we don't exit the service if this part fails;
    // this is an acceptable degradation
    in_stream
        .for_each_concurrent(None, move |event| async move {
            // TODO implement sending to service
            log::info!("Sending UptimeEvent: {:?}", event);
        })
        .await;

    Ok(())
}
