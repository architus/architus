//! Defines processors for the following events:
//! - `ReactionAdd`
//! - `ReactionRemove`
//! - `ReactionBulkRemove`

use super::{extract, extract_id, extract_member};
use crate::event::{
    Agent, Channel, Content, Emoji, Entity, IdParams, Message, Nickname, NormalizedEvent,
    Source, UserLike,
};
use crate::gateway::path::Path;
use crate::gateway::{Context, Processor, ProcessorFleet};
use crate::rpc::logs::event::{EventOrigin, EventType};
use lazy_static::lazy_static;
use std::fmt::{self, Write as _};
use twilight_model::channel::ReactionType;
use twilight_model::gateway::event::EventType as GatewayEventType;

lazy_static! {
    static ref USER_ID_PATH: Path = Path::from("user_id");
    static ref MESSAGE_ID_PATH: Path = Path::from("message_id");
    static ref CHANNEL_ID_PATH: Path = Path::from("channel_id");
    static ref MEMBER_PATH: Path = Path::from("member");
    static ref EMOJI_PATH: Path = Path::from("emoji");
}

#[allow(clippy::too_many_lines)]
pub fn register_all(fleet: &mut ProcessorFleet) {
    // Register ReactionAdd processor
    fleet.register(
        GatewayEventType::ReactionAdd,
        Processor::sync(|source| {
            let ctx = source.context();

            // Reaction add events include a partial member object that we can use
            let member_option = ctx.gateway(&MEMBER_PATH, extract_member).ok();
            let user = if let Some(member) = member_option {
                UserLike {
                    id: member.user.id.0,
                    name: Some(member.user.name.clone()),
                    nickname: Some(Nickname::from(member.nick.clone())),
                    discriminator: member.user.discriminator.parse::<u16>().ok(),
                    ..UserLike::default()
                }
            } else {
                let id = ctx.gateway(&USER_ID_PATH, extract_id)?;
                UserLike {
                    id,
                    ..UserLike::default()
                }
            };

            let reaction = ctx.gateway(&EMOJI_PATH, extract::<ReactionType>)?;
            let channel_id = ctx.gateway(&CHANNEL_ID_PATH, extract_id)?;
            let message_id = ctx.gateway(&MESSAGE_ID_PATH, extract_id)?;

            Ok(NormalizedEvent {
                event_type: EventType::ReactionAdd,
                id_params: IdParams::Three(user.id, message_id, ctx.event.ingress_timestamp),
                timestamp: ctx.event.ingress_timestamp,
                guild_id: ctx.event.guild_id,
                reason: None,
                audit_log_id: None,
                channel: Some(Channel {
                    id: channel_id,
                    ..Channel::default()
                }),
                agent: Some(Agent {
                    special_type: Agent::type_from_id(user.id, ctx.config),
                    entity: Entity::UserLike(user),
                }),
                subject: Some(Entity::Message(Message { id: message_id })),
                auxiliary: match reaction {
                    ReactionType::Unicode { .. } => None,
                    ReactionType::Custom { id, .. } => Some(Entity::Emoji(Emoji { id: id.0 })),
                },
                content: format_content(reaction, ctx)?,
                origin: EventOrigin::Gateway,
                // ctx to be dropped before we move inner out of source,
                // since ctx's inner field borrows source.inner
                source: Source {
                    gateway: Some(source.inner),
                    ..Source::default()
                },
            })
        }),
    );
    // Register ReactionRemove processor
    fleet.register(
        GatewayEventType::ReactionRemove,
        Processor::sync(|source| {
            let ctx = source.context();

            let user_id = ctx.gateway(&USER_ID_PATH, extract_id)?;
            let reaction = ctx.gateway(&EMOJI_PATH, extract::<ReactionType>)?;
            let channel_id = ctx.gateway(&CHANNEL_ID_PATH, extract_id)?;
            let message_id = ctx.gateway(&MESSAGE_ID_PATH, extract_id)?;

            Ok(NormalizedEvent {
                event_type: EventType::ReactionRemove,
                id_params: IdParams::Three(user_id, message_id, ctx.event.ingress_timestamp),
                timestamp: ctx.event.ingress_timestamp,
                guild_id: ctx.event.guild_id,
                reason: None,
                audit_log_id: None,
                channel: Some(Channel {
                    id: channel_id,
                    ..Channel::default()
                }),
                agent: Some(Agent {
                    special_type: Agent::type_from_id(user_id, ctx.config),
                    entity: Entity::UserLike(UserLike {
                        id: user_id,
                        ..UserLike::default()
                    }),
                }),
                subject: Some(Entity::Message(Message { id: message_id })),
                auxiliary: match reaction {
                    ReactionType::Unicode { .. } => None,
                    ReactionType::Custom { id, .. } => Some(Entity::Emoji(Emoji { id: id.0 })),
                },
                content: format_content(reaction, ctx)?,
                origin: EventOrigin::Gateway,
                // ctx to be dropped before we move inner out of source,
                // since ctx's inner field borrows source.inner
                source: Source {
                    gateway: Some(source.inner),
                    ..Source::default()
                },
            })
        }),
    );
    // Register ReactionBulkRemove processors
    fleet.register(
        GatewayEventType::ReactionRemoveEmoji,
        Processor::sync(|source| {
            let ctx = source.context();

            let reaction = ctx.gateway(&EMOJI_PATH, extract::<ReactionType>)?;
            let channel_id = ctx.gateway(&CHANNEL_ID_PATH, extract_id)?;
            let message_id = ctx.gateway(&MESSAGE_ID_PATH, extract_id)?;

            Ok(NormalizedEvent {
                event_type: EventType::ReactionBulkRemove,
                id_params: IdParams::Two(message_id, ctx.event.ingress_timestamp),
                timestamp: ctx.event.ingress_timestamp,
                guild_id: ctx.event.guild_id,
                reason: None,
                audit_log_id: None,
                channel: Some(Channel {
                    id: channel_id,
                    ..Channel::default()
                }),
                agent: None,
                subject: Some(Entity::Message(Message { id: message_id })),
                auxiliary: match reaction {
                    ReactionType::Unicode { .. } => None,
                    ReactionType::Custom { id, .. } => Some(Entity::Emoji(Emoji { id: id.0 })),
                },
                content: format_content(reaction, ctx)?,
                origin: EventOrigin::Gateway,
                // ctx to be dropped before we move inner out of source,
                // since ctx's inner field borrows source.inner
                source: Source {
                    gateway: Some(source.inner),
                    ..Source::default()
                },
            })
        }),
    );
    fleet.register(
        GatewayEventType::ReactionRemoveAll,
        Processor::sync(|source| {
            let ctx = source.context();

            let channel_id = ctx.gateway(&CHANNEL_ID_PATH, extract_id)?;
            let message_id = ctx.gateway(&MESSAGE_ID_PATH, extract_id)?;

            Ok(NormalizedEvent {
                event_type: EventType::ReactionBulkRemove,
                id_params: IdParams::Two(message_id, ctx.event.ingress_timestamp),
                timestamp: ctx.event.ingress_timestamp,
                guild_id: ctx.event.guild_id,
                reason: None,
                audit_log_id: None,
                channel: Some(Channel {
                    id: channel_id,
                    ..Channel::default()
                }),
                agent: None,
                subject: Some(Entity::Message(Message { id: message_id })),
                auxiliary: None,
                content: Content {
                    inner: String::from("all reactions removed"),
                    ..Content::default()
                },
                origin: EventOrigin::Gateway,
                // ctx to be dropped before we move inner out of source,
                // since ctx's inner field borrows source.inner
                source: Source {
                    gateway: Some(source.inner),
                    ..Source::default()
                },
            })
        }),
    );
}

