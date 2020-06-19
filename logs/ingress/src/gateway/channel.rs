use crate::audit_log;
use crate::gateway::{GatewayContext, NormalizedEvent, Source};
use logs_lib::id;
use logs_lib::{to_json, ActionOrigin, ActionType, AuditLogEntryType};
use serenity;
use serenity::model::channel::Channel;
use serenity::model::event::Event;
use serenity::model::guild::AuditLogEntry;
use serenity::model::permissions::Permissions;
use std::sync::Arc;
use std::time::Duration;

// Channel event wrapper
#[derive(Clone, Debug)]
pub struct ChannelEvent<'a>(&'a Event);

impl<'a> ChannelEvent<'a> {
    /// Extracts the action type
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

    /// Extracts the underlying Channel
    fn channel(&self) -> &Channel {
        match &self.0 {
            Event::ChannelCreate(event) => &event.channel,
            Event::ChannelDelete(event) => &event.channel,
            Event::ChannelUpdate(event) => &event.channel,
            _ => panic!("invalid channel event type"),
        }
    }

    /// Attempts to get the underlying timestamp behind the event
    fn underlying_ts(&self) -> Option<u64> {
        match &self.0 {
            Event::ChannelCreate(event) => Some(id::extract_timestamp(event.channel.id().0)),
            Event::ChannelDelete(_) | Event::ChannelUpdate(_) => None,
            _ => panic!("invalid channel event type"),
        }
    }

    /// Attempts to get the audit log entry behind the wrapped channel event
    async fn audit_log(&self, context: &GatewayContext) -> Option<AuditLogEntry> {
        if let Channel::Guild(guild_channel) = self.channel() {
            // Try to extract the guild from the channel
            let cache = Arc::clone(&context.discord.cache);
            if let Some(guild) = guild_channel.guild(cache).await {
                // Make sure the bot has permissions first
                if context.has_perms(&guild, Permissions::VIEW_AUDIT_LOG).await {
                    let channel_id = self.channel().id();
                    let mut search = audit_log::SearchQuery {
                        entry_type: Some(self.audit_log_entry_type()),
                        ..audit_log::SearchQuery::new(guild.id, |entry: &AuditLogEntry| {
                            entry.target_id == Some(channel_id.0)
                        })
                    };

                    // For ChannelCreate, we can use the underlying timestamp
                    // (extracted from the channel's Id), for the rest, we use
                    // the context's timestamp (captured upon receiving the gateway event)
                    search.target_timestamp = if let Event::ChannelCreate(_) = &self.0 {
                        Some(id::extract_timestamp(channel_id.0))
                    } else {
                        None
                    };

                    // For ChannelCreate and ChannelDelete, there will only be one
                    // audit log entry of that type and for the given channel in the log.
                    // However, for ChannelUpdate, there might be more than one,
                    // so we use an increasing search bound
                    search.strategy = if let Event::ChannelUpdate(_) = &self.0 {
                        let max_duration = Duration::from_secs(3);
                        audit_log::Strategy::GrowingInterval { max: max_duration }
                    } else {
                        audit_log::Strategy::First
                    };

                    return audit_log::get_entry(Arc::clone(&context.discord.http), search)
                        .await
                        .ok();
                }
            }
        }

        None
    }
}

/// Handles the channel guild event, attempting to find the corresponding audit log entry
/// and attaching it to the built action as a hybrid action.
///
/// Only supports `ChannelCreate`, `ChannelDelete`, and `ChannelUpdate` events
pub async fn handle(raw_event: Event, context: GatewayContext) -> Option<NormalizedEvent> {
    let event = ChannelEvent(&raw_event);
    let mut normalized = NormalizedEvent {
        action_type: event.action_type(),
        subject_id: Some(event.channel().id().0),
        origin: ActionOrigin::Gateway,
        source: Source::gateway(&raw_event),
        ..context.event(event.underlying_ts())
    };

    if let Channel::Guild(guild_channel) = event.channel() {
        normalized.guild_id = Some(guild_channel.guild_id);

        // Try to access the audit log entry corresponding to this gateway event
        if let Some(entry) = event.audit_log(&context).await {
            normalized.agent_id = Some(entry.user_id);
            normalized.audit_log_id = Some(entry.id);
            normalized.origin = ActionOrigin::Hybrid;
            normalized.source.audit_log = to_json(&entry);
        }
    }

    Some(normalized)
}
