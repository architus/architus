use crate::audit_log;
use lazy_static::lazy_static;
use log::{debug, trace, warn};
use logs_lib::id::{self, HoarFrost, IdProvisioner};
use logs_lib::{time, ActionOrigin, ActionType, AuditLogEntryType};
use serde::ser::Serialize;
use serde_json;
use serenity;
use serenity::async_trait;
use serenity::client::bridge::gateway::GatewayIntents;
use serenity::model::event::ChannelCreateEvent;
use serenity::model::event::Event;
use serenity::model::guild::AuditLogEntry;
use serenity::model::id::{AuditLogEntryId, GuildId, UserId};
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
    pub id: HoarFrost,
    pub timestamp: u64,
    pub source: Source,
    pub origin: ActionOrigin,
    pub action_type: ActionType,
    pub guild_id: Option<GuildId>,
    pub agent_id: Option<UserId>,
    // Subject Id can be any id
    pub subject_id: Option<u64>,
    pub audit_log_id: Option<AuditLogEntryId>,
}

pub struct Source {
    pub gateway: Option<serde_json::Value>,
    pub audit_log: Option<serde_json::Value>,
}

impl Source {
    fn gateway<T: Serialize>(gateway_event: &T) -> Self {
        Self {
            gateway: try_serialize(gateway_event),
            audit_log: None,
        }
    }

    fn hybrid<T: Serialize, V: Serialize>(gateway_event: &T, audit_log_entry: &V) -> Self {
        Self {
            gateway: try_serialize(gateway_event),
            audit_log: try_serialize(audit_log_entry),
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

    async fn channel_create(
        &self,
        event: ChannelCreateEvent,
        context: Context,
    ) -> Option<NormalizedEvent> {
        let channel_id = event.channel.id();
        let mut agent_id: Option<UserId> = None;
        let mut guild_id: Option<GuildId> = None;
        let mut audit_log_entry: Option<AuditLogEntry> = None;

        if let Some(guild_channel) = event.channel.clone().guild() {
            if let Some(guild) = guild_channel.guild(context.cache).await {
                guild_id = Some(guild.id);

                // try to access the audit log entry corresponding to this
                let entry_future = audit_log::get_entry(
                    context.http,
                    guild,
                    AuditLogEntryType::ChannelCreate,
                    Some(id::extract_timestamp(channel_id.0)),
                    |entry| {
                        entry
                            .target_id
                            .map(|id| id == channel_id.0)
                            .unwrap_or(false)
                    },
                );
                if let Ok(entry) = entry_future.await {
                    agent_id = Some(entry.user_id);
                    audit_log_entry = Some(entry);
                }
            }
        }

        match audit_log_entry {
            // if there was a corresponding audit log entry, then the event is hybrid
            Some(audit_log) => Some(NormalizedEvent {
                action_type: ActionType::ChannelCreate,
                id: self.id_provisioner.provision(),
                origin: ActionOrigin::Hybrid,
                source: Source::hybrid(&event, &audit_log),
                timestamp: id::extract_timestamp(channel_id.0),
                subject_id: Some(channel_id.0),
                guild_id,
                agent_id,
                audit_log_id: Some(audit_log.id),
            }),
            // otherwise it's a normal gateway event
            None => Some(NormalizedEvent {
                action_type: ActionType::ChannelCreate,
                id: self.id_provisioner.provision(),
                origin: ActionOrigin::Gateway,
                source: Source::gateway(&event),
                timestamp: id::extract_timestamp(channel_id.0),
                subject_id: Some(channel_id.0),
                guild_id,
                agent_id: None,
                audit_log_id: None,
            }),
        }
    }

    async fn normalize(&self, raw_event: Event, context: Context) -> Option<NormalizedEvent> {
        match raw_event {
            // handle all log ingestion events
            Event::ChannelCreate(event) => self.channel_create(event, context).await,
            Event::Unknown(event) => {
                warn!("Received Event::Unknown: {:?}", event);
                Some(NormalizedEvent {
                    action_type: ActionType::Unknown,
                    id: self.id_provisioner.provision(),
                    source: Source::gateway(&event),
                    origin: ActionOrigin::Hybrid,
                    timestamp: time::millisecond_ts(),
                    subject_id: None,
                    guild_id: None,
                    agent_id: None,
                    audit_log_id: None,
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

#[async_trait]
impl RawEventHandler for Handler {
    /// triggers on every event coming in from the gateway
    async fn raw_event(&self, context: Context, raw_event: Event) {
        trace!("Event: {:?}", raw_event);
        match self.normalize(raw_event, context).await {
            None => {}
            Some(_) => {
                // TODO consume
            }
        }
    }
}
