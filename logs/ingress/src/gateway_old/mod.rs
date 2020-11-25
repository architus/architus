mod channel;

use crate::event::{NormalizedEvent, Source};
use lazy_static::lazy_static;
use log::{debug, warn};
use logs_lib::id::IdProvisioner;
use logs_lib::{time, ActionOrigin, ActionType};
use serde_json;
use serenity;
use serenity::async_trait;
use serenity::client::bridge::gateway::GatewayIntents;
use serenity::model::event::Event;
use serenity::model::guild::Guild;
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

pub struct GatewayContext {
    discord: Context,
    id_provisioner: Arc<IdProvisioner>,
    timestamp: u64,
}

impl GatewayContext {
    /// Utility to get live permissions for the bot user for the given guild
    #[must_use]
    async fn has_perms(&self, guild: &Guild, target: Permissions) -> bool {
        let user_id = self.discord.cache.current_user().await.id;
        let bot_perms = guild.member_permissions(user_id).await;
        let mut base_perms = target;
        base_perms.remove(bot_perms);
        base_perms.is_empty()
    }

    /// Constructs a normalized event using default empty values for most fields.
    /// Uses the internal timestamp as the Id timestamp and the fallback
    /// for the underlying timestamp if None is supplied
    #[must_use]
    fn event(&self, underlying_ts: Option<u64>) -> NormalizedEvent {
        let timestamp: u64 = underlying_ts.unwrap_or(self.timestamp);
        NormalizedEvent {
            id: self.id_provisioner.with_ts(self.timestamp),
            timestamp,
            source: Source::empty(),
            origin: ActionOrigin::Internal,
            action_type: ActionType::Unknown,
            reason: None,
            guild_id: None,
            agent_id: None,
            subject_id: None,
            audit_log_id: None,
        }
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
    #[must_use]
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
            timestamp: time::millisecond_ts(),
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

/// Attempts to normalize a gateway event into a normalized event struct,
/// including the corresponding audit log entry (if it exists)
async fn normalize(raw_event: Event, context: GatewayContext) -> Option<NormalizedEvent> {
    match raw_event {
        Event::ChannelCreate(_) | Event::ChannelDelete(_) | Event::ChannelUpdate(_) => {
            channel::handle(raw_event, context).await
        }
        Event::Unknown(_) | _ => {
            warn!("Received unhandled event: {:?}", raw_event);
            Some(NormalizedEvent {
                action_type: ActionType::Unknown,
                source: Source::gateway(&raw_event),
                origin: ActionOrigin::Gateway,
                ..context.event(None)
            })
        }
    }
}
