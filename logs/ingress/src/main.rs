// mod audit_log;
mod config;
// mod data;
mod event;
// mod gateway;

use anyhow::{Context, Result};
use clap::{App, Arg};
use config::Configuration;
use lazy_static::lazy_static;
use log::{error, info};
use futures::StreamExt;
use tokio_compat_02::FutureExt;
use twilight_gateway::{Event, EventTypeFlags, Intents, Shard};
use twilight_model::gateway::event::shard::Payload;

/// Bootstraps the bot and begins listening for gateway events
#[tokio::main]
async fn main() {
    env_logger::init();
    match run().compat().await {
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
    let mut events = shard.some_events(event_types);

    shard.start().await.context("Could not start shard")?;
    info!("Created shard and preparing to listen for gateway events");

    // Listen for all raw gateway events and process them
    while let Some(event) = events.next().await {
        match event {
            Event::ShardPayload(Payload{ bytes }) => {
                if let Ok(as_str) = std::str::from_utf8(&bytes) {
                    // TODO consume
                    println!("Event: {}", as_str);
                }
            },
            _ => {},
        }
    }

    Ok(())
}
