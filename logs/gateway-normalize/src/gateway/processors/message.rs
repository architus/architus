//! Defines processors for the following events:
//! - `MessageSend` (from `GatewayEventType::MessageCreate`)
//! - `MessageReply` (from `GatewayEventType::MessageCreate`)
//! - `MessageEdit` (from `GatewayEventType::MessageUpdate`)
//! - `MessageDelete` (from `GatewayEventType::MessageDelete`)
//! - `MessageBulkDelete` (from `GatewayEventType::MessageDeleteBulk`,
//!    using audit log info to form a hybrid event)

use crate::gateway::{Processor, ProcessorContext, ProcessorError, ProcessorFleet};
use architus_logs_lib::event::{
    Agent, AgentSpecialType, Channel, Content, Entity, EventOrigin, EventType, IdParams, Message,
    Nickname, NormalizedEvent, Source, UserLike,
};
use chrono::DateTime;
use std::collections::BTreeSet;
use std::convert::TryFrom;
use twilight_model::channel::message::{Message as DiscordMessage, MessageType};
use twilight_model::gateway::event::EventType as GatewayEventType;
use twilight_model::gateway::payload::{MessageCreate, MessageUpdate};
use twilight_model::user::User as DiscordUser;
use unic_segment::Graphemes;

pub fn register_all(fleet: &mut ProcessorFleet) {
    fleet.register(
        GatewayEventType::MessageCreate,
        Processor::sync(message_send_reply),
    );
    fleet.register(
        GatewayEventType::MessageUpdate,
        Processor::sync(message_edit),
    );
    fleet.register(
        GatewayEventType::MessageDelete,
        Processor::sync(message_delete),
    );
    fleet.register(
        GatewayEventType::MessageDeleteBulk,
        Processor::sync(message_delete_bulk),
    );
}

/// Handles `GatewayEventType::MessageCreate`
fn message_send_reply(ctx: ProcessorContext<'_>) -> Result<NormalizedEvent, ProcessorError> {
    // Deserialize the typed twilight event struct
    let event = serde_json::from_value::<MessageCreate>(ctx.source.clone())
        .map_err(|err| ProcessorError::Fatal(err.into()))?;

    let channel_id = event.channel_id.0;
    let message_id = event.id.0;
    let timestamp = DateTime::parse_from_rfc3339(&event.timestamp)
        .map_err(|err| ProcessorError::Fatal(err.into()))?;
    let timestamp_ms = u64::try_from(timestamp.timestamp_millis())
        .map_err(|err| ProcessorError::Fatal(err.into()))?;
    let agent = agent_from_source(&ctx, &event.0);
    let content = source_content(&ctx, event.content.clone());

    match event.kind {
        MessageType::Regular => Ok(NormalizedEvent {
            event_type: EventType::MessageSend,
            id_params: IdParams::One(message_id),
            timestamp: timestamp_ms,
            guild_id: ctx.event.guild_id,
            reason: None,
            audit_log_id: None,
            channel: Some(Channel {
                id: channel_id,
                ..Channel::default()
            }),
            agent: Some(agent),
            subject: Some(Entity::Message(Message { id: message_id })),
            auxiliary: None,
            content,
            origin: EventOrigin::Gateway,
            source: Source {
                gateway: Some(ctx.source),
                ..Source::default()
            },
        }),
        MessageType::Reply => {
            let replied_message = event
                .reference
                .as_ref()
                .and_then(|r| r.message_id)
                .map(|id| Entity::Message(Message { id: id.0 }));
            Ok(NormalizedEvent {
                event_type: EventType::MessageReply,
                id_params: IdParams::One(message_id),
                timestamp: timestamp_ms,
                guild_id: ctx.event.guild_id,
                reason: None,
                audit_log_id: None,
                channel: Some(Channel {
                    id: channel_id,
                    ..Channel::default()
                }),
                agent: Some(agent),
                subject: Some(Entity::Message(Message { id: message_id })),
                auxiliary: replied_message,
                content,
                origin: EventOrigin::Gateway,
                source: Source {
                    gateway: Some(ctx.source),
                    ..Source::default()
                },
            })
        }
        _ => {
            // TODO support additional message types
            // https://discord.com/developers/docs/resources/channel#message-object-message-types
            Err(ProcessorError::Drop)
        }
    }
}

