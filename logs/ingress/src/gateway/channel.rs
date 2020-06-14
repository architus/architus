use crate::audit_log;
use crate::gateway::{GatewayContext, NormalizedEvent, Source};
use logs_lib::id;
use logs_lib::{ActionOrigin, ActionType, AuditLogEntryType};
use serde_json;
use serenity;
use serenity::model::channel::Channel;
use serenity::model::event::Event;
use serenity::model::guild::AuditLogEntry;
use serenity::model::id::{ChannelId, GuildId, UserId};
use serenity::model::permissions::Permissions;
use std::sync::Arc;

// Channel event wrapper
#[derive(Clone, Debug)]
pub struct ChannelEvent(Event);

impl ChannelEvent {
    // Extracts the action type
    fn action_type(&self) -> ActionType {
        match self.0 {
            Event::ChannelCreate(_) => ActionType::ChannelCreate,
            Event::ChannelDelete(_) => ActionType::ChannelDelete,
            Event::ChannelUpdate(_) => ActionType::ChannelUpdate,
            _ => panic!("invalid channel event type"),
        }
    }

    // Extracts the audit log entry type
    fn audit_log_entry_type(&self) -> AuditLogEntryType {
        match self.0 {
            Event::ChannelCreate(_) => AuditLogEntryType::ChannelCreate,
            Event::ChannelDelete(_) => AuditLogEntryType::ChannelDelete,
            Event::ChannelUpdate(_) => AuditLogEntryType::ChannelUpdate,
            _ => panic!("invalid channel event type"),
        }
    }

    // Extracts the underlying Channel
    fn channel(&self) -> &Channel {
        match &self.0 {
            Event::ChannelCreate(event) => &event.channel,
            Event::ChannelDelete(event) => &event.channel,
            Event::ChannelUpdate(event) => &event.channel,
            _ => panic!("invalid channel event type"),
        }
    }

    // Attempts to serialize the underlying channel gateway event
    fn serialize(&self) -> Result<serde_json::Value, serde_json::Error> {
        match &self.0 {
            Event::ChannelCreate(event) => serde_json::to_value(event),
            Event::ChannelDelete(event) => serde_json::to_value(event),
            Event::ChannelUpdate(event) => serde_json::to_value(event),
            _ => panic!("invalid channel event type"),
        }
    }
}

/// Handles the channel guild event, attempting to find the corresponding audit log entry
/// and attaching it to the built action as a hybrid action.
///
/// Only supports `ChannelCreate`, `ChannelDelete`, and `ChannelUpdate` events
pub async fn handle(event: Event, context: GatewayContext) -> Option<NormalizedEvent> {
    let event = ChannelEvent(event);
    let action_type = event.action_type();
    let audit_log_entry_type = event.audit_log_entry_type();
    let channel_id: ChannelId = event.channel().id();

    let mut agent_id: Option<UserId> = None;
    let mut guild_id: Option<GuildId> = None;
    let mut audit_log_entry: Option<AuditLogEntry> = None;
    match event.channel() {
        Channel::Guild(guild_channel) => {
            let cache = Arc::clone(&context.discord.cache);
            if let Some(guild) = guild_channel.guild(cache).await {
                guild_id = Some(guild.id);
                // Make sure the bot has permissions first
                if context.has_perms(&guild, Permissions::VIEW_AUDIT_LOG).await {
                    // try to access the audit log entry corresponding to this
                    let entry_result = audit_log::get_entry(
                        context.discord.http,
                        guild,
                        audit_log_entry_type,
                        Some(id::extract_timestamp(channel_id.0)),
                        |entry| {
                            entry
                                .target_id
                                .map(|id| id == channel_id.0)
                                .unwrap_or(false)
                        },
                    )
                    .await;

                    if let Ok(entry) = entry_result {
                        agent_id = Some(entry.user_id);
                        audit_log_entry = Some(entry);
                    }
                }
            }
        }
        _ => {}
    }

    let event_json: serde_json::Value = event.serialize().unwrap_or_else(|err| {
        serde_json::to_value(format!(
            "Serialization failed for channel event (channel id {:?}): {}",
            channel_id.0, err
        ))
        .unwrap()
    });

    match audit_log_entry {
        // if there was a corresponding audit log entry, then the event is hybrid
        Some(audit_log) => Some(NormalizedEvent {
            action_type,
            id: context.id_provisioner.provision(),
            origin: ActionOrigin::Hybrid,
            source: Source::hybrid(&event_json, &audit_log),
            timestamp: id::extract_timestamp(channel_id.0),
            subject_id: Some(channel_id.0),
            guild_id,
            agent_id,
            audit_log_id: Some(audit_log.id),
        }),
        // otherwise it's a normal gateway event
        None => Some(NormalizedEvent {
            action_type,
            id: context.id_provisioner.provision(),
            origin: ActionOrigin::Gateway,
            source: Source::gateway(&event_json),
            timestamp: id::extract_timestamp(channel_id.0),
            subject_id: Some(channel_id.0),
            guild_id,
            agent_id: None,
            audit_log_id: None,
        }),
    }
}
