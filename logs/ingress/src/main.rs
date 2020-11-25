mod config;
mod event;
mod gateway;

use crate::gateway::OriginalEvent;
use anyhow::{Context, Result};
use clap::{App, Arg};
use config::Configuration;
use futures::StreamExt;
use lazy_static::lazy_static;
use log::{debug, error, info};
use twilight_gateway::{Event, EventTypeFlags, Intents, Shard};
use twilight_model::gateway::event::gateway::GatewayEventDeserializer;
use twilight_model::gateway::event::shard::Payload;
use logs_lib::time;
use std::sync::Arc;

/// Bootstraps the bot and begins listening for gateway events
#[tokio::main]
async fn main() {
    env_logger::init();
    match run().await {
        Ok(_) => info!("Exiting"),
        Err(err) => error!("{:?}", err),
    }
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

/// Attempts to initialize the bot and listen for gateway events
async fn run() -> Result<()> {
    // Use clap to pass in a config path
    let app = App::new("logs-ingress").arg(
        Arg::with_name("config")
            .short("c")
            .long("config")
            .value_name("FILE")
            .help("TOML config file path")
            .takes_value(true)
            .required(true),
    );
    let matches = app.get_matches();
    let config_path = matches.value_of("config").unwrap();

    // Parse the config from the path and use it to initialize the event stream
    let config = Configuration::try_load(config_path)?;
    let event_types = EventTypeFlags::SHARD_PAYLOAD;
    let mut shard = Shard::new(config.secrets.discord_token, *INTENTS);
    let events = shard.some_events(event_types);

    // Initialize the gateway event processor
    let processor = Arc::new(gateway::Processor::new());

    shard.start().await.context("Could not start shard")?;
    info!("Created shard and preparing to listen for gateway events");

    // Listen for all raw gateway events and process them,
    // re-emitting half-processed gateway events to be consumed by the processor
    let processor_copy = Arc::clone(&processor);
    let gateway_event_stream = events.filter_map(|event| async move {
        if let Event::ShardPayload(Payload { bytes }) = event {
            if let Ok(json) = std::str::from_utf8(&bytes) {
                // Use twilight's fast pre-deserializer to determine the op type,
                // and only deserialize it if it:
                // - is a proper Gateway dispatch event
                // - has a matching processor
                if let Some(deserializer) = GatewayEventDeserializer::from_json(json) {
                    let (op, seq, event_type) = deserializer.into_parts();
                    if op != 0 {
                        return None;
                    }

                    if let Some(event_type) = event_type.as_deref() {
                        // Make sure we can process the event
                        if !processor_copy.can_process(event_type) {
                            return None;
                        }

                        // Convert the event into an owned version
                        // and emit as the stream item
                        let event_type = event_type.to_owned();
                        let result = serde_json::from_str::<serde_json::Value>(json);
                        if let Ok(value) = result {
                            return Some(OriginalEvent {
                                seq,
                                event_type,
                                json: value,
                                rx_timestamp: time::millisecond_ts(),
                            });
                        }
                    }
                }
            }
        }

        None
    });

    // Normalize each event coming from the gateway,
    // and process them in parallel where possible via buffer_unordered
    let normalized_event_stream = gateway_event_stream
        .map(|event| async move {
            match processor.normalize(event).await {
                Ok(normalized_event) => Some(normalized_event),
                Err(err) => {
                    debug!("Event normalization failed for event: {:?}", err);
                    None
                }
            }
        })
        .buffer_unordered(config.normalized_stream_concurrency);

    // Send each normalized event to the logging import service,
    // acting as a sink for this stream
    normalized_event_stream
        .for_each_concurrent(Some(config.import_stream_concurrency), |event| async move {
            // TODO implement sending via gRPC to import service
            info!("Normalized event received at sink: {:?}", event);
        })
        .await;

    Ok(())
}