/// Handles `GatewayEventType::MessageUpdate`
fn message_edit(ctx: ProcessorContext<'_>) -> Result<NormalizedEvent, ProcessorError> {
    // Deserialize the typed twilight event struct
    let event = serde_json::from_value::<MessageUpdate>(ctx.source.clone())
        .map_err(|err| ProcessorError::Fatal(err.into()))?;

    // Make sure the event has a valid author, content, and edited_timestamp field.
    // If not, it is a class of message update we don't care to log.
    let (author, content, edited_timestamp) =
        match (&event.author, &event.content, &event.edited_timestamp) {
            (Some(author), Some(content), Some(edited_content)) => {
                (author, content, edited_content)
            }
            _ => {
                slog::debug!(
                    ctx.logger,
                    "dropping MessageUpdate event due to missing author or content";
                    "event" => ?event,
                );
                return Err(ProcessorError::Drop);
            }
        };

    let channel_id = event.channel_id.0;
    let message_id = event.id.0;
    let timestamp = DateTime::parse_from_rfc3339(edited_timestamp)
        .map_err(|err| ProcessorError::Fatal(err.into()))?;
    let timestamp_ms = u64::try_from(timestamp.timestamp_millis())
        .map_err(|err| ProcessorError::Fatal(err.into()))?;

    let agent = construct_agent(&ctx, author, None);
    let content = source_content(&ctx, content.clone());

    match event.kind {
        Some(MessageType::Regular) | Some(MessageType::Reply) => Ok(NormalizedEvent {
            event_type: EventType::MessageEdit,
            id_params: IdParams::Two(message_id, timestamp_ms),
            timestamp: timestamp_ms,
            guild_id: ctx.event.guild_id,
            reason: None,
            audit_log_id: None,
            channel: Some(Channel {
                id: channel_id,
                ..Channel::default()
            }),
            agent: Some(agent),
            subject: Some(Entity::Message(Message { id: message_id })),
            auxiliary: None,
            content,
            origin: EventOrigin::Gateway,
            source: Source {
                gateway: Some(ctx.source),
                ..Source::default()
            },
        }),
        _ => {
            // TODO support additional message types
            // https://discord.com/developers/docs/resources/channel#message-object-message-types
            Err(ProcessorError::Drop)
        }
    }
}

/// Handles `GatewayEventType::MessageDelete`
fn message_delete(_ctx: ProcessorContext<'_>) -> Result<NormalizedEvent, ProcessorError> {
    // TODO implement
    Err(ProcessorError::Drop)
}

/// Handles `GatewayEventType::MessageDeleteBulk`
fn message_delete_bulk(_ctx: ProcessorContext<'_>) -> Result<NormalizedEvent, ProcessorError> {
    // TODO implement
    Err(ProcessorError::Drop)
}

fn agent_from_source(ctx: &ProcessorContext<'_>, message: &DiscordMessage) -> Agent {
    // Check if the agent was a webhook
    if message.webhook_id.is_some() {
        return Agent {
            special_type: AgentSpecialType::Webhook,
            entity: Entity::UserLike(UserLike {
                id: message.author.id.0,
                ..UserLike::default()
            }),
            webhook_username: Some(message.author.name.clone()),
        };
    }

    // Otherwise, determine the agent data from the author & member fields
    let nickname_option = message
        .member
        .as_ref()
        .map(|m| Nickname::from(m.nick.clone()));
    construct_agent(ctx, &message.author, nickname_option)
}

fn construct_agent(
    ctx: &ProcessorContext<'_>,
    author: &DiscordUser,
    nickname: Option<Nickname>,
) -> Agent {
    Agent {
        special_type: Agent::type_from_discord_user(author, Some(ctx.config.bot_user_id)),
        entity: Entity::UserLike(UserLike {
            id: author.id.0,
            name: Some(author.name.clone()),
            nickname,
            discriminator: author.discriminator.parse::<u16>().ok(),
            color: None,
        }),
        webhook_username: None,
    }
}

fn find_unicode_emojis(content: &str, emoji_db: &crate::emoji::Db) -> Vec<String> {
    // Iterate over every grapheme cluster in the content
    // and check each one to see if it is an emoji by looking it up in our emoji DB.
    // This should be O(n) on the length of the content, and hopefully isn't too slow.
    // Potential alternates:
    // - https://docs.rs/emoji/0.2.1/emoji/lookup_by_glyph/index.html
    // - https://docs.rs/emojis/0.1.2/emojis/fn.lookup.html
    // Known limitations:
    // - doesn't identify '👍🏾' correctly, but does identify '👨‍👧‍👦', '🇦🇺', and '1️⃣'
    let mut all_shortcodes = BTreeSet::<String>::new();
    for grapheme_cluster in Graphemes::new(content) {
        if let Some(shortcodes) = emoji_db.to_shortcodes(grapheme_cluster) {
            all_shortcodes.extend(shortcodes.iter().cloned());
        }
    }

    all_shortcodes.into_iter().collect::<Vec<_>>()
}

pub fn source_content(ctx: &ProcessorContext<'_>, message_content: String) -> Content {
    let custom_emojis = architus_logs_lib::content::find_custom_emoji_uses(&message_content);
    Content {
        users_mentioned: architus_logs_lib::content::find_user_mentions(&message_content),
        roles_mentioned: architus_logs_lib::content::find_role_mentions(&message_content),
        channels_mentioned: architus_logs_lib::content::find_channel_mentions(&message_content),
        emojis_used: find_unicode_emojis(&message_content, &ctx.emojis),
        custom_emojis_used: custom_emojis.ids,
        custom_emoji_names_used: custom_emojis
            .names
            .into_iter()
            .map(String::from)
            .collect::<Vec<_>>(),
        url_stems: architus_logs_lib::content::collect_url_stems(
            architus_logs_lib::content::find_urls(&message_content),
        ),
        inner: message_content,
    }
}
