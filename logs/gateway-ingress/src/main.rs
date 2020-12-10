#![warn(clippy::all, clippy::pedantic, clippy::nursery)]

mod config;
mod connection;
mod debounced_pool;

use crate::config::Configuration;
use crate::connection::{Tracker, UpdateMessage};
use anyhow::{Context, Result};
use architus_id::time;
use backoff::{future::FutureOperation as _, Error as BackoffError, ExponentialBackoff};
use chrono::{DateTime, NaiveDateTime, Utc};
use futures::{try_join, Stream, StreamExt, TryStreamExt};
use gateway_queue_lib::GatewayEvent;
use lapin::options::QueueDeclareOptions;
use lapin::{
    publisher_confirm::Confirmation, types::FieldTable, BasicProperties, Connection,
    ConnectionProperties,
};
use lazy_static::lazy_static;
use log::{debug, error, info, warn};
use std::convert::{Into, TryFrom};
use std::future::Future;
use std::ops::Deref;
use std::sync::Arc;
use tokio::sync::mpsc::UnboundedSender;
use twilight_gateway::{Event, EventTypeFlags, Intents, Shard};
use twilight_model::gateway::event::gateway::GatewayEventDeserializer;
use twilight_model::gateway::event::shard::Payload;
use twilight_model::gateway::OpCode;

