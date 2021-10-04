//! Defines processors for the following events:
//! - `MessageSend` (from `GatewayEventType::MessageCreate`)
//! - `MessageReply` (from `GatewayEventType::MessageCreate`)
//! - `MessageEdit` (from `GatewayEventType::MessageUpdate`)
//! - `MessageDelete` (from `GatewayEventType::MessageDelete`)
//! - `MessageBulkDelete` (from `GatewayEventType::MessageDeleteBulk`,
//!    using audit log info to form a hybrid event)

use crate::event::{
    Agent, Channel, Content, Entity, IdParams, Message, Nickname, NormalizedEvent, Source, UserLike,
};
use crate::gateway::{Processor, ProcessorContext, ProcessorFleet};
use crate::rpc::logs::event::{AgentSpecialType, EventOrigin, EventType};
use chrono::DateTime;
use fancy_regex::Regex;
use lazy_static::lazy_static;
use std::collections::BTreeSet;
use std::convert::TryFrom;
use twilight_model::channel::message::{Message as DiscordMessage, MessageType};
use twilight_model::gateway::event::EventType as GatewayEventType;
use twilight_model::gateway::payload::MessageCreate;
use unic_segment::Graphemes;
use url::Url;

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
fn message_send_reply(ctx: ProcessorContext<'_>) -> anyhow::Result<NormalizedEvent> {
    // Deserialize the typed twilight event struct
    let event = serde_json::from_value::<MessageCreate>(ctx.source.clone())?;

    let channel_id = event.channel_id.0;
    let message_id = event.id.0;
    let timestamp = DateTime::parse_from_rfc3339(&event.timestamp)?;
    let timestamp_ms = u64::try_from(timestamp.timestamp_millis())?;
    let agent = source_agent(&ctx, &event.0);
    let content = source_content(&ctx, &event.0);

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
            Err(anyhow::anyhow!(format!("message type {:?} not implemented", event.kind)))
        }
    }
}

fn source_agent(ctx: &ProcessorContext<'_>, message: &DiscordMessage) -> Agent {
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
    Agent {
        special_type: Agent::type_from_discord_user(&message.author, &ctx.config),
        entity: Entity::UserLike(UserLike {
            id: message.author.id.0,
            name: Some(message.author.name.clone()),
            nickname: message
                .member
                .as_ref()
                .map(|m| Nickname::from(m.nick.clone())),
            discriminator: message.author.discriminator.parse::<u16>().ok(),
            color: None,
        }),
        webhook_username: None,
    }
}

