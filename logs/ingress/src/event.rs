use lazy_static::lazy_static;
use log::{debug, trace, warn};
use logs_lib::id::{self, IdProvisioner};
use logs_lib::{time, ActionType};
use serde::ser::Serialize;
use serde_json;
use serenity::async_trait;
use serenity::client::bridge::gateway::GatewayIntents;
use serenity::model::event::Event;
use serenity::prelude::*;

lazy_static! {
    /// Includes all guild-related events to signal to Discord that we intend to
    /// receive and process them
    pub static ref INTENTS: GatewayIntents = GatewayIntents::GUILDS
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

/// Normalized log event to send to log ingestion
pub struct NormalizedEvent {
    pub id: u64,
    pub timestamp: u64,
    pub action_type: ActionType,
    pub guild_id: Option<u64>,
    pub agent_id: Option<u64>,
    pub subject_id: Option<u64>,
    pub data: Option<serde_json::Value>,
}

/// Bot event handler (used to dispatch events to the main service)
pub struct Handler {
    id_provisioner: IdProvisioner,
}

impl Default for Handler {
    fn default() -> Self {
        Self {
            id_provisioner: IdProvisioner::new(),
        }
    }
}

impl Handler {
    pub fn new() -> Self {
        Default::default()
    }

    async fn normalize(&self, raw_event: Event) -> Option<NormalizedEvent> {
        match raw_event {
            // Handle all log ingestion events
            Event::ChannelCreate(event) => Some(NormalizedEvent {
                action_type: ActionType::ChannelCreate,
                id: self.id_provisioner.provision(),
                data: try_serialize(&event),
                timestamp: id::extract_timestamp(event.channel.id().0),
                subject_id: Some(event.channel.id().0),
                guild_id: event.channel.guild().map(|guild| guild.id.0),
                // TODO is there a way to get the agent id here?
                // TODO Perhaps from an async API call?
                agent_id: None,
            }),
            Event::Unknown(event) => {
                warn!("Received Event::Unknown: {:?}", event);
                Some(NormalizedEvent {
                    action_type: ActionType::Unknown,
                    id: self.id_provisioner.provision(),
                    data: try_serialize(&event),
                    timestamp: time::millisecond_ts(),
                    subject_id: None,
                    guild_id: None,
                    agent_id: None,
                })
            },
            _ => None
            // Event::ChannelDelete(_) => Self::handle(raw_event, EventType::ChannelDelete).await,
            // Event::ChannelPinsUpdate(_) => Self::handle(raw_event, EventType::ChannelPinsUpdate).await
            // Event::ChannelUpdate(_) => Self::handle(raw_event, EventType::ChannelUpdate).await,
            // Event::GuildBanAdd(_) => Self::handle(raw_event, EventType::GuildBanAdd).await,
            // Event::GuildBanRemove(_) => Self::handle(raw_event, EventType::GuildBanRemove).await,
            // Event::GuildEmojisUpdate(_) => Self::handle(raw_event, EventType::GuildEmojisUpdate).await
            // Event::GuildIntegrationsUpdate(_) => Self::handle(raw_event, EventType::GuildIntegrationsUpdate).await
            // Event::GuildMemberAdd(_) => Self::handle(raw_event, EventType::GuildMemberAdd).await,
            // Event::GuildMemberRemove(_) => Self::handle(raw_event, EventType::GuildMemberRemove).await
            // Event::GuildMemberUpdate(_) => Self::handle(raw_event, EventType::GuildMemberUpdate).await
            // Event::GuildMembersChunk(_) => Self::handle(raw_event, EventType::GuildMembersChunk).await
            // Event::GuildRoleCreate(_) => Self::handle(raw_event, EventType::GuildRoleCreate).await,
            // Event::GuildRoleDelete(_) => Self::handle(raw_event, EventType::GuildRoleDelete).await,
            // Event::GuildRoleUpdate(_) => Self::handle(raw_event, EventType::GuildRoleUpdate).await,
            // Event::GuildUnavailable(_) => Self::handle(raw_event, EventType::GuildUnavailable).await
            // Event::GuildUpdate(_) => Self::handle(raw_event, EventType::GuildUpdate).await,
            // Event::MessageCreate(_) => Self::handle(raw_event, EventType::MessageCreate).await,
            // Event::MessageDelete(_) => Self::handle(raw_event, EventType::MessageDelete).await,
            // Event::MessageDeleteBulk(_) => Self::handle(raw_event, EventType::MessageDeleteBulk).await
            // Event::MessageUpdate(_) => Self::handle(raw_event, EventType::MessageUpdate).await,
            // Event::ReactionAdd(_) => Self::handle(raw_event, EventType::ReactionAdd).await,
            // Event::ReactionRemove(_) => Self::handle(raw_event, EventType::ReactionRemove).await,
            // Event::ReactionRemoveAll(_) => Self::handle(raw_event, EventType::ReactionRemoveAll).await
            // Event::VoiceStateUpdate(_) => Self::handle(raw_event, EventType::VoiceStateUpdate).await
            // Event::VoiceServerUpdate(_) => Self::handle(raw_event, EventType::VoiceServerUpdate).await
            // Event::WebhookUpdate(_) => Self::handle(raw_event, EventType::WebhookUpdate).await,
        }
    }
}

fn try_serialize<T: Serialize>(source: &T) -> Option<serde_json::Value> {
    let result = serde_json::to_value(source);
    if let Err(e) = result {
        debug!("an error occurred while serializing event data: {:?}", e);
        None
    } else {
        result.ok()
    }
}

#[async_trait]
impl RawEventHandler for Handler {
    /// Triggers on every event coming in from the gateway
    async fn raw_event(&self, _ctx: Context, raw_event: Event) {
        trace!("Event: {:?}", raw_event);
        match self.normalize(raw_event).await {
            None => {}
            Some(_) => {
                // TODO consume
            }
        }
    }
}