/// Attempts to initialize the bot and listen for gateway events
#[tokio::main]
async fn main() -> Result<()> {
    env_logger::init();
    let config_path = std::env::args().nth(1).expect(
        "no config path given \
        \nUsage: \
        \nlogs-gateway-ingress [config-path]",
    );

    // Parse the config from the path and use it to initialize the event stream
    let config = Arc::new(Configuration::try_load(config_path)?);
    let initialization_backoff = config.initialization_backoff.build();

    // Connect to the Discord Gateway and initialize the shard
    let shard_connect = || async {
        let mut shard = Shard::new(config.secrets.discord_token.clone(), *INTENTS);
        shard.start().await.map_err(|err| {
            warn!(
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
    let shard = Arc::new(shard);
    info!("Created shard and preparing to listen for gateway events");

    // Listen for lifecycle events and send them to the uptime tracker
    let (tracker, update_tx) = Tracker::new(Arc::clone(&config));
    let lifecycle_events = EventTypeFlags::GATEWAY_HEARTBEAT_ACK
        | EventTypeFlags::GUILD_CREATE
        | EventTypeFlags::GUILD_DELETE
        | EventTypeFlags::SHARD_CONNECTED
        | EventTypeFlags::SHARD_DISCONNECTED;
    let lifecycle_event_sink =
        process_lifecycle_events(shard.some_events(lifecycle_events), update_tx.clone());

    // Connect to RabbitMQ and start re-publishing events on the queue
    let publish_sink = publish_events(Arc::clone(&shard), Arc::clone(&config));

    try_join!(tracker.run(), lifecycle_event_sink, publish_sink)?;

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
    let try_stream = StreamExt::map(in_stream, |event| Ok::<Event, SendError>(event));
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

/// Manages the lifecycle of the connection to the downstream RabbitMQ queue
/// and the stream of raw events being forwarded from the Discord Gateway.
/// Attempts to re-connect to RabbitMQ upon lost connection,
/// and will update the stateful uptime tracker accordingly
async fn publish_events(shard: Arc<Shard>, config: Arc<Configuration>) -> Result<()> {
    // TODO implement connection loop with RabbitMQ
    // that manages lifecycle and attempts to re-connect upon errors
    // Also will re-create stream from shard when re-starting loop
    Ok(())
}

// /// Stream processor function that takes in a raw stream of gateway events
// /// and uses twilight's fast pre-deserializer to validate that
// /// they are valid and usable events before parsing and re-emitting them
// fn pipe_gateway_events(
//     in_stream: impl Stream<Item = Event>,
//     processor: Arc<gateway::Processor>,
// ) -> impl Stream<Item = GatewayEvent> {
//     // Get the opcode byte number for `OpCode::Event` packets
//     let event_opcode: u8 =
//         match serde_json::to_value(OpCode::Event).expect("Couldn't turn OpCode::Event into json") {
//             serde_json::Value::Number(n) => n
//                 .as_u64()
//                 .and_then(|i| TryFrom::try_from(i).ok())
//                 .expect("Couldn't turn OpCode::Event into u8"),
//             _ => panic!("serialization from OpCode produced non-u8"),
//         };
//
//     in_stream.filter_map(move |event| {
//         let processor_copy = Arc::clone(&processor);
//         async move {
//             if let Event::ShardPayload(Payload { bytes }) = event {
//                 let json = std::str::from_utf8(&bytes).ok()?;
//                 // Use twilight's fast pre-deserializer to determine the op type,
//                 // and only deserialize it if it:
//                 // - is a proper Gateway dispatch event
//                 // - has a matching processor
//                 let deserializer = GatewayEventDeserializer::from_json(json)?;
//                 let (op, seq, event_type) = deserializer.into_parts();
//                 if op != event_opcode {
//                     return None;
//                 }
//
//                 // Make sure we can process the event
//                 let event_type = event_type.as_deref()?;
//                 if !processor_copy.can_process(event_type) {
//                     return None;
//                 }
//
//                 let value = serde_json::from_str::<serde_json::Value>(json).ok()?;
//                 if let serde_json::Value::Object(map) = value {
//                     // Attempt to find the ".d" value (contains the Gateway message payload)
//                     // https://discord.com/developers/docs/topics/gateway#payloads-gateway-payload-structure
//                     let mut map = map;
//                     let inner_json = map.remove("d")?;
//                     return Some(OriginalEvent {
//                         seq,
//                         event_type: event_type.to_owned(),
//                         json: inner_json,
//                         rx_timestamp: time::millisecond_ts(),
//                     });
//                 }
//             }
//
//             None
//         }
//     })
// }
//
// /// Acts as a stream sink, taking each item from the gateway event stream
// /// and publishing them to the shared durable queue (backed by RabbitMQ).
// async fn publish_gateway_events(
//     in_stream: impl Stream<Event = GatewayEvent>,
//     config: Arc<Configuration>,
// ) -> Result<()> {
//     // Connect to the RabbitMQ instance
//     let rmq_url = config.services.gateway_queue;
//     let rmq_connect = || async {
//         let conn = Connection::connect(&rmq_url, ConnectionProperties::default())
//             .await
//             .map_err(|err| {
//                 warn!(
//                     "Couldn't connect to RabbitMQ, retrying after backoff: {:?}",
//                     err
//                 );
//                 err
//             })?;
//         Ok(conn)
//     };
//     let rmq_connection = rmq_connect
//         .retry(&mut initialization_backoff)
//         .await
//         .context("Could not connect to the RabbitMQ gateway queue")?;
//     info!("Connected to RabbitMQ at {}", rmq_url);
//
//     // Declare the RMQ channel to publish incoming events to
//     let rmq_channel = rmq_connection
//         .create_channel()
//         .await
//         .context("Could not create a new RabbitMQ channel")?;
//     let queue_options = QueueDeclareOptions {
//         durable: true,
//         ..Default::default()
//     };
//     let queue = rmq_channel
//         .queue_declare(
//             &config.rabbitmq_queue_name,
//             queue_options,
//             FieldTable::default(),
//         )
//         .await
//         .context("Could not declare the RabbitMQ queue")?;
//     info!("Declared RabbitMQ queue {}", config.rabbitmq_queue_name);
//
//     in_stream
//         .try_for_each_concurrent(move |event| async {
//             info!("publishing event: {:?}", event);
//             Ok(())
//         })
//         .await
// }
//
// /// Turns the given millisecond timestamp into a readable string
// fn readable_timestamp(timestamp: u64) -> Result<String> {
//     let sec =
//         i64::try_from(timestamp / 1_000).context("Could not convert timestamp seconds to i64")?;
//     let nsec = u32::try_from((timestamp % 1_000).saturating_mul(1_000_000))
//         .context("Could not convert timestamp nanoseconds to u32")?;
//     let naive_datetime = NaiveDateTime::from_timestamp_opt(sec, nsec)
//         .context("Could not convert timestamp to Naive DateTime")?;
//     let datetime: DateTime<Utc> = DateTime::from_utc(naive_datetime, Utc);
//     Ok(datetime.format("%Y-%m-%d %H:%M:%S").to_string())
// }
