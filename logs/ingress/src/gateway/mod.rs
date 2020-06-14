mod channel;

use lazy_static::lazy_static;
use log::{debug, warn};
use logs_lib::id::{HoarFrost, IdProvisioner};
use logs_lib::{time, ActionOrigin, ActionType};
use serde::ser;
use serde::Serialize;
use serde_json;
use serenity;
use serenity::async_trait;
use serenity::client::bridge::gateway::GatewayIntents;
use serenity::model::event::Event;
use serenity::model::guild::Guild;
use serenity::model::id::{AuditLogEntryId, GuildId, UserId};
use serenity::model::permissions::Permissions;
use serenity::prelude::*;
use std::sync::Arc;

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
#[derive(Clone, PartialEq, Debug, Serialize)]
pub struct NormalizedEvent {
    pub id: HoarFrost,
    // Unix timestamp
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

#[derive(Clone, PartialEq, Debug, Serialize)]
pub struct Source {
    pub gateway: Option<serde_json::Value>,
    pub audit_log: Option<serde_json::Value>,
}

impl Source {
    fn gateway<T: ser::Serialize>(gateway_event: &T) -> Self {
        Self {
            gateway: try_serialize(gateway_event),
            audit_log: None,
        }
    }

    fn hybrid<T: ser::Serialize, V: ser::Serialize>(
        gateway_event: &T,
        audit_log_entry: &V,
    ) -> Self {
        Self {
            gateway: try_serialize(gateway_event),
            audit_log: try_serialize(audit_log_entry),
        }
    }
}

pub struct GatewayContext {
    discord: Context,
    id_provisioner: Arc<IdProvisioner>,
}

impl GatewayContext {
    pub async fn has_perms(&self, guild: &Guild, target: Permissions) -> bool {
        let user_id = self.discord.cache.current_user().await.id;
        let bot_perms = guild.member_permissions(user_id).await;
        let mut base_perms = target;
        base_perms.remove(bot_perms);
        base_perms.is_empty()
    }
}

/// Bot event handler (used to dispatch events to the main service)
pub struct Handler {
    id_provisioner: Arc<IdProvisioner>,
}

impl Default for Handler {
    fn default() -> Self {
        Self {
            id_provisioner: Arc::new(IdProvisioner::new()),
        }
    }
}

impl Handler {
    pub fn new() -> Self {
        Default::default()
    }
}

#[async_trait]
impl RawEventHandler for Handler {
    /// triggers on every event coming in from the gateway
    async fn raw_event(&self, base_context: Context, event: Event) {
        // Wraps the context in a single object
        let context = GatewayContext {
            discord: base_context,
            id_provisioner: Arc::clone(&self.id_provisioner),
        };

        // Fork every handler to let it execute long-running tasks
        tokio::spawn(async move {
            match normalize(event, context).await {
                None => {}
                Some(event) => {
                    if let Ok(json_str) = serde_json::to_string(&event) {
                        debug!("{}", json_str);
                    }
                }
            }
        });
    }
}

async fn normalize(raw_event: Event, context: GatewayContext) -> Option<NormalizedEvent> {
    match raw_event {
        // handle all log ingestion events
        Event::ChannelCreate(_) | Event::ChannelDelete(_) | Event::ChannelUpdate(_) => channel::handle(raw_event, context).await,
        // Event::ChannelPinsUpdate(_) => Self::handle(raw_event, EventType::ChannelPinsUpdate).await
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
        Event::Unknown(event) => {
            warn!("Received Event::Unknown: {:?}", event);
            Some(NormalizedEvent {
                action_type: ActionType::Unknown,
                id: context.id_provisioner.provision(),
                source: Source::gateway(&event),
                origin: ActionOrigin::Gateway,
                timestamp: time::millisecond_ts(),
                subject_id: None,
                guild_id: None,
                agent_id: None,
                audit_log_id: None,
            })
        },
        _ => None
    }
}

fn try_serialize<T: ser::Serialize>(source: &T) -> Option<serde_json::Value> {
    let result = serde_json::to_value(source);
    if let Err(e) = result {
        debug!("an error occurred while serializing event data: {:?}", e);
        None
    } else {
        result.ok()
    }
}