/// Writes an embedded emoji that will be displayed using rich formatting.
/// If a name is not supplied, then the embed will still work in the logs UI
pub fn write_emoji(
    writer: &mut impl fmt::Write,
    name: Option<&str>,
    id: u64,
    animated: bool,
) -> Result<(), fmt::Error> {
    let animated_prefix = if animated { "a" } else { "" };
    let name = name.unwrap_or("");
    write!(writer, "<{}:{}:{}>", animated_prefix, name, id)
}

/// Formats a reaction content block
pub fn format_content(
    reaction: ReactionType,
    ctx: Context<'_>,
) -> Result<Content, anyhow::Error> {
    let mut content = String::from("");
    match reaction {
        ReactionType::Unicode { name } => {
            content.push_str(&name);
            if let Some(shortcodes) = ctx.emojis.to_shortcodes(&name) {
                for shortcode in shortcodes {
                    write!(content, " :{}:", shortcode)?;
                }
            }
            Ok(Content {
                inner: content,
                emojis_used: vec![name],
                ..Content::default()
            })
        }
        ReactionType::Custom { id, animated, name } => {
            write_emoji(&mut content, name.as_deref(), id.0, animated)?;
            if let Some(name) = name {
                write!(content, " :{}:", name)?;
                Ok(Content {
                    inner: content,
                    custom_emojis_used: vec![id.0],
                    custom_emoji_names_used: vec![name],
                    ..Content::default()
                })
            } else {
                write!(content, " :{}:", id)?;
                Ok(Content {
                    inner: content,
                    custom_emojis_used: vec![id.0],
                    ..Content::default()
                })
            }
        }
    }
}
