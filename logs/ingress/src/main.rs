use serenity::model::event::EventType;
use lazy_static::lazy_static;
use log::{trace, warn, debug, error, info};
use serenity::async_trait;
use serenity::client::bridge::gateway::GatewayIntents;
use serenity::framework::standard::StandardFramework;
use serenity::model::event::Event;
use serenity::model::gateway::Ready;
use serenity::prelude::*;

mod env;
use env::Environment;

lazy_static! {
    /// Includes all guild-related events to signal to Discord that we intend to
    /// receive and process them
    static ref INTENTS: GatewayIntents = GatewayIntents::GUILDS
        | GatewayIntents::GUILD_MEMBERS
        | GatewayIntents::GUILD_BANS
        | GatewayIntents::GUILD_EMOJIS
        | GatewayIntents::GUILD_INTEGRATIONS
        | GatewayIntents::GUILD_WEBHOOKS
        | GatewayIntents::GUILD_INVITES
        | GatewayIntents::GUILD_VOICE_STATES
        | GatewayIntents::GUILD_MEMBERS
        | GatewayIntents::GUILD_MESSAGES
        | GatewayIntents::GUILD_MESSAGE_REACTIONS;
}

/// Bootstraps the bot and begins listening for gateway events
#[tokio::main]
async fn main() {
    env_logger::init();
    let env = Environment::load();

    let framework = StandardFramework::new();
    let mut client = Client::new(&env.token)
        .event_handler(Handler{})
        .raw_event_handler(Handler{})
        .framework(framework)
        .intents(*INTENTS)
        .await
        .expect("error creating client");

    if let Err(why) = client.start().await {
        error!("Client error: {:?}", why);
    }
}

/// Bot event handler (used to dispatch events to the main service)
struct Handler {}

#[async_trait]
impl EventHandler for Handler {
    /// Called once bot connects and is ready
    async fn ready(&self, _ctx: Context, ready: Ready) {
        info!("{} is connected!", ready.user.tag());
    }
}

impl Handler {
    /// Handles propagation of Gateway events to the main service
    async fn handle(_event: Event, event_type: EventType) {
        debug!("Received Event::{:?}", event_type);
        // TODO implement
    }
}

#[async_trait]
impl RawEventHandler for Handler {
    /// Triggers on every event coming in from the gateway
    async fn raw_event(&self, _ctx: Context, raw_event: Event) {
        trace!("Event: {:?}", raw_event);
        match raw_event {
            // Handle all log ingestion events
            Event::ChannelCreate(_) => Self::handle(raw_event, EventType::ChannelCreate).await,
            Event::ChannelDelete(_) => Self::handle(raw_event, EventType::ChannelDelete).await,
            Event::ChannelPinsUpdate(_) => Self::handle(raw_event, EventType::ChannelPinsUpdate).await,
            Event::ChannelUpdate(_) => Self::handle(raw_event, EventType::ChannelUpdate).await,
            Event::GuildBanAdd(_) => Self::handle(raw_event, EventType::GuildBanAdd).await,
            Event::GuildBanRemove(_) => Self::handle(raw_event, EventType::GuildBanRemove).await,
            Event::GuildCreate(_) => Self::handle(raw_event, EventType::GuildCreate).await,
            Event::GuildDelete(_) => Self::handle(raw_event, EventType::ChannelCreate).await,
            Event::GuildEmojisUpdate(_) => Self::handle(raw_event, EventType::GuildEmojisUpdate).await,
            Event::GuildIntegrationsUpdate(_) => Self::handle(raw_event, EventType::GuildIntegrationsUpdate).await,
            Event::GuildMemberAdd(_) => Self::handle(raw_event, EventType::GuildMemberAdd).await,
            Event::GuildMemberRemove(_) => Self::handle(raw_event, EventType::GuildMemberRemove).await,
            Event::GuildMemberUpdate(_) => Self::handle(raw_event, EventType::GuildMemberUpdate).await,
            Event::GuildMembersChunk(_) => Self::handle(raw_event, EventType::GuildMembersChunk).await,
            Event::GuildRoleCreate(_) => Self::handle(raw_event, EventType::GuildRoleCreate).await,
            Event::GuildRoleDelete(_) => Self::handle(raw_event, EventType::GuildRoleDelete).await,
            Event::GuildRoleUpdate(_) => Self::handle(raw_event, EventType::GuildRoleUpdate).await,
            Event::GuildUnavailable(_) => Self::handle(raw_event, EventType::GuildUnavailable).await,
            Event::GuildUpdate(_) => Self::handle(raw_event, EventType::GuildUpdate).await,
            Event::MessageCreate(_) => Self::handle(raw_event, EventType::MessageCreate).await,
            Event::MessageDelete(_) => Self::handle(raw_event, EventType::MessageDelete).await,
            Event::MessageDeleteBulk(_) => Self::handle(raw_event, EventType::MessageDeleteBulk).await,
            Event::MessageUpdate(_) => Self::handle(raw_event, EventType::MessageUpdate).await,
            Event::ReactionAdd(_) => Self::handle(raw_event, EventType::ReactionAdd).await,
            Event::ReactionRemove(_) => Self::handle(raw_event, EventType::ReactionRemove).await,
            Event::ReactionRemoveAll(_) => Self::handle(raw_event, EventType::ReactionRemoveAll).await,
            Event::VoiceStateUpdate(_) => Self::handle(raw_event, EventType::VoiceStateUpdate).await,
            Event::VoiceServerUpdate(_) => Self::handle(raw_event, EventType::VoiceServerUpdate).await,
            Event::WebhookUpdate(_) => Self::handle(raw_event, EventType::WebhookUpdate).await,
            Event::Unknown(event) => warn!("Received Event::Unknown: {:?}", event),
            _ => {},
        }
    }
}