fn source_content(ctx: &ProcessorContext<'_>, message: &DiscordMessage) -> Content {
    // Thank you Stack Overflow :)
    // https://stackoverflow.com/a/17773849/13192375
    const URL_REGEX_RAW: &'static str = r#"(https?:\/\/(?:www\.|(?!www))[a-zA-Z0-9][a-zA-Z0-9-]+[a-zA-Z0-9]\.[^\s]{2,}|www\.[a-zA-Z0-9][a-zA-Z0-9-]+[a-zA-Z0-9]\.[^\s]{2,}|https?:\/\/(?:www\.|(?!www))[a-zA-Z0-9]+\.[^\s]{2,}|www\.[a-zA-Z0-9]+\.[^\s]{2,})"#;

    lazy_static! {
        static ref CHANNEL_MENTION_REGEX: Regex = Regex::new(r#"<#([0-9]+)>"#).unwrap();
        static ref CUSTOM_EMOJI_MENTION_REGEX: Regex =
            Regex::new(r#"<a?:([A-Za-z0-9_-]+):([0-9]+)>"#).unwrap();
        static ref URL_REGEX: Regex = Regex::new(URL_REGEX_RAW).unwrap();
    }

    let mut mentioned_channels = BTreeSet::<u64>::new();
    for maybe_capture in CHANNEL_MENTION_REGEX.captures_iter(&message.content) {
        if let Ok(capture) = maybe_capture {
            if let Ok(channel_id) = &capture[1].parse::<u64>() {
                mentioned_channels.insert(*channel_id);
            }
        }
    }

    let mut mentioned_custom_emojis = BTreeSet::<u64>::new();
    let mut mentioned_custom_emoji_names = BTreeSet::<String>::new();
    for maybe_capture in CUSTOM_EMOJI_MENTION_REGEX.captures_iter(&message.content) {
        if let Ok(capture) = maybe_capture {
            if let Ok(custom_emoji_id) = &capture[2].parse::<u64>() {
                mentioned_custom_emojis.insert(*custom_emoji_id);
            }

            mentioned_custom_emoji_names.insert(String::from(&capture[1]));
        }
    }

    let mut url_stems = BTreeSet::<String>::new();
    for maybe_capture in URL_REGEX.captures_iter(&message.content) {
        if let Ok(capture) = maybe_capture {
            let mut raw_url = String::from(&capture[0]);
            if !raw_url.starts_with("https://") && !raw_url.starts_with("http://") {
                raw_url.push_str("https://");
            }

            if let Some(stems) = get_url_stems(raw_url) {
                url_stems.extend(stems.into_iter());
            }
        }
    }

    Content {
        inner: message.content.clone(),
        users_mentioned: message
            .mentions
            .iter()
            .map(|u| u.id.0)
            .collect::<BTreeSet<_>>()
            .into_iter()
            .collect::<Vec<_>>(),
        // Channels mentioned from Discord's message object are unreliable;
        // so we manually parse them here.
        channels_mentioned: mentioned_channels.into_iter().collect::<Vec<_>>(),
        roles_mentioned: message
            .mention_roles
            .iter()
            .map(|u| u.0)
            .collect::<BTreeSet<_>>()
            .into_iter()
            .collect::<Vec<_>>(),
        emojis_used: source_unicode_emojis(ctx, &message.content),
        custom_emojis_used: mentioned_custom_emojis.into_iter().collect::<Vec<_>>(),
        custom_emoji_names_used: mentioned_custom_emoji_names.into_iter().collect::<Vec<_>>(),
        url_stems: url_stems.into_iter().collect::<Vec<_>>(),
    }
}

fn get_url_stems(raw_url: impl AsRef<str>) -> Option<Vec<String>> {
    let parsed_url = Url::parse(raw_url.as_ref()).ok()?;
    let domain = parsed_url.host_str()?;

    let mut segments_reversed = domain.split(".").collect::<Vec<_>>();
    segments_reversed.reverse();
    if segments_reversed.len() < 2 {
        return None;
    }

    let (first, rest) = segments_reversed.as_slice().split_at(1);
    let mut accum = first.into_iter().collect::<Vec<_>>();
    let mut url_stems = Vec::<String>::new();
    for segment in rest {
        accum.push(segment);
        url_stems.push(
            accum
                .iter()
                .rev()
                .map(|s| String::from(**s))
                .collect::<Vec<String>>()
                .join("."),
        );
    }

    Some(url_stems)
}

fn source_unicode_emojis(ctx: &ProcessorContext<'_>, content: &str) -> Vec<String> {
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
        if let Some(shortcodes) = ctx.emojis.to_shortcodes(grapheme_cluster) {
            all_shortcodes.extend(shortcodes.iter().cloned());
        }
    }

    all_shortcodes.into_iter().collect::<Vec<_>>()
}

/// Handles `GatewayEventType::MessageUpdate`
fn message_edit(_ctx: ProcessorContext<'_>) -> anyhow::Result<NormalizedEvent> {
    // TODO implement
    Err(anyhow::anyhow!("MessageUpdate not implemented"))
}

/// Handles `GatewayEventType::MessageDelete`
fn message_delete(_ctx: ProcessorContext<'_>) -> anyhow::Result<NormalizedEvent> {
    // TODO implement
    Err(anyhow::anyhow!("MessageDelete not implemented"))
}

/// Handles `GatewayEventType::MessageDeleteBulk`
fn message_delete_bulk(_ctx: ProcessorContext<'_>) -> anyhow::Result<NormalizedEvent> {
    // TODO implement
    Err(anyhow::anyhow!("MessageDeleteBulk not implemented"))
}
